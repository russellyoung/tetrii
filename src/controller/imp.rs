use gtk::prelude::*;
use gtk::subclass::prelude::*;
//use gtk::glib;
use gtk::prelude::GridExt;
use gtk::{glib, CompositeTemplate};
use gtk::glib::clone;
use crate::Board;

//#[derive(Debug, Default)]
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "controller.ui")]
pub struct Controller {
    #[template_child]
    pub boardcount: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub width: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub height: TemplateChild<gtk::DropDown>,
    #[template_child]
    pub start_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub quit_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub preview_check: TemplateChild<gtk::CheckButton>,
    //    pub grid: gtk::Grid,
}

#[glib::object_subclass]
impl ObjectSubclass for Controller {
    const NAME: &'static str = "Controller";
    type Type = super::Controller;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
//        klass.bind_template_callbacks();
//        UtilityCallbacks::bind_template_callbacks(klass);
        // You can skip this if empty
    }

//    fn new() -> Self {
//        let controller = Self { grid: gtk::Grid::builder().row_homogeneous(true).column_homogeneous(true).build(), };
//        controller
//    }

    // You must call `Widget`'s `init_template()` within `instance_init()`.
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}


impl ObjectImpl for Controller {
    // You must call `Widget`'s `init_template()` within `instance_init()`.
//    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
//        obj.init_template();
//    }
    // Here we are overriding the glib::Objcet::contructed
    // method. Its what gets called when we create our Object
    // and where we can initialize things.
    fn constructed(&self) {
        self.parent_constructed();
        let gcontroller = self.obj();
        let controller = self;
        //self.start_button.connect_clicked(clone!(@weak window => move |_| { println!("{:#?}", window); }));
        self.quit_button.connect_clicked(clone!(@weak gcontroller => move |_| gcontroller.destroy()));
        self.start_button.connect_clicked(clone!(@weak controller => move |x| controller.start_boards(x)));
        // I'm sure this can be done in the template file, but I couldn't find how, either in the doc or testing. I tried
        // setting the "selected" and "selected-item" properties but they did not work
        self.width.set_property("selected", 2u32);
        self.height.set_property("selected", 10u32);
//        self.obj().set_child(Some(&self.grid));
    }

}

#[gtk::template_callbacks]
impl Controller {
    #[template_callback]
    fn start_boards(&self, button: &gtk::Button) {
        let board_count = self.boardcount.selected() + 1;
        let width = self.width.selected() + 8;
        let height = self.height.selected() + 10;
        let preview = self.preview_check.is_active();
        let app = &self.obj().application().unwrap();
        for i in 0u32..board_count {
            let b = Board::new(i, app, width, height, preview);
            b.show();
        }
            
        println!("boardcount is {:#?}, width is {:#?}, height is {:#?}", self.boardcount.selected_item(), self.width, self.height);
        button.set_label("I was clicked!");
//        self.label.set_label("The button was clicked!");
    }
    #[template_callback(function, name = "strlen")]
    fn string_length(s: &str) -> u64 {
        s.len() as u64
    }
}


impl WidgetImpl for Controller {}
impl WindowImpl for Controller {}
impl ApplicationWindowImpl for Controller {}
