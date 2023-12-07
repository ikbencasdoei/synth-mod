use std::{collections::VecDeque, time::Duration};

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
    last_deltas: VecDeque<Duration>,
}

impl Default for App {
    fn default() -> Self {
        #[cfg(target_arch = "wasm32")]
        console_error_panic_hook::set_once();
        Self {
            rack: Rack::default(),
            output: Output::new(),
            last_instant: Instant::now(),
            last_deltas: VecDeque::new(),
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
            viewport: egui::ViewportBuilder::default().with_inner_size(Vec2::new(1280.0, 720.0)),
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

    /// Draw ui
    fn show(&mut self, ctx: &Context, avg_delta: Duration) {
        puffin::profile_function!();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(env!("CARGO_PKG_NAME"));
                ui.separator();

                self.output.show(ui);
                ui.separator();

                ui.label(format!("{:.1}ms", avg_delta.as_secs_f32() * 1000.0))
                    .on_hover_text_at_pointer("average frame time");
                ui.separator();
            });
        });

        self.rack.show(ctx, self.output.sample_rate_or_default());
    }

    /// Process modules & audio output
    fn process(&mut self, delta: Duration) {
        puffin::profile_function!();

        if let Some(instance) = self.output.instance_mut() {
            let outputs = self
                .rack
                .process_amount(instance.sample_rate(), instance.free_len())
                .into_iter()
                .map(|frames| {
                    let mut mixed = Frame::ZERO;

                    for frame in frames {
                        mixed += frame;
                    }

                    mixed
                })
                .collect::<Vec<_>>();

            instance.push_iter(outputs.into_iter());
        } else {
            let samples =
                (self.output.sample_rate_or_default() as f32 * delta.as_secs_f32()) as usize;
            self.rack
                .process_amount(self.output.sample_rate_or_default(), samples);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        puffin::profile_function!();
        puffin::GlobalProfiler::lock().new_frame();

        if PROFILING {
            puffin_egui::profiler_window(ctx);
        }

        puffin::profile_scope!("app");

        let delta = self.last_instant.elapsed();

        self.last_deltas.push_front(delta);
        if self.last_deltas.len() > 50 {
            self.last_deltas.pop_back();
        }

        let avg_delta = self.last_deltas.iter().sum::<Duration>() / self.last_deltas.len() as u32;

        self.last_instant = Instant::now();

        self.show(ctx, avg_delta);

        self.process(delta);

        if ctx.input(|input| input.key_pressed(egui::Key::F2)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot)
        }

        ctx.input(|input| {
            for event in input.raw.events.iter() {
                if let egui::Event::Screenshot {
                    viewport_id: _,
                    image,
                } = event
                {
                    image::save_buffer(
                        "screenshot.png",
                        image.as_raw(),
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
            }
        });

        ctx.request_repaint();
    }
}
