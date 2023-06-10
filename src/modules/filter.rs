use biquad::{Biquad, DirectForm1, ToHertz};
use eframe::egui::{self, Ui};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    frame::Frame,
    module::{Input, Module, ModuleDescription, Port, PortDescription},
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
    left: Option<DirectForm1<f32>>,
    right: Option<DirectForm1<f32>>,
    filter_type: FilterType,
    cutoff: f32,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            left: None,
            right: None,
            filter_type: FilterType::LowPass,
            cutoff: 50.0,
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

        if let Some(left) = &mut self.left {
            left.update_coefficients(coeffs);
        } else {
            self.left = Some(DirectForm1::<f32>::new(coeffs));
        }

        if let Some(right) = &mut self.right {
            right.update_coefficients(coeffs);
        } else {
            self.right = Some(DirectForm1::<f32>::new(coeffs));
        }
    }
}

impl Module for Filter {
    fn describe() -> ModuleDescription<Self>
    where
        Self: Sized,
    {
        ModuleDescription::new(|| Filter::new())
            .name("🕳 Filter")
            .port(PortDescription::<FilterInput>::input())
            .port(PortDescription::<FilterOutput>::output())
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        let mut frame = ctx.get_input::<FilterInput>();

        if self.left.is_none() {
            self.update_coeffs(ctx.sample_rate())
        }

        frame = match frame {
            Frame::Mono(frame) => Frame::Mono(self.left.as_mut().unwrap().run(frame)),
            Frame::Stereo(left, right) => Frame::Stereo(
                self.left.as_mut().unwrap().run(left),
                self.right.as_mut().unwrap().run(right),
            ),
        };

        ctx.set_output::<FilterOutput>(frame);
    }

    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::DragValue::new(&mut self.cutoff)
                        .clamp_range(10.0..=f32::MAX)
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
