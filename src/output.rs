use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream, StreamConfig,
};
use eframe::egui::{self, RichText, Ui};
use ringbuf::{HeapRb, Producer};

use crate::{damper::LinearDamper, frame::Frame};

type RingProducer = Producer<Frame, Arc<HeapRb<Frame>>>;

pub struct Output {
    _stream: Stream,
    pub config: StreamConfig,
    producer: RingProducer,
    current_frames: Vec<Frame>,
    volume: f32,
    paused: bool,
    damper: LinearDamper<f32>,
    is_err: Arc<AtomicBool>,
}

impl Output {
    fn fetch_device() -> Option<Device> {
        let host = cpal::default_host();
        host.default_output_device()
    }

    fn fetch_stream_config(device: &Device) -> Option<StreamConfig> {
        Some(
            device
                .supported_output_configs()
                .ok()?
                .next()?
                .with_max_sample_rate()
                .config(),
        )
    }

    fn ringbuf_size(config: &StreamConfig, duration: Duration) -> usize {
        (config.sample_rate.0 as f32 * duration.as_secs_f32()) as usize
    }

    fn create_stream(
        device: &Device,
        config: &StreamConfig,
    ) -> Option<(Stream, RingProducer, Arc<AtomicBool>)> {
        let (producer, mut consumer) = {
            let duration = Duration::from_secs_f32(0.15);
            let rb: _ = HeapRb::<Frame>::new(Self::ringbuf_size(config, duration));
            rb.split()
        };

        let is_err = Arc::new(AtomicBool::new(false));

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    for chunk in data.iter_mut().array_chunks::<2>() {
                        let (a, b) = consumer.pop().unwrap_or_default().as_f32_tuple();
                        *chunk[0] = a;
                        *chunk[1] = b;
                    }
                },
                {
                    let is_err = is_err.clone();
                    move |_| {
                        is_err.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                },
                None,
            )
            .ok()?;

        stream.play().ok()?;

        Some((stream, producer, is_err))
    }

    pub fn new() -> Option<Self> {
        let device = Self::fetch_device()?;
        let config = Self::fetch_stream_config(&device)?;
        let (stream, producer, is_err) = Self::create_stream(&device, &config)?;

        Some(Self {
            _stream: stream,
            damper: LinearDamper::new(0.0001, 0.0),
            config,
            producer,
            current_frames: Vec::new(),
            volume: 0.5,
            paused: true,
            is_err,
        })
    }

    pub fn is_full(&self) -> bool {
        self.producer.is_full()
    }

    pub fn push_frame(&mut self, value: Frame) {
        self.current_frames.push(value)
    }

    pub fn commit_frames(&mut self) -> Result<(), ()> {
        let mut new = Frame::ZERO;

        for frame in self.current_frames.drain(0..self.current_frames.len()) {
            new += frame;
        }

        let ampl = if self.paused {
            self.damper.frame(0.0)
        } else {
            self.damper.frame(self.volume)
        };

        self.producer.push(new * ampl).map_err(|_| ())
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.selectable_value(&mut self.paused, false, "▶");
        ui.selectable_value(&mut self.paused, true, "⏸");
        ui.add(
            egui::DragValue::new(&mut self.volume)
                .speed(0.01)
                .clamp_range(0.0..=1.0),
        )
        .on_hover_text_at_pointer("volume");
        ui.separator();
        ui.label(RichText::new(format!("{}", self.config.sample_rate.0)).monospace())
            .on_hover_text_at_pointer("sample rate");
        ui.separator();

        ui.label(RichText::new(format!("{}", self.config.channels)).monospace())
            .on_hover_text_at_pointer("channels");
    }

    pub fn is_valid(&self) -> bool {
        !self.is_err.load(std::sync::atomic::Ordering::Relaxed)
    }
}
