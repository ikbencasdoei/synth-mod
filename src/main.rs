#![feature(iter_array_chunks)]
#![feature(trait_upcasting)]

mod app;
mod damper;
mod frame;
mod instance;
mod io;
mod module;
mod modules;
mod output;
mod rack;
mod types;

use app::App;

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    App::default().run()
}
