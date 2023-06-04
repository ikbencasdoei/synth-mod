use eframe::egui::{self, Ui};

use crate::{
    frame::Frame,
    module::{Input, Module, ModuleDescription, Port, PortValueBoxed},
    rack::rack::{ProcessContext, ShowContext},
};

impl PortValueBoxed for f32 {
    fn to_string(&self) -> String {
        format!("{:.2}", self)
    }
    fn as_value(&self) -> f32 {
        *self
    }
}

pub struct FrameInput;

impl Port for FrameInput {
    type Type = f32;

    fn name() -> &'static str {
        "output"
    }
}

impl Input for FrameInput {
    fn default() -> Self::Type {
        0.0
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
            .add_input::<FrameInput>()
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
        self.current = Some(Frame::Mono(ctx.get_input::<FrameInput>() * self.volume))
    }
}
