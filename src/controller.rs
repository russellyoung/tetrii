#![allow(unused)]
use crate::Board;
use crate::Config;

use std::rc::Rc;
use std::cell::RefCell;
use gtk::gdk::Display;
use std::fs;

use gtk::prelude::*;

use gtk::{CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};

const APP_ID: &str = "com.young-0.tetrii.rust";

pub struct Controller {
    pub xapp_rc: Rc<gtk::Application>,
    config: Config,
    //window_opt: Option<gtk::Window>,
    boards: Vec<Rc<RefCell<Board>>>,
    self_ref: Option<Rc<RefCell<Controller>>>,
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

impl Controller {
    pub fn new(config: Config) -> Rc<RefCell<Controller>> {
        gtk::init().expect("Error initializing gtk");
        load_css(&config.style);     // needs app to be active before this can be done
        let app = gtk::Application::new( Some(APP_ID), Default::default(), ); 
        let controller = Controller {
            xapp_rc: Rc::new(app),
            config: config,
//            window_opt: None,     // cannot be filled in
            boards: Vec::new(),
            self_ref: None,
            state: State::Setup,
        };
        let controller_ref = RefCell::new(controller);
        let controller_rc = Rc::new(controller_ref);
        let copy_rc = Rc::clone(&controller_rc);
        controller_rc.borrow_mut().self_ref = Some(copy_rc);
        controller_rc
    }
    // accessors: config properties and boards must be accessed through the Controller to avoid
    // share panics
    
    pub fn board_ref(&self, i: usize) -> &Rc<RefCell<Board>> { &self.boards[i] }
    // Config property access
    pub fn prop_u32(&self, prop: &str) -> u32 {
        match prop {
            "boards"      => self.config.boards,
            _             => panic!("property {} does not exist or is not of type u32", prop)
        }
    }
    pub fn prop_u16(&self, prop: &str) -> u16 {
        match prop {
            "width"       => self.config.width,
            "height"      => self.config.height,
            "cell_size"   => self.config.cell_size,
            _             => panic!("property {} does not exist or is not of type u16", prop)
        }
    }
    pub fn prop_f64(&self, prop: &str) -> f64 {
        match prop {
            "delay"       => self.config.delay,
            _             => panic!("property {} does not exist or is not of type f64", prop)
        }
    }
    pub fn prop_bool(&self, prop: &str) -> bool {
        match prop {
            "preview"     => self.config.preview,
            _             => panic!("property {} does not exist or is not of type f64", prop)
        }
    }
    
    pub fn prop_string(&self, prop: &str) -> &String {
        match prop {
            "config_file" => &self.config.config_file,
            "style"       => &self.config.style,
            _             => panic!("property {} does not exist or is not of type f64", prop)
        }
    }
    
    pub fn set_state(&mut self, new_state: State) {
        self.state = new_state;
        if new_state == State::Setup {
            // kill boards
        } else {
            self.boards.iter().for_each(|b| { b.borrow_mut().change_state(new_state); });
        }
    }
    
    pub fn show_boards(&mut self) {
        let mut v:Vec<Rc<RefCell<Board>>> = Vec::new();
        unsafe {   // App is single threaded, there won't be any problem here.
            for i in 0..self.config.boards as usize {
                v.push(Board::new_ref(i + 1, self.get_ref()));
                //self.boards.push(Board::new_ref(i + 1, self.get_ref()));
            }
            //    boards.iter().for_each(|b| { b.borrow().show(); });
            //self.boards.iter().for_each(|b| { b.borrow().show(); });
            v.iter().for_each(|b| { b.borrow().show(); });
        }
    }

    fn get_ref(&self) -> Rc<RefCell<Controller>> {
        let x = self.self_ref.as_ref().unwrap();
        Rc::clone(&x)
    }

    pub fn app_rc(&self) -> Rc<gtk::Application> { Rc::clone(&self.xapp_rc) }
    
    pub fn build_ui(&self) {
        //        let window = MainWindow::new(&self.app);
        let app: &gtk::Application = &self.app_rc();
        let window: &gtk::ApplicationWindow = &gtk::ApplicationWindow::new(app);
        window.set_title(Some(&("Tetrii")));
        let grid = gtk::Grid::builder().row_homogeneous(true).column_homogeneous(true).build();
        let quit_button = gtk::Button::builder().label("Quit").build();
        quit_button.connect_clicked(move |_| println!("Hello World"));
        let start_button = gtk::Button::builder().label("Start").build();
        {
            let copy = self.get_ref();
            start_button.connect_clicked(move |_| copy.borrow_mut().show_boards());
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

