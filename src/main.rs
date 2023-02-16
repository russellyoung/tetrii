#![allow(unused)]
// TODO: implement cell_size
mod board;
mod config;
mod controller;

use config::Config;
use board::Board;
use crate::controller::Controller;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "com.young-0.tetrii.rust";

fn main() {
    gtk::init().expect("Error initializing gtk");
    let app = gtk::Application::new( Some(APP_ID), Default::default(), );
    app.connect_activate(move |app| Controller::new(&app).build_ui());

    let empty: Vec<String> = vec![];  // thanks to stackoverflow, I learned EMPTY is needed to keep GTK from interpreting the command line flags
    app.run_with_args(&empty);
}
