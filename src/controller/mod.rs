mod imp;

use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::prelude::IsA;
use gtk::Widget;
use gtk::prelude::GridExt;

glib::wrapper! {
    pub struct Controller(ObjectSubclass<imp::Controller>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl Controller {
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P) -> Self {
        glib::Object::builder().property("application", app).build()
    }

//    pub fn attach(&self, button: &impl IsA<Widget>, x: i32, y: i32) {
//        self.imp().grid.attach(button, x, y, 1, 1);
//    }
    
}

