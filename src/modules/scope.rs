use eframe::{
    egui::{
        self,
        plot::{Legend, Line, Plot},
        Ui,
    },
    epaint::Color32,
};

use crate::{
    module::{Input, Module, ModuleDescription, Port},
    rack::rack::{ProcessContext, ShowContext},
};

pub struct ScopeInput;

impl Port for ScopeInput {
    type Type = f32;

    fn name() -> &'static str {
        "input"
    }
}

impl Input for ScopeInput {
    fn default() -> Self::Type {
        0.0
    }
}

enum State {
    Updating { pos: usize },
    Waiting { waited: usize },
}

pub struct Scope {
    buffer: Vec<f32>,
    size: usize,
    interval: usize,
    state: State,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            size: 10000,
            interval: 50000,
            state: State::Updating { pos: 0 },
        }
    }
}

impl Scope {
    pub fn points(&self) -> Vec<[f64; 2]> {
        self.buffer
            .iter()
            .step_by((self.size / 10000).max(1))
            .enumerate()
            .map(|(i, frame)| [i as f64, *frame as f64])
            .collect()
    }
}

impl Module for Scope {
    fn describe() -> ModuleDescription {
        ModuleDescription::new(|| Scope::default())
            .set_name("📈 Scope")
            .add_input::<ScopeInput>()
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        let frame = ctx.get_input::<ScopeInput>();

        match self.state {
            State::Updating { pos } => {
                if pos >= self.size {
                    self.state = State::Waiting { waited: 0 };
                    if self.buffer.len() > self.size {
                        self.buffer.resize(self.size, 0.0)
                    }
                } else {
                    if self.buffer.len() > pos {
                        *self.buffer.get_mut(pos).unwrap() = frame;
                    } else {
                        self.buffer.push(frame);
                    }
                    self.state = State::Updating { pos: pos + 1 };
                }
            }
            State::Waiting { waited } => {
                if self.interval > waited {
                    self.state = State::Waiting { waited: waited + 1 }
                } else {
                    self.state = State::Updating { pos: 0 }
                }
            }
        }
    }

    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("duration");
            {
                let mut seconds = self.size / (ctx.sample_rate as usize / 1000);
                if ui
                    .add(
                        egui::DragValue::new(&mut seconds)
                            .suffix(" ms")
                            .speed(5)
                            .clamp_range(1..=usize::MAX),
                    )
                    .changed()
                {
                    self.size = seconds * (ctx.sample_rate as usize / 1000)
                }
            }

            ui.label("interval");
            {
                let mut interval = self.interval / (ctx.sample_rate as usize / 1000);
                if ui
                    .add(egui::DragValue::new(&mut interval).suffix(" ms").speed(10))
                    .changed()
                {
                    self.interval = interval * (ctx.sample_rate as usize / 1000)
                }
            }
        });

        Plot::new(ctx.instance)
            .legend(Legend::default())
            .height(100.0)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .show(ui, |ui| {
                ui.line({
                    Line::new(self.points())
                        .color(Color32::LIGHT_GREEN)
                        .name("plot")
                })
            });
    }
}
