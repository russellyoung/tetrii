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
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P, count: u32, width: u32, height: u32, preview: bool) -> Self {
        let controller: Controller = glib::Object::builder().property("application", app).build();
        controller.imp().add_boards(count, width, height, preview);
        controller
    }
}

