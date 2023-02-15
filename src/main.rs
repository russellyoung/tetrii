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

fn main() {
    let config = Config::build_config();
    gtk::init().expect("Error initializing gtk");
    let controller_rc = Controller::new(config);
    let controller_rc_activate = Rc::clone(&controller_rc);
    let controller = controller_rc.borrow();
    controller.app.connect_activate(move |_app| controller_rc_activate.borrow().build_ui());
    let empty: Vec<String> = vec![];  // thanks to stackoverflow, I learned EMPTY is needed to keep GTK from interpreting the command line flags
    controller.app.run_with_args(&empty);
}

fn build_ui(controller: &Controller) {
    controller.build_ui();
}
