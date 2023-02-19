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
    gtk::init().expect("Error initializing gtk");
    let app = gtk::Application::new( Some(APP_ID), Default::default(), );
    app.connect_activate(|app| {
        //let win = Controller::new(app);
        load_css(&"style.css");     // needs app to be active before this can be done
        //let win = Board::new(app, 10, 20, 0);
        let  win = Controller::new(app);
        win.show();
    });
    app.run();
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
