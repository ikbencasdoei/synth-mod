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

pub struct Audio {
    pub volume: f32,
    current: Option<Frame>,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            volume: 1.0,
            current: Default::default(),
        }
    }
}

impl Audio {
    pub fn current_frame(&self) -> Option<Frame> {
        self.current
    }
}

impl Module for Audio {
    fn describe() -> ModuleDescription {
        ModuleDescription::new(Audio::default)
            .set_name("🔊 Audio Output")
            .add_input_description(
                PortDescription::new_input::<AudioInput>()
                    .add_conversion(|sample: f32| Frame::Mono(sample)),
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
        self.current = Some(ctx.get_input::<AudioInput>() * self.volume)
    }
}
