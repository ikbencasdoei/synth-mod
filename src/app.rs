use eframe::{
    egui::{self, Context},
    epaint::Vec2,
};

use crate::{frame::Frame, output::Output, rack::rack::Rack};

const SCALE: f32 = 1.5;

pub struct App {
    pub rack: Rack,
    output: Option<Output>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            rack: Rack::default(),
            output: Output::new(),
        }
    }
}

impl App {
    pub fn run(self) {
        let options = eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(1280.0, 720.0)),
            centered: true,
            // maximized: true,
            ..Default::default()
        };

        eframe::run_native(
            env!("CARGO_PKG_NAME"),
            options,
            Box::new(|cc| {
                cc.egui_ctx.set_pixels_per_point(SCALE);
                // cc.egui_ctx.set_debug_on_hover(true);
                Box::new(self)
            }),
        )
        .unwrap();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(env!("CARGO_PKG_NAME"));
                ui.separator();

                if let Some(output) = &mut self.output {
                    output.show(ui);
                    if !output.is_valid() {
                        self.output = None;
                    }
                } else {
                    ui.label("⚠ could not initialize audio output!");
                    if ui.button("retry").clicked() {
                        self.output = Output::new();
                    }
                }
                ui.separator();
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.rack.show(
                ui,
                self.output
                    .as_ref()
                    .map(|output| output.config.sample_rate.0)
                    .unwrap_or_default(),
            );
        });

        if let Some(output) = &mut self.output {
            while !output.is_full() {
                let outputs = self.rack.process(output.config.sample_rate.0);

                if outputs.len() > 0 {
                    for frame in outputs {
                        output.push_frame(frame)
                    }
                } else {
                    output.push_frame(Frame::ZERO)
                }

                output
                    .commit_frames()
                    .expect("ringbuffer should not overflow using output.is_full");
            }
        }

        ctx.request_repaint();
    }
}
