pub mod imp;

use gtk::subclass::prelude::*;
use gtk::{gio, glib};

glib::wrapper! {
    pub struct Options(ObjectSubclass<imp::Options>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl Options {
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P) -> Self { glib::Object::builder().property("application", app).build() }

    pub fn set_values(&self, count: u32, width: u32, height: u32, preview: bool) { self.imp().set_values(count, width, height, preview); }

    pub fn make_controller(&self, ) { self.imp().make_controller(); }
}

