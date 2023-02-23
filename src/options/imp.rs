use crate::controller::Controller;
use gtk::{glib, CompositeTemplate};
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

//#[derive(Debug, Default)]
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "options.ui")]
pub struct Options {
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
        let goptions = self.obj();
        let options = self;
        //self.start_button.connect_clicked(clone!(@weak window => move |_| { println!("{:#?}", window); }));
        self.quit_button.connect_clicked(clone!(@weak goptions => move |_| goptions.destroy()));
        self.start_button.connect_clicked(clone!(@weak options => move |_| options.make_controller()));
        // I'm sure this can be done in the template file, but I couldn't find how, either in the doc or testing. I tried
        // setting the "selected" and "selected-item" properties but they did not work
        self.width.set_property("selected", 2u32);
        self.height.set_property("selected", 10u32);
        //        self.obj().set_child(Some(&self.grid));
    }

}

impl Options {
    pub fn set_values(&self, count: u32, width: u16, height: u16, preview: bool) {
        self.boardcount.set_property("selected", count - 1);
        self.width.set_property("selected", (width - 8) as u32);
        self.height.set_property("selected", (height - 10) as u32);
        self.preview_check.set_active(preview);
    }
        
    fn make_controller(&self, ) {
        Controller::new(
            &self.obj().application().unwrap(),
            self.boardcount.selected() + 1,
            self.width.selected() + 8,
            self.height.selected() + 10,
            self.preview_check.is_active())
            .show();
    }
}


impl WidgetImpl for Options {}
impl WindowImpl for Options {}
impl ApplicationWindowImpl for Options {}
