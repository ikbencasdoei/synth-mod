use eframe::{
    egui::{self, Context},
    epaint::Vec2,
};

use crate::{frame::Frame, output::Output, rack::rack::Rack};

const SCALE: f32 = 1.5;

pub struct App {
    pub rack: Rack,
    output: Output,
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

                self.output.show(ui);
                ui.separator();
            });
        });

        self.rack
            .show(ctx, self.output.sample_rate().unwrap_or_default());

        while !self.output.is_full() {
            let outputs = self
                .rack
                .process(self.output.sample_rate().unwrap_or_default());

            if !outputs.is_empty() {
                for frame in outputs {
                    self.output.push_frame(frame)
                }
            } else {
                self.output.push_frame(Frame::ZERO)
            }

            self.output
                .instance
                .as_mut()
                .unwrap()
                .commit_frames()
                .expect("ringbuffer should not overflow using output.is_full");
        }

        ctx.request_repaint();
    }
}
