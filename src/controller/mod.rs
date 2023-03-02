pub mod imp;

use crate::controller::imp::summary::Summary;

use gtk::{gio, glib};
use gtk::glib::closure_local;
use gtk::subclass::prelude::*;
use gtk::prelude::ObjectExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::GtkWindowExt;
use crate::controller::imp::{has_instance, set_instance, controller_full};
use gtk::prelude::GtkApplicationExt;

glib::wrapper! {
    pub struct Controller(ObjectSubclass<imp::Controller>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl Controller {
    fn new<P: glib::IsA<gtk::Application>>(app: &P, count: u32, width: u32, height: u32, preview: bool) -> Self {
        let controller: Controller = glib::Object::builder().property("application", app).build();
        controller.imp().internal.borrow_mut().summary = Some(Summary::new(app));
        controller.imp().initialize(count, width, height, preview);
        
        controller.set_focusable(true);
        controller.connect_closure(
            "board-report",
            false,
            closure_local!(|ctrlr: Controller, id: u32, points: u32, lines: u32, piece_num: u32| {
                let _ = &ctrlr.imp().piece_crashed(id, points, lines, piece_num);
            }),
        );
        controller.connect_closure(
            "board-lost",
            false,
            closure_local!(|ctrlr: Controller, id: u32| {
                let _ = &ctrlr.imp().board_lost(id);
            }),
        );
        controller.connect_closure(
            "mouse-click",
            false,
            closure_local!(|ctrlr: Controller, id: u32, button: u32, | {
                let _ = &ctrlr.imp().mouse_click(id, button);
            }),
        );
        controller.connect_closure(
            "select",
            false,
            closure_local!(|ctrlr: Controller, id: u32, | {
                let _ = &ctrlr.imp().set_board(id);
            }),
        );
        controller
    }

	// This gets a ref to an existing one and only makes a new one if it does not exist. Maybe rethink?
    pub fn new_ref<P: glib::IsA<gtk::Application>>(app: &P, count: u32, width: u32, height: u32, preview: bool) -> &crate::controller::Controller {
		if !has_instance() {
			set_instance(Controller::new(app, count, width, height, preview));
		}
		controller_full()
	}

	pub fn exit(self) {
		self.application().unwrap().windows().iter().for_each(|w| w.destroy());
	}
}

