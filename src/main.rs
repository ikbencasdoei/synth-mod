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

use app::App;

fn main() {
    App::default().run()
}
