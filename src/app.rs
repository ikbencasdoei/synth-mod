use std::time::Duration;

use eframe::egui::{self, Context};
#[cfg(not(target_arch = "wasm32"))]
use eframe::epaint::Vec2;
use wasm_timer::Instant;

use crate::{frame::Frame, output::Output, rack::rack::Rack};

const SCALE: f32 = 1.5;
const PROFILING: bool = false;

pub struct App {
    pub rack: Rack,
    output: Output,
    last_instant: Instant,
}

impl Default for App {
    fn default() -> Self {
        #[cfg(target_arch = "wasm32")]
        console_error_panic_hook::set_once();
        Self {
            rack: Rack::default(),
            output: Output::new(),
            last_instant: Instant::now(),
        }
    }
}

impl App {
    #[cfg(target_arch = "wasm32")]
    pub fn run(self) {
        puffin::set_scopes_on(PROFILING);

        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .set_title(env!("CARGO_PKG_NAME"));

        let options = eframe::WebOptions::default();

        wasm_bindgen_futures::spawn_local(async {
            eframe::WebRunner::new()
                .start(
                    "canvas",
                    options,
                    Box::new(|cc| {
                        cc.egui_ctx.set_pixels_per_point(SCALE);
                        // cc.egui_ctx.set_debug_on_hover(true);
                        Box::new(self)
                    }),
                )
                .await
                .unwrap();
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(self) {
        puffin::set_scopes_on(PROFILING);

        let options = eframe::NativeOptions {
            initial_window_size: Some(Vec2::new(1280.0, 720.0)),
            centered: true,
            // maximized: true,
            follow_system_theme: false,
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

    fn show(&mut self, ctx: &Context) {
        puffin::profile_function!();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(env!("CARGO_PKG_NAME"));
                ui.separator();

                self.output.show(ui);
                ui.separator();
            });
        });

        self.rack.show(ctx, self.output.sample_rate_or_default());
    }

    fn process(&mut self, delta: Duration) {
        puffin::profile_function!();

        if let Some(instance) = self.output.instance_mut() {
            let amount = instance.free_len();

            let outputs = self.rack.process_amount(instance.sample_rate(), amount);

            for i in 0..amount {
                let mut mixed = Frame::ZERO;
                if let Some(frames) = outputs.get(i) {
                    for &frame in frames {
                        mixed += frame;
                    }
                }

                instance
                    .push_frame(mixed)
                    .expect("producer should not be full");
            }
        } else {
            let samples =
                (self.output.sample_rate_or_default() as f32 * delta.as_secs_f32()) as usize;
            self.rack
                .process_amount(self.output.sample_rate_or_default(), samples);
        }
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

        self.show(ctx);

        self.process(delta);

        ctx.request_repaint();
    }
}
