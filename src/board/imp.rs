use self::glib::{BindingFlags, ParamSpec, ParamSpecInt, Value};
use gtk::glib;
use gtk::CompositeTemplate;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use gtk::prelude::GridExt;
use std::cell::Cell;

#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "board.ui")]
pub struct Board {
    pub show_preview: OnceCell<bool>,
    pub width: OnceCell<u32>,
    pub height: OnceCell<u32>,

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
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}


impl ObjectImpl for Board { }

impl WidgetImpl for Board {}
impl BoxImpl for Board {}
