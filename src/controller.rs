#![allow(unused)]
use crate::Board;
use crate::Config;

use std::rc::Rc;
use std::cell::RefCell;
use gtk::gdk::Display;
use std::fs;

use gtk::prelude::*;

use gtk::{CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};

pub struct Controller<'a> {
    pub app: &'a gtk::Application,
    pub config: Config,
    boards: Vec<Board>,
    state: State
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State { Setup, Paused, Running, Finished, }

impl State {
    fn toggle(&self) -> State {
        match self {
            State::Finished => State::Finished,
            State::Paused => State::Running,
            State::Running => State::Paused,
            State::Setup => State::Setup,
        }
    }
}

impl<'a> Controller<'a> {
    pub fn new(app: &'a gtk::Application) -> Controller {
        gtk::init().expect("Error initializing gtk");
        let config = Config::build_config();
        load_css(&config.style);     // needs app to be active before this can be done
        Controller {
            app: app,
            config: config,
            boards: Vec::new(),
            state: State::Setup,
        }
    }
    // accessors: config properties and boards must be accessed through the Controller to avoid
    // share panics
    
    pub fn set_state(&mut self, new_state: State) {
        self.state = new_state;
        if new_state == State::Setup {
            // kill boards
        } else {
//            self.boards.iter().for_each(|b| { b.borrow_mut().change_state(new_state); });
        }
    }
    
    pub fn show_boards(&mut self) {
        for i in 0..self.config.boards as usize {
            self.boards.push(Board::new(i + 1, &self));
        }
        self.boards.iter().for_each(|b| { b.show(); });
    }

    pub fn build_ui(&mut self) {
        //        let window = MainWindow::new(&self.app);
        let window: &gtk::ApplicationWindow = &gtk::ApplicationWindow::new(self.app);
        window.set_title(Some(&("Tetrii")));
        let grid = gtk::Grid::builder().row_homogeneous(true).column_homogeneous(true).build();
        let quit_button = gtk::Button::builder().label("Quit").build();
        quit_button.connect_clicked(move |_| println!("Hello World"));
        let start_button = gtk::Button::builder().label("Start").build();
        {
            // ?? s there any way to pass this structure into the handler?
            //start_button.connect_clicked(move |_| xxx.show_boards());
        }
        let pause_button = gtk::Button::builder().label("Pause").build();
        grid.attach(&start_button, 0, 0, 1, 1);
        grid.attach(&pause_button, 1, 0, 1, 1);
        grid.attach(&quit_button, 2, 0, 1, 1);
        window.set_child(Some(&grid));
        window.show();
        //        self.state = State::Paused;
        //        self.show_boards();
    }
    
    
}

// loads the CSS file, exits with error message if ti can't be found. The program will not work at all without proper CSS
fn load_css(filename: &String) {
    let provider = CssProvider::new();
    let css_data = fs::read(filename).expect("could not find CSS file");
    provider.load_from_data(&css_data);
    StyleContext::add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

