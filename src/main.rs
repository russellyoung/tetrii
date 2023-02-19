#![allow(unused)]

mod board;
mod config;
mod controller;

use config::Config;
use controller::Controller;
use board::Board;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::gdk::Display;

use gtk::prelude::*;
use std::fs;

const APP_ID: &str = "com.young-0.tetrii.rust";

fn main() {
    let config = Config::build_config();
    println!("{:#?}", config);
    gtk::init().expect("Error initializing gtk");
    let app = gtk::Application::new( Some(APP_ID), Default::default(), );
    let height = config.height;
    let width = config.width;
    let preview = config.preview;
    app.connect_activate(move |appx| {
        //let win = Controller::new(app);
        load_css(&"style.css");     // needs app to be active before this can be done
        //let win = Board::new(app, 10, 20, 0);
        let  win = Controller::new(appx);
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
