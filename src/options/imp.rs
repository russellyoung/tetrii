use crate::controller::Controller;
use std::cell::RefCell;
use std::rc::Rc;
use std::fs;
use std::env;


use gtk::{glib, CompositeTemplate};
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::gdk::Display;

#[derive(Debug, )]
struct Internal {
	count: u32,
	height: u32,
	width: u32,
    cell_size: u32, 
	preview: bool,
}
impl Default for Internal {
    fn default() -> Internal { Internal { count: 2, height: 20, width: 10, cell_size: 25, preview: true, }}
}

//#[derive(Debug, Default)]
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "options.ui")]
pub struct Options {
	internal: Rc<RefCell<Internal>>,
	
    #[template_child]
    pub board_count: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub width_widget: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub height_widget: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub cell_widget: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub apply_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub cancel_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub preview_check: TemplateChild<gtk::CheckButton>,
    //    pub grid: gtk::Grid,
}

#[glib::object_subclass]
impl ObjectSubclass for Options {
    const NAME: &'static str = "Options";
    type Type = super::Options;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
//        klass.bind_template_callbacks();
//        UtilityCallbacks::bind_template_callbacks(klass);
        // You can skip this if empty
    }

    // You must call `Widget`'s `init_template()` within `instance_init()`.
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}


impl ObjectImpl for Options {
    // You must call `Widget`'s `init_template()` within `instance_init()`.
//    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
//        obj.init_template();
//    }
    // Here we are overriding the glib::Objcet::contructed
    // method. Its what gets called when we create our Object
    // and where we can initialize things.
    fn constructed(&self) {
        self.parent_constructed();
        let options = self;
        self.cancel_button.connect_clicked(clone!(@weak options => move |_| {
			options.set_display_from_values();
			options.obj().hide();
		}));
        self.apply_button.connect_clicked(clone!(@weak options => move |_| {
			options.set_values_from_display();
			options.remake_controller();
			options.obj().hide();
		}));
        // I'm sure this can be done in the template file, but I couldn't find how, either in the doc or testing. I tried
        // setting the "selected" and "selected-item" properties but they did not work
        self.width_widget.set_property("selected", 2u32);
        self.height_widget.set_property("selected", 10u32);
        //        self.obj().set_child(Some(&self.grid));
    }

}

impl WidgetImpl for Options {}
impl WindowImpl for Options {}
impl ApplicationWindowImpl for Options {}

impl Options {
	pub fn destroy(&self) { self.obj().destroy(); }

	// inject values into options, store in struct and display in ui
    pub fn set_values(&self, count: u32, width: u32, height: u32, cell_size: u32, preview: bool) {
		{
			let mut internal = self.internal.borrow_mut();
            if internal.cell_size != cell_size { set_cellsize(cell_size); }
			(internal.count, internal.width, internal.height, internal.cell_size, internal.preview) = (count, width, height, cell_size, preview);
		}
		self.set_display_from_values();
	}

	// set display values from struct
	fn set_display_from_values(&self) {
		let internal = self.internal.borrow();
        self.board_count.set_property("selected", internal.count - 1);
        self.width_widget.set_property("selected", internal.width - 8);
        self.height_widget.set_property("selected", internal.height - 10);
        self.cell_widget.set_property("selected", (internal.cell_size - 10)/5);
        self.preview_check.set_active(internal.preview);
    }

	// update struct values from display
	fn set_values_from_display(&self) {
		let mut internal = self.internal.borrow_mut();
        let cell_pixels = self.cell_widget.selected()*5 + 10;
        if internal.cell_size != cell_pixels {
            set_cellsize(cell_pixels);
        }
		(internal.count, internal.width, internal.height, internal.cell_size, internal.preview) = (
            self.board_count.selected() + 1,
            self.width_widget.selected() + 8,
            self.height_widget.selected() + 10,
            cell_pixels,
            self.preview_check.is_active(),
        );
	}
        
    pub fn make_controller(&self, ) {
		let internal = self.internal.borrow();
        Controller::new_ref(&self.obj().application().unwrap(),internal.count, internal.width, internal.height, internal.preview)
            .show();
    }

    pub fn remake_controller(&self, ) {
		let internal = self.internal.borrow();
		let controller = crate::controller_inst();
		controller.initialize(internal.count, internal.width, internal.height, internal.preview);
	}
}

fn set_cellsize(size: u32) {
    let str = format!(".cell {{ min-height: {}px; min-width: {}px;}}", size, size);
    read_style(str.as_bytes());
}

pub fn read_style(css_data: &[u8]) {
    let provider = CssProvider::new();
    provider.load_from_data(css_data);
    StyleContext::add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn load_style_from_file(filename: &str) {
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
    read_style(&css_data.unwrap());
}

