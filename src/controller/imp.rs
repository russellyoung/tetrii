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
    pub boards_container: TemplateChild<gtk::Box>,
    #[template_child]
    pub total_points: TemplateChild<gtk::Label>,
    #[template_child]
    pub total_lines: TemplateChild<gtk::Label>,
    #[template_child]
    pub start_buttonx: TemplateChild<gtk::Button>,
    #[template_child]
    pub quit_buttonx: TemplateChild<gtk::Button>,
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
        self.quit_buttonx.connect_clicked(clone!(@weak gcontroller => move |_| gcontroller.destroy()));
        //5self.start_buttonx.connect_clicked(clone!(@weak controller => move |x| controller.start_boards(x)));
    }

}

impl Controller {
        
    pub fn add_boards(&self, board_count: u32, width: u32, height: u32, preview: bool) {
        let container = &self.boards_container;
        for i in 0..board_count {
            let b = Board::new(i, width, height, preview);
            container.append(&b);
        }
    }
}


impl WidgetImpl for Controller {}
impl WindowImpl for Controller {}
impl ApplicationWindowImpl for Controller {}
