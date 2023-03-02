//pub mod imp;

use gtk::{gio, glib};
//use gtk::prelude::*;
//use gtk::subclass::prelude::*;
//use gtk::glib::clone;
//use gtk::glib::subclass::Signal;

glib::wrapper! {
    pub struct Summary(ObjectSubclass<imp::Summary>)
    @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl Summary {
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P, ) -> Self {
		glib::Object::builder().property("application", app).build()
	}
}

pub mod imp {
    use std::cell::RefCell;

    use gtk::{glib, CompositeTemplate};
    use gtk::glib::clone;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;


    #[derive(Debug, CompositeTemplate, Default)]
    #[template(file = "summary.ui")]
    pub struct Summary {
        per_board: RefCell<Vec<[u32; 9]>>,
        
        #[template_child]
        summary_grid: TemplateChild<gtk::Grid>,
        #[template_child]
        summary_close: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Summary {
        const NAME: &'static str = "Summary";
        type Type = super::Summary;
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

    impl ObjectImpl for Summary {
        // Here we are overriding the glib::Objcet::contructed
        // method. Its what gets called when we create our Object
        // and where we can initialize things.
        fn constructed(&self) {
            self.parent_constructed();
            let summary = self;
            self.summary_close.connect_clicked(clone!(@weak summary => move |_| {
			    summary.obj().hide();
		    }));
        }
    }

    impl WidgetImpl for Summary {}
    impl WindowImpl for Summary {}
    impl ApplicationWindowImpl for Summary {}

    //////////////////////////////////////////////////////////////////

    impl Summary {
        fn len(&self) -> i32 { self.per_board.borrow().len() as i32 }
        
        pub fn initialize(&self, count: u32) {
            let mut boards = self.per_board.borrow_mut();
            boards.clear();
            for _i in 0..count {
                let x: [u32; 9] = Default::default();
                boards.push(x);
            }
            while self.summary_grid.child_at(0, 1).is_some() {
                self.summary_grid.remove_row(1);
            }
        }    

        pub fn update_entry(&self, id: u32, points: u32, lines: u32, piece: u32) {
            let mut boards = self.per_board.borrow_mut();
            let id_usize = id as usize;
            boards[id_usize][0] += points;
            boards[id_usize][1] += lines;
            boards[id_usize][2 + piece as usize] += 1;
        }

        pub fn build_display(&self) {
            let mut totals: [u32; 9] = [0; 9];
            let boards = self.per_board.borrow();
            for i in 0..boards.len() {
                self.add_line_to_display(&(i + 1).to_string(), (i + 1) as i32, &boards[i]);
                Summary::add_to_totals(&mut totals, &boards[i]);
            }
            self.add_line_to_display("Total", self.len() + 1, &totals);
        }

	    pub fn add_line_to_display(&self, text: &str, row: i32, data: &[u32; 9]) {
		    self.summary_grid.attach(&gtk::Label::builder().label(text).build(), 0, row, 1, 1);
		    for i in 0..9 {
                self.summary_grid.attach(&gtk::Label::builder().label(&data[i].to_string()).build(), (i + 1) as i32, row, 1, 1);
		    }
	    }

        fn add_to_totals(totals: &mut [u32; 9], board: &[u32; 9]) {
            for (tref, bval) in (*totals).iter_mut().zip(board) {
                *tref += bval;
            }
        }
    }

}    
