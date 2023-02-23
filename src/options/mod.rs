mod imp;

use gtk::subclass::prelude::*;
use gtk::{gio, glib};

glib::wrapper! {
    pub struct Options(ObjectSubclass<imp::Options>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl Options {
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub fn set_defaults(&self, count: u32, width: u16, height: u16, preview: bool) {
        self.imp().set_values(count, width, height, preview);
    }
}

