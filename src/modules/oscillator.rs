use std::f32::consts::PI;

use eframe::egui::{self, Ui};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    module::{Input, Module, ModuleDescription, Port, PortDescription},
    rack::rack::{ProcessContext, ShowContext},
};

#[derive(Clone, Copy, PartialEq, EnumIter)]
pub enum Wave {
    Sine,
    Square,
    Triangle,
    Saw,
}

impl Wave {
    pub fn as_str(&self) -> &str {
        match self {
            Wave::Sine => "Sine",
            Wave::Square => "Square",
            Wave::Triangle => "Triangle",
            Wave::Saw => "Saw",
        }
    }
}

pub struct FrequencyInput;

impl Port for FrequencyInput {
    type Type = f32;

    fn name() -> &'static str {
        "freq"
    }
}

impl Input for FrequencyInput {
    fn default() -> Self::Type {
        70.0
    }

    fn show(value: &mut Self::Type, ui: &mut Ui) {
        ui.add(
            egui::DragValue::new(value)
                .clamp_range(0.0..=f32::MAX)
                .speed(1.0)
                .suffix(" Hz"),
        );
    }
}

pub struct FrameOutput;

impl Port for FrameOutput {
    type Type = f32;

    fn name() -> &'static str {
        "sample"
    }
}

pub struct Oscillator {
    pub wave: Wave,
    index: f32,
    alternating: bool,
}

impl Default for Oscillator {
    fn default() -> Self {
        Self {
            wave: Wave::Sine,
            index: 0.0,
            alternating: true,
        }
    }
}

impl Module for Oscillator {
    fn describe() -> ModuleDescription<Self> {
        ModuleDescription::default()
            .name("📉 Oscillator")
            .port(PortDescription::<FrequencyInput>::input())
            .port(PortDescription::<FrameOutput>::output())
    }

    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {
        ui.horizontal(|ui| {
            egui::ComboBox::new(ctx.instance, "wave")
                .selected_text(format!("{:?}", self.wave.as_str()))
                .show_ui(ui, |ui| {
                    for wave in Wave::iter() {
                        ui.selectable_value(&mut self.wave, wave, wave.as_str());
                    }
                });

            ui.checkbox(&mut self.alternating, "alternating");
        });
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        let mut ampl = match self.wave {
            Wave::Sine => (self.index * 2.0 * PI).sin(),
            Wave::Square => self.index.round() * 2.0 - 1.0,
            Wave::Triangle => ((1.0 - self.index) * 4.0 - 2.0).abs() - 1.0,
            Wave::Saw => (self.index * 2.0) - 1.0,
        };

        if !self.alternating {
            ampl = (ampl + 1.0) / 2.0;
        }

        let len = 1.0 / ctx.sample_rate() as f32;
        self.index += len * ctx.get_input::<FrequencyInput>();
        self.index %= 1.0;

        ctx.set_output::<FrameOutput>(ampl)
    }
}
