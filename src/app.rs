use std::time::Instant;

use eframe::{
    egui::{self, Context},
    epaint::Vec2,
};

use crate::{frame::Frame, output::Output, rack::rack::Rack};

const SCALE: f32 = 1.5;
const PROFILING: bool = true;

pub struct App {
    pub rack: Rack,
    output: Output,
    last_instant: Instant,
    last_sample_rate: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            rack: Rack::default(),
            output: Output::new(),
            last_instant: Instant::now(),
            last_sample_rate: 44100,
        }
    }
}

impl App {
    pub fn run(self) {
        puffin::set_scopes_on(PROFILING);

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
        puffin::profile_function!();
        puffin::GlobalProfiler::lock().new_frame();

        if PROFILING {
            puffin_egui::profiler_window(ctx);
        }

        puffin::profile_scope!("app");

        let delta = self.last_instant.elapsed();
        self.last_instant = Instant::now();

        {
            puffin::profile_scope!("show");

            egui::TopBottomPanel::top("top").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(env!("CARGO_PKG_NAME"));
                    ui.separator();

                    self.output.show(ui);
                    ui.separator();
                });
            });

            self.rack.show(
                ctx,
                self.output.sample_rate().unwrap_or(self.last_sample_rate),
            );
        }

        {
            puffin::profile_scope!("process");

            if self.output.has_valid_instance() {
                while !self.output.is_full() {
                    let outputs = self.rack.process(
                        self.output
                            .sample_rate()
                            .expect("if output has an instance this should be present"),
                    );

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

                self.last_sample_rate = self.output.sample_rate().unwrap()
            } else {
                let samples = (self.last_sample_rate as f32 * delta.as_secs_f32()) as usize;
                for _ in 0..samples {
                    self.rack.process(self.last_sample_rate);
                }
            }
        }

        ctx.request_repaint();
    }
}
