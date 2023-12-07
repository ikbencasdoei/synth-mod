use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream, StreamConfig,
};
use eframe::{
    egui::{self, RichText, Ui},
    epaint::Color32,
};
use ringbuf::{HeapRb, Producer};

use crate::{damper::LinearDamper, frame::Frame};

type RingProducer = Producer<Frame, Arc<HeapRb<Frame>>>;

/// Instance of the application's audio output.
pub struct StreamInstance {
    _stream: Stream,
    pub config: StreamConfig,
    producer: RingProducer,
    is_err: Arc<AtomicBool>,
    damper: LinearDamper<f32>,
    volume: f32,
    muted: bool,
    protection: bool,
}

impl StreamInstance {
    fn ringbuf_size(config: &StreamConfig, duration: Duration) -> usize {
        (config.sample_rate.0 as f32 * duration.as_secs_f32()) as usize
    }

    fn new(device: Device, config: StreamConfig) -> Option<Self> {
        let (producer, mut consumer) = {
            let duration = Duration::from_secs_f32(0.15);
            let rb = HeapRb::<Frame>::new(Self::ringbuf_size(&config, duration));
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
            damper: LinearDamper::new_cutoff(config.sample_rate.0),
            config,
            producer,
            is_err,
            volume: 0.5,
            muted: false,
            protection: false,
        })
    }

    pub fn is_valid(&self) -> bool {
        !self.is_err.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn free_len(&self) -> usize {
        self.producer.free_len()
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    pub fn push_iter(&mut self, iter: impl Iterator<Item = Frame>) {
        let mut map = iter.map(|frame| {
            let ampl = if self.muted || self.protection {
                self.damper.frame(0.0)
            } else {
                self.damper.frame(self.volume)
            };
            frame * ampl
        });
        self.producer.push_iter(&mut map);
    }

    fn show(&mut self, ui: &mut Ui) {
        let icon = if self.muted { "🔇" } else { "🔊" };
        if ui
            .add(egui::Label::new(icon).sense(egui::Sense::click()))
            .clicked()
        {
            self.muted = !self.muted;
        }

        ui.add(
            egui::DragValue::new(&mut self.volume)
                .speed(0.01)
                .clamp_range(0.0..=1.0),
        )
        .on_hover_text_at_pointer("volume");
        ui.separator();
        ui.label(RichText::new(format!("{}", self.sample_rate())).monospace())
            .on_hover_text_at_pointer("sample rate");
        ui.separator();

        ui.label(RichText::new(format!("{}", self.channels())).monospace())
            .on_hover_text_at_pointer("channels");

        if self.producer.free_len() > self.damper.cutoff_samples() as usize {
            self.protection = true;
            ui.separator();
            ui.label(RichText::new("⚠ cant keep up!").color(Color32::GOLD));
        } else {
            self.protection = false;
        }
    }
}

/// Manages the application's audio output.
pub struct Output {
    pub instance: Option<StreamInstance>,
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
        let mut new = Self { instance: None };

        new.init_instance();

        new
    }

    fn init_instance(&mut self) -> Option<&mut StreamInstance> {
        let device = Self::fetch_device()?;
        let config = Self::fetch_stream_config(&device)?;

        self.instance = StreamInstance::new(device, config);

        self.instance.as_mut()
    }

    pub fn check_instance(&mut self) {
        if self
            .instance
            .as_ref()
            .is_some_and(|instance| !instance.is_valid())
        {
            self.instance = None
        }
    }

    pub fn instance_mut(&mut self) -> Option<&mut StreamInstance> {
        self.check_instance();
        self.instance.as_mut()
    }

    pub fn instance_mut_or_init(&mut self) -> Option<&mut StreamInstance> {
        if !self.instance_mut().is_some() {
            self.init_instance();
        }

        self.instance_mut()
    }

    pub fn sample_rate_or_default(&self) -> u32 {
        self.instance
            .as_ref()
            .map(|instance| instance.sample_rate())
            .unwrap_or(44100)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if let Some(instance) = &mut self.instance_mut_or_init() {
            instance.show(ui)
        } else {
            ui.label(RichText::new("⚠ could not initialize audio output!").color(Color32::GOLD));
            if ui.button("retry").clicked() {
                self.init_instance();
            }
            ui.separator();
            ui.label(RichText::new(format!("({})", self.sample_rate_or_default())).monospace())
                .on_hover_text_at_pointer("fallback sample rate");
        }
    }
}
