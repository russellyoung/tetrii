#![allow(unused)]

mod board;
mod config;
mod options;
mod controller;

use config::Config;
use options::Options;
use board::Board;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::gdk::Display;

use gtk::prelude::*;
use std::fs;

const APP_ID: &str = "com.young-0.tetrii.rust";

fn main() {
    let config = Config::build_config();
    gtk::init().expect("Error initializing gtk");
    let app = gtk::Application::new( Some(APP_ID), Default::default(), );
    let height = config.height;
    let width = config.width;
    let preview = config.preview;
    app.connect_activate(move |appx| {
        load_css(&"style.css");     // needs app to be active before this can be done
        //let win = Board::new(app, 10, 20, 0);
        let  win = Options::new(appx);
        win.show();
        win.set_defaults(config.boards, width, height, preview);
    });
    let empty: Vec<String> = vec![];  // thanks to stackoverflow, I learned EMPTY is needed to keep GTK from interpreting the command line flags
    app.run_with_args(&empty);
}

fn load_css(filename: &str) {
    let provider = CssProvider::new();
    let css_data = fs::read(filename).expect("could not find CSS file");
    provider.load_from_data(&css_data);
    StyleContext::add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
pub const CMD_LEFT: u32             = 0x1;
pub const CMD_RIGHT: u32            = 0x2;
pub const CMD_DOWN: u32             = 0x4;
pub const CMD_CLOCKWISE: u32        = 0x8;
pub const CMD_COUNTERCLOCKWISE: u32 = 0x10;
pub const CMD_CHEAT: u32            = 0x80000000;
pub const CMD_CHEAT_END: u32        = 0x80000100;

