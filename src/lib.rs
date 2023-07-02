#![feature(iter_array_chunks)]
#![feature(trait_upcasting)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
mod damper;
mod frame;
mod instance;
pub mod io;
pub mod module;
pub mod modules;
mod output;
mod rack;
mod types;
