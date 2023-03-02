pub mod imp;

use gtk::{gio, glib};
//use gtk::prelude::*;
//use gtk::subclass::prelude::*;
//use gtk::glib::clone;
//use gtk::glib::subclass::Signal;

glib::wrapper! {
    pub struct Summary(ObjectSubclass<imp::Summary>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl Summary {
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P, ) -> Self {
		glib::Object::builder().property("application", app).build()
	}
}

