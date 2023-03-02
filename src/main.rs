#![allow(dead_code)]

mod board;
mod config;
mod options;
mod controller;

use config::Config;
use options::Options;
use board::Board;

use gtk::prelude::*;
use gtk::subclass::prelude::ObjectSubclassIsExt;

use once_cell::sync::Lazy;

const APP_ID: &str = "com.young-0.tetrii.rust";

fn main() {
    let config = Config::build_config();
    gtk::init().expect("Error initializing gtk");
    let app = gtk::Application::new( Some(APP_ID), Default::default(), );
    let height = config.height;
    let width = config.width;
    let preview = config.preview;
    let cell_size = config.cell_size;
    app.connect_activate(move |appx| {
        //let win = Board::new(app, 10, 20, 0);
        let  options = Options::new(appx);
		options::imp::load_style_from_file("style.css");
        options.set_values(config.boards, width, height, cell_size, preview);
		options.make_controller();
		unsafe {OPTIONS = Some(options); }
    });
    let empty: Vec<String> = vec![];  // thanks to stackoverflow, I learned EMPTY is needed to keep GTK from interpreting the command line flags
    app.run_with_args(&empty);
}
fn exit() {
	controller_inst().destroy();
	options_inst().destroy();
}

//////////////////////////////////////////////////////////////////
//
// STATIC MUTS
//
// I don't like statics in any language - they often are a lazy man's solution. Still, sometimes they are needed,
// and these are the ones that either I can't avoid or are too much work to avoid (LMS). I also don't like using
// UNSAFE. The name gives the impression, I'm sure intentionally, that it is not encouraged. So, here also are
// accessor functions to move all the UNSAFEs out of the rest of the code.
//
// (MORE)
//////////////////////////////////////////////////////////////////
static mut BOARDS: Lazy<Vec<Board>> = Lazy::new(Vec::new);
static mut CONTROLLER: Option<crate::controller::Controller> = None;
static mut OPTIONS: Option<crate::options::Options> = None;

fn board(which: usize) -> &'static Board { unsafe { &BOARDS[which] } }
fn controller_inst<'a>() -> &'a crate::controller::imp::Controller { unsafe { CONTROLLER.as_ref().unwrap().imp() }}
fn options_inst<'a>() -> &'a crate::options::imp::Options { unsafe { OPTIONS.as_ref().unwrap().imp() }}
