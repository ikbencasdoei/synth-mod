#![cfg(not(target_arch = "wasm32"))]

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender},
};

use eframe::egui::{Slider, Ui};
use rfd::FileDialog;
use rubato::{FftFixedIn, Resampler};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::DecoderOptions,
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::Hint,
};

use crate::{
    frame::Frame,
    module::{Module, ModuleDescription, Port, PortDescription},
    rack::rack::{ProcessContext, ShowContext},
};

pub struct FileOutput;

impl Port for FileOutput {
    type Type = Frame;

    fn name() -> &'static str {
        "output"
    }
}

enum Message {
    Decoded(Option<Vec<Frame>>),
    PickedFile(PathBuf),
}

/// A [`Module`] that decodes and plays files
pub struct File {
    pub buffer: Vec<Frame>,
    pub seek: usize,
    pub playing: bool,
    path: String,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    loading: bool,
}

impl Default for File {
    fn default() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            buffer: Vec::new(),
            seek: 0,
            playing: false,
            path: String::new(),
            sender,
            receiver,
            loading: false,
        }
    }
}

impl File {
    pub fn decode(path: impl AsRef<Path>, target_sample_rate: usize) -> Option<Vec<Frame>> {
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
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    return None;
                }
                Err(symphonia::core::errors::Error::IoError(err)) => {
                    if err.kind() != ErrorKind::UnexpectedEof {
                        dbg!(err);
                    }
                    break;
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
                Err(symphonia::core::errors::Error::IoError(err)) => {
                    dbg!(err);
                    continue;
                }
                Err(symphonia::core::errors::Error::DecodeError(err)) => {
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
        let mut separated: Vec<Vec<f32>> = (0..channels).into_iter().map(|_| Vec::new()).collect();

        for (i, sample) in buffer.into_iter().enumerate() {
            separated[i % channels].push(sample)
        }

        let mut resampler = FftFixedIn::<f32>::new(
            spec.unwrap().rate as usize,
            target_sample_rate,
            separated.first()?.len(),
            1024,
            channels,
        )
        .unwrap();

        let resampled = resampler.process(&separated, None).ok()?;

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

    #[allow(dead_code)]
    pub fn open_file(&self, path: impl AsRef<Path>) {
        self.sender
            .send(Message::PickedFile(path.as_ref().into()))
            .ok();
    }

    fn update(&mut self, sample_rate: usize) {
        self.loading = true;
        std::thread::spawn({
            let sender = self.sender.clone();
            let path = self.path.clone();
            move || {
                sender
                    .send(Message::Decoded(Self::decode(&path, sample_rate)))
                    .ok();
            }
        });
    }

    fn open_picker(&self) {
        let mut dialog = FileDialog::new().add_filter("audio", &["mp3"]);

        if !self.path.is_empty() {
            dialog = dialog.set_directory(&self.path);
        }

        std::thread::spawn({
            let sender = self.sender.clone();
            move || {
                if let Some(path) = dialog.pick_file() {
                    sender.send(Message::PickedFile(path)).ok();
                }
            }
        });
    }
}

impl Module for File {
    fn describe() -> crate::module::ModuleDescription<Self>
    where
        Self: Sized,
    {
        ModuleDescription::new(File::default)
            .name("📁 File")
            .port(PortDescription::<FileOutput>::output())
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
        let messages = self.receiver.try_iter().collect::<Vec<_>>();
        for message in messages {
            match message {
                Message::Decoded(buffer) => {
                    if let Some(buffer) = buffer {
                        self.buffer = buffer;
                    }
                    self.loading = false
                }
                Message::PickedFile(path) => {
                    self.path = path.to_string_lossy().to_string();
                    self.update(ctx.sample_rate as usize);
                }
            }
        }

        ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.buffer.is_empty(), |ui| {
                ui.selectable_value(&mut self.playing, true, "▶");
                ui.selectable_value(&mut self.playing, false, "⏸");
            });

            if ui.text_edit_singleline(&mut self.path).changed() {
                self.update(ctx.sample_rate as usize);
            }

            if ui.button("pick").clicked() {
                self.open_picker()
            }

            if self.loading {
                ui.spinner();
            }
        });

        ui.horizontal(|ui| {
            let progress = self.seek as f32 / ctx.sample_rate as f32;
            let total = self.buffer.len() as f32 / ctx.sample_rate as f32;
            ui.label(format!(
                "{:02}:{:02}.{:02}/{:02}:{:02}.{:02}",
                (progress as u32 / 60) % 60,
                progress as u32 % 60,
                (progress * 100.0 % 100.0).floor(),
                (total as u32 / 60) % 60,
                total as u32 % 60,
                (total * 100.0 % 100.0).floor()
            ));

            ui.scope(|ui| {
                ui.style_mut().spacing.slider_width = ui.available_width();

                let mut seek = self.seek;

                let response = ui.add_enabled(
                    !self.buffer.is_empty(),
                    Slider::new(&mut seek, 0..=self.buffer.len().max(1)).show_value(false),
                );

                if response.drag_released() {
                    self.seek = seek;
                }
            });
        });

        if !self.buffer.is_empty() {
            ui.horizontal(|ui| {
                let size = std::mem::size_of_val(self.buffer.as_slice());

                let text = match size.ilog10() {
                    0..=2 => format!("{} bytes", size),
                    3..=5 => format!("{:.1} kB", size as f32 / 10f32.powi(3)),
                    6..=8 => format!("{:.1} MB", size as f32 / 10f32.powi(6)),
                    9..=u32::MAX => format!("{:.1} GB", size as f32 / 10f32.powi(9)),
                };

                ui.label(format!("{text}, todo: fix this"));
            });
        }
    }
}
