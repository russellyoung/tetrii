// TODO: implement cell_size
mod board;
mod config;
use std::fs;

use config::Config;
use board::Board;
use gtk::prelude::*;
use gtk::{CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::gdk::Display;
use std::rc::Rc;
use std::cell::RefCell;

const APP_ID: &str = "com.young-0.tetrii.rust";

fn main() {
    // CONFIG gets data by merging command line args, config file, and defaults
    let config = Config::build_config();

    let app = gtk::Application::new(
        Some(APP_ID),
        Default::default(),
    );
    match gtk::init() {
        Err(err) => eprintln!("ERROR: {}", err),
        _ => (),
    }
    
    load_css(&config.style);     // needs app to be active before this can be done

    let boards: Vec<Rc<Board>> = Vec::new();
    let p_boards: RefCell<Vec<Rc<Board>>> = RefCell::new(boards);
    app.connect_activate( move |app| { build_ui(app, &p_boards, &config); });

    let empty: Vec<String> = vec![];  // thanks to stackoverflow, I learned EMPTY is needed to keep GTK from interpreting the command line flags
    app.run_with_args(&empty);
}

// loads the CSS file, exits with error message if ti can't be found. The program will not work at all
// without proper CSS
fn load_css(filename: &String) {
    let provider = CssProvider::new();
    let binding = match fs::read(filename) {
        Ok(x) => x,
        Err(_err) => panic!("Could not find CSS file {}", filename),
    };
    provider.load_from_data(&binding);
    StyleContext::add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &gtk::Application, p_boards: &RefCell<Vec<Rc<Board>>>, config: &Config) {
    let mut boards = p_boards.borrow_mut();
    for i in 0..config.boards as usize {
        boards.push(Board::new(i + 1, app, config));
    }
    boards.iter().for_each(|b| { b.show(); });
}

