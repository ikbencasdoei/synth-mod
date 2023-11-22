use eframe::egui::Ui;
use rand::Rng;

use crate::{
    module::{Module, ModuleDescription, Port, PortDescription},
    rack::rack::{ProcessContext, ShowContext},
};

pub struct NoiseOutput;

impl Port for NoiseOutput {
    type Type = f32;

    fn name() -> &'static str {
        "output"
    }
}

#[derive(Default)]
pub struct Noise {}

impl Module for Noise {
    fn describe() -> ModuleDescription<Self>
    where
        Self: Sized,
    {
        ModuleDescription::default()
            .name("⎙ Noise")
            .port(PortDescription::<NoiseOutput>::output())
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        ctx.set_output::<NoiseOutput>(rand::thread_rng().gen_range(-1.0..=1.0))
    }

    fn show(&mut self, _: &ShowContext, _: &mut Ui) {}
}
