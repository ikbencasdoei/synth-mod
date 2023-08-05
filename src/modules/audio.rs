use std::sync::mpsc::Sender;

use eframe::egui::{self, Ui};

use crate::{
    frame::Frame,
    module::{Input, Module, ModuleDescription, Port, PortDescription},
    rack::rack::{ProcessContext, ShowContext},
};

pub struct AudioInput;

impl Port for AudioInput {
    type Type = Frame;

    fn name() -> &'static str {
        "output"
    }
}

impl Input for AudioInput {
    fn default() -> Self::Type {
        Frame::ZERO
    }
}

/// The audio output module
pub struct Audio {
    pub volume: f32,
    pub sender: Option<Sender<Frame>>,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            volume: 1.0,
            sender: None,
        }
    }
}

impl Module for Audio {
    fn describe() -> ModuleDescription<Self> {
        ModuleDescription::default().name("🔊 Audio Output").port(
            PortDescription::<AudioInput>::input().conversion(|sample: f32| Frame::Mono(sample)),
        )
    }

    fn show(&mut self, _: &ShowContext, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("volume:");
            ui.add(
                egui::DragValue::new(&mut self.volume)
                    .clamp_range(0.0..=2.0)
                    .speed(0.01),
            );
        });
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        if let Some(sender) = self.sender.as_ref() {
            sender
                .send(ctx.get_input::<AudioInput>() * self.volume)
                .unwrap();
        }
    }
}
