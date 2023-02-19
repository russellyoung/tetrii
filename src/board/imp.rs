use self::glib::{BindingFlags, ParamSpec, ParamSpecInt, Value};
use gtk::glib;
use gtk::CompositeTemplate;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

use gtk::prelude::GridExt;
use std::cell::Cell;

//#[derive(Debug, Default)]
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "board.ui")]
pub struct Board {
    show_preview: Cell<bool>,
    width: Cell<u32>,
    height: Cell<u32>,

//    #[template_child]
//    pub points: TemplateChild<gtk::Label>,
    #[template_child]
    pub playingarea: TemplateChild<gtk::Grid>,
    #[template_child]
    pub preview: TemplateChild<gtk::Grid>,
    #[template_child]
    pub points: TemplateChild<gtk::Label>,
    #[template_child]
    pub lines: TemplateChild<gtk::Label>,
}

#[glib::object_subclass]
impl ObjectSubclass for Board {
    const NAME: &'static str = "Board";
    type Type = super::Board;
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
        println!("instance_init");
        obj.init_template();
    }
}


impl ObjectImpl for Board {
    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Lazy<Vec<ParamSpec>> =
            Lazy::new(|| vec![
                ParamSpecInt::builder("width").build(),
                ParamSpecInt::builder("height").build(),
                ParamSpecInt::builder("preview").build(),
            ]);
        PROPERTIES.as_ref()
    }

    fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
        match pspec.name() {
            "height" => {
                let height: i32 = value.get().unwrap();
                if height < 10 || height > 50 {
                    panic!("Height must be between 10 and 50");
                }
                self.height.replace(height as u32);
            },
            "width" => {
                let width: i32 = value.get().unwrap();
                if width < 8 || width > 28 {
                    panic!("width must be between 8 and 28");
                }
                self.width.replace(width as u32);
            },
            "preview" => {self.show_preview.replace(value.get::<i32>().unwrap() > 0); },
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "height" => self.height.get().to_value(),
            "width" => self.width.get().to_value(),
            "preview" => self.show_preview.get().to_value(),
            _ => unimplemented!(),
        }
    }
    
    // You must call `Widget`'s `init_template()` within `instance_init()`.
//    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
//        obj.init_template();
//    }
    // Here we are overriding the glib::Objcet::contructed
    // method. Its what gets called when we create our Object
    // and where we can initialize things.
    fn constructed(&self) {
        self.parent_constructed();
//        self.obj().set_child(Some(&self.grid));
    }



}




/*
struct UtilityCallbacks {}

#[gtk::template_callbacks(functions)]
impl UtilityCallbacks {
}

impl UtilityCallbacks {


}
*/



impl WidgetImpl for Board {}
impl WindowImpl for Board {}
impl ApplicationWindowImpl for Board {}
