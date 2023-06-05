use std::path::Path;

use eframe::egui::{Slider, Ui};
use rfd::FileDialog;
use rubato::{FftFixedIn, Resampler};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::DecoderOptions,
    errors::Error,
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::Hint,
};

use crate::{
    frame::Frame,
    module::{Module, ModuleDescription, Port, PortValueBoxed},
    rack::rack::{ProcessContext, ShowContext},
};

pub struct FileOutput;

impl Port for FileOutput {
    type Type = Frame;

    fn name() -> &'static str {
        "output"
    }
}

impl PortValueBoxed for Frame {
    fn to_string(&self) -> String {
        match self {
            Frame::Mono(sample) => format!("Mono({})", sample),
            Frame::Stereo(a, b) => {
                format!("Stereo({},{})", a, b)
            }
        }
    }

    fn as_value(&self) -> f32 {
        self.as_f32_mono()
    }
}

pub struct File {
    pub buffer: Vec<Frame>,
    pub seek: usize,
    pub playing: bool,
    path: String,
}

impl Default for File {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            seek: Default::default(),
            playing: Default::default(),
            path: String::new(),
        }
    }
}

impl File {
    pub fn decode(path: impl AsRef<Path>) -> Option<Vec<Frame>> {
        let file = std::fs::File::open(&path).ok()?;

        let source = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

        let mut hint = Hint::new();
        if let Some(extension) = path.as_ref().extension() {
            hint.with_extension(&extension.to_string_lossy());
        }

        let probe = symphonia::default::get_probe()
            .format(
                &hint,
                source,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .ok()?;

        let mut format = probe.format;

        let track = format
            .tracks()
            .iter()
            .find(|track| track.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)?;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .ok()?;

        let track_id = track.id;

        let mut buffer = Vec::<f32>::new();
        let mut spec = None;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::ResetRequired) => {
                    return None;
                }
                Err(err) => {
                    dbg!(err);
                    break;
                }
            };

            while !format.metadata().is_latest() {
                format.metadata().pop();
            }

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    spec = Some(*decoded.spec());
                    let duration = decoded.capacity() as u64;

                    let mut sample_buffer = SampleBuffer::new(duration, spec?);
                    sample_buffer.copy_interleaved_ref(decoded);
                    buffer.extend(sample_buffer.samples());
                }
                Err(Error::IoError(err)) => {
                    dbg!(err);
                    continue;
                }
                Err(Error::DecodeError(err)) => {
                    dbg!(err);
                    continue;
                }
                Err(err) => {
                    eprintln!("{}", err);
                    return None;
                }
            }
        }

        let channels = spec.unwrap().channels.count();
        let mut seperated: Vec<Vec<f32>> = (0..channels).into_iter().map(|_| Vec::new()).collect();

        for (i, sample) in buffer.into_iter().enumerate() {
            seperated[i % channels].push(sample)
        }

        let mut resampler = FftFixedIn::<f32>::new(
            spec.unwrap().rate as usize,
            192000,
            seperated.first()?.len(),
            1024,
            channels,
        )
        .unwrap();

        let resampled = resampler.process(&seperated, None).ok()?;

        let buffer: Vec<Frame> = match resampled.len() {
            1 => resampled[0]
                .iter()
                .map(|frame| Frame::Mono(*frame))
                .collect(),
            2 => resampled[0]
                .iter()
                .zip(resampled[1].iter())
                .map(|(a, b)| Frame::Stereo(*a, *b))
                .collect(),
            _ => return None,
        };

        Some(buffer)
    }

    pub fn try_decode(&mut self, path: &str) {
        self.path = path.to_string();
        self.update()
    }

    pub fn update(&mut self) {
        if let Some(buffer) = Self::decode(&self.path) {
            self.buffer = buffer;
        }
    }
}

impl Module for File {
    fn describe() -> crate::module::ModuleDescription
    where
        Self: Sized,
    {
        ModuleDescription::new(File::default)
            .set_name("📁 File")
            .add_output::<FileOutput>()
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        let frame = if self.playing {
            if self.seek < self.buffer.len() {
                self.seek += 1;
                self.buffer.get(self.seek - 1).copied().unwrap()
            } else {
                self.playing = false;
                self.seek = 0;
                Frame::default()
            }
        } else {
            Frame::default()
        };

        ctx.set_output::<FileOutput>(frame);
    }

    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.playing, true, "▶");
            ui.selectable_value(&mut self.playing, false, "⏸");

            ui.text_edit_singleline(&mut self.path);
            if ui.button("load").clicked() {
                self.update();
            }
            if ui.button("pick").clicked() {
                let mut dialog = FileDialog::new().add_filter("audio", &["mp3"]);

                if !self.path.is_empty() {
                    dialog = dialog.set_directory(&self.path);
                }

                if let Some(path) = dialog.pick_file() {
                    self.try_decode(&path.to_string_lossy())
                }
            }
        });

        ui.horizontal(|ui| {
            let secs = self.seek as f32 / ctx.sample_rate as f32;
            ui.label(format!(
                "{:02}:{:02}:{:02}",
                (secs as u32 / 60) % 60,
                secs as u32 % 60,
                (secs * 100.0 % 100.0).floor()
            ));

            ui.scope(|ui| {
                ui.style_mut().spacing.slider_width = ui.available_width();

                let mut seek = self.seek;

                let response =
                    ui.add(Slider::new(&mut seek, 0..=self.buffer.len().max(1)).show_value(false));

                if response.drag_released() {
                    self.seek = seek;
                }
            });
        });
    }
}
