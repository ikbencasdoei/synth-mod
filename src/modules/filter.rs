use biquad::{Biquad, DirectForm1, ToHertz};
use eframe::egui::{self, Ui};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    frame::Frame,
    module::{Input, Module, ModuleDescription, Port},
    rack::rack::{ProcessContext, ShowContext},
};

pub struct FilterInput;

impl Port for FilterInput {
    type Type = Frame;

    fn name() -> &'static str {
        "input"
    }
}

impl Input for FilterInput {
    fn default() -> Self::Type {
        Frame::ZERO
    }
}

pub struct FilterOutput;

impl Port for FilterOutput {
    type Type = Frame;

    fn name() -> &'static str {
        "output"
    }
}

#[derive(Clone, Copy, PartialEq, EnumIter)]
enum FilterType {
    LowPass,
    HighPass,
}

impl FilterType {
    pub fn as_str(&self) -> &str {
        match self {
            FilterType::LowPass => "lowpass",
            FilterType::HighPass => "highpass",
        }
    }
}

pub struct Filter {
    left: DirectForm1<f32>,
    right: DirectForm1<f32>,
    filter_type: FilterType,
    cutoff: f32,
}

impl Filter {
    pub fn new() -> Self {
        let filter_type = FilterType::LowPass;

        //TODO: make this work with all sample rates
        let sample_rate = 192000.hz();
        let cutoff = 50.0;

        let coeffs = match filter_type {
            FilterType::LowPass => biquad::Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                sample_rate,
                cutoff.hz(),
                biquad::Q_BUTTERWORTH_F32,
            )
            .unwrap(),
            FilterType::HighPass => biquad::Coefficients::<f32>::from_params(
                biquad::Type::HighPass,
                sample_rate,
                cutoff.hz(),
                biquad::Q_BUTTERWORTH_F32,
            )
            .unwrap(),
        };

        Self {
            left: DirectForm1::<f32>::new(coeffs),
            right: DirectForm1::<f32>::new(coeffs),
            filter_type,
            cutoff,
        }
    }

    fn update_coeffs(&mut self, sample_rate: u32) {
        let coeffs = match self.filter_type {
            FilterType::LowPass => biquad::Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                sample_rate.hz(),
                self.cutoff.max(1.0).hz(),
                biquad::Q_BUTTERWORTH_F32,
            ),
            FilterType::HighPass => biquad::Coefficients::<f32>::from_params(
                biquad::Type::HighPass,
                sample_rate.hz(),
                self.cutoff.max(1.0).hz(),
                biquad::Q_BUTTERWORTH_F32,
            ),
        };

        let Ok(coeffs) = coeffs else {
            return
        };

        self.left.update_coefficients(coeffs);
        self.right.update_coefficients(coeffs);
    }
}

impl Module for Filter {
    fn describe() -> ModuleDescription
    where
        Self: Sized,
    {
        ModuleDescription::new(|| Filter::new())
            .set_name("🕳 Filter")
            .add_input::<FilterInput>()
            .add_output::<FilterOutput>()
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        let mut frame = ctx.get_input::<FilterInput>();

        frame = match frame {
            Frame::Mono(frame) => Frame::Mono(self.left.run(frame)),
            Frame::Stereo(left, right) => Frame::Stereo(self.left.run(left), self.right.run(right)),
        };

        ctx.set_output::<FilterOutput>(frame);
    }

    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::DragValue::new(&mut self.cutoff)
                        .clamp_range(1.0..=f32::MAX)
                        .speed(1.0)
                        .suffix(" Hz"),
                )
                .changed()
            {
                self.update_coeffs(ctx.sample_rate)
            }

            egui::ComboBox::new(ctx.instance, "wave")
                .selected_text(format!("{:?}", self.filter_type.as_str()))
                .show_ui(ui, |ui| {
                    for filter in FilterType::iter() {
                        if ui
                            .selectable_value(&mut self.filter_type, filter, filter.as_str())
                            .changed()
                        {
                            self.update_coeffs(ctx.sample_rate)
                        }
                    }
                });
        });
    }
}
