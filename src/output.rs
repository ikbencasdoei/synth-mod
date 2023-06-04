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

pub struct StreamInstance {
    _stream: Stream,
    pub config: StreamConfig,
    producer: RingProducer,
    is_err: Arc<AtomicBool>,
    current_frames: Vec<Frame>,
}

impl StreamInstance {
    fn ringbuf_size(config: &StreamConfig, duration: Duration) -> usize {
        (config.sample_rate.0 as f32 * duration.as_secs_f32()) as usize
    }

    fn new(device: Device, config: StreamConfig) -> Option<Self> {
        let (producer, mut consumer) = {
            let duration = Duration::from_secs_f32(0.15);
            let rb: _ = HeapRb::<Frame>::new(Self::ringbuf_size(&config, duration));
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

        Some(Self {
            _stream: stream,
            config,
            producer,
            is_err,
            current_frames: Vec::new(),
        })
    }

    pub fn is_invalid(&self) -> bool {
        self.is_err.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_full(&self) -> bool {
        self.producer.is_full()
    }

    pub fn push_frame(&mut self, value: Frame, volume: f32) {
        self.current_frames.push(value * volume)
    }

    pub fn commit_frames(&mut self) -> Result<(), ()> {
        let mut new = Frame::ZERO;

        for frame in self.current_frames.drain(0..self.current_frames.len()) {
            new += frame;
        }

        self.producer.push(new).map_err(|_| ())
    }
}

pub struct Output {
    pub instance: Option<StreamInstance>,
    volume: f32,
    paused: bool,
    damper: LinearDamper<f32>,
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

    pub fn new() -> Self {
        let mut new = Self {
            instance: None,
            damper: LinearDamper::new(0.0001, 0.0),
            volume: 0.5,
            paused: true,
        };

        new.init_instance();

        new
    }

    pub fn is_full(&self) -> bool {
        if let Some(instance) = &self.instance {
            instance.is_full()
        } else {
            true
        }
    }

    pub fn push_frame(&mut self, value: Frame) {
        if let Some(instance) = &mut self.instance {
            let ampl = if self.paused {
                self.damper.frame(0.0)
            } else {
                self.damper.frame(self.volume)
            };

            instance.push_frame(value, ampl)
        }
    }

    fn init_instance(&mut self) {
        let Some(device) = Self::fetch_device() else {
            return
        };
        let Some(config) = Self::fetch_stream_config(&device) else {
            return
        };
        self.instance = StreamInstance::new(device, config);
    }

    pub fn sample_rate(&self) -> Option<u32> {
        let instance = self.instance.as_ref()?;
        Some(instance.config.sample_rate.0)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if self
            .instance
            .as_ref()
            .is_some_and(|instance| instance.is_invalid())
        {
            self.instance = None;
        }

        if let Some(instance) = &mut self.instance {
            ui.selectable_value(&mut self.paused, false, "▶");
            ui.selectable_value(&mut self.paused, true, "⏸");
            ui.add(
                egui::DragValue::new(&mut self.volume)
                    .speed(0.01)
                    .clamp_range(0.0..=1.0),
            )
            .on_hover_text_at_pointer("volume");
            ui.separator();
            ui.label(RichText::new(format!("{}", instance.config.sample_rate.0)).monospace())
                .on_hover_text_at_pointer("sample rate");
            ui.separator();

            ui.label(RichText::new(format!("{}", instance.config.channels)).monospace())
                .on_hover_text_at_pointer("channels");
        } else {
            ui.label("⚠ could not initialize audio output!");
            if ui.button("retry").clicked() {
                self.init_instance();
            }
        }
    }
}
