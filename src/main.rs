#![allow(dead_code)]

mod board;
mod config;
mod options;
mod controller;

use config::Config;
use options::Options;
use board::Board;

use std::fs;
use std::env;
use gtk::{CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::gdk::Display;
use gtk::prelude::*;

use once_cell::sync::Lazy;

const APP_ID: &str = "com.young-0.tetrii.rust";

fn main() {
    let config = Config::build_config();
    gtk::init().expect("Error initializing gtk");
    let app = gtk::Application::new( Some(APP_ID), Default::default(), );
    let height = config.height;
    let width = config.width;
    let preview = config.preview;
    app.connect_activate(move |appx| {
        load_css("style.css");     // needs app to be active before this can be done
        //let win = Board::new(app, 10, 20, 0);
        let  win = Options::new(appx);
        win.show();
        win.set_defaults(config.boards, width, height, preview);
    });
    let empty: Vec<String> = vec![];  // thanks to stackoverflow, I learned EMPTY is needed to keep GTK from interpreting the command line flags
    app.run_with_args(&empty);
}

// read the css file. If it is not in the current directory go pu the tree to look for it. This way
// the program will run from any of the crate subdirectories.
fn load_css(filename: &str) {
    let mut path = env::current_dir().unwrap();
	path.push(filename);
    let mut css_data = fs::read(&path);//.expect("could not find CSS file");
	while css_data.is_err() {
		path.pop();
		if !path.pop() {
			panic!("Cannot find file {} anywhere on the current trunk, file is required for program to run", filename);
		}
		path.push(filename);
		css_data = fs::read(&path);//.expect("could not find CSS file");
	}
    let provider = CssProvider::new();
    provider.load_from_data(&css_data.unwrap());
    StyleContext::add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
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

fn board(which: usize) -> &'static Board { unsafe { &BOARDS[which] } }
