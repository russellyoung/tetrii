mod imp;

use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::prelude::IsA;
use gtk::Widget;
use gtk::prelude::GridExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::BoxExt;
use once_cell::sync::OnceCell;

glib::wrapper! {
    pub struct Board(ObjectSubclass<imp::Board>)
    @extends gtk::Widget, gtk::Box, @implements gio::ActionMap, gio::ActionGroup;
}

impl Board {
    pub fn new (num: u32, width: u32, height: u32, preview: bool) -> Self {
        let title = format!("Board {}", num + 1);
        let board: Board = glib::Object::builder().build();
        board.imp().width.set(width);
        board.imp().height.set(height);
        board.imp().show_preview.set(preview);

        let this: &imp::Board = board.imp();
        this.playingarea.set_focusable(true);
        for x in 0..width {
            for y in 0..height {
                this.playingarea.attach(&Board::make_cell(), x as i32, y as i32, 1, 1);
            }
        }
        if preview {
            for x in 0..4 {
                for y in 0..2 {
                    this.preview.attach(&Board::make_cell(), x as i32, y as i32, 1, 1);
                }
            }
        }
        board
    }

    // helper function to make a single cell
    fn make_cell() -> gtk::Box {
        let cell = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let label = gtk::Label::builder()
            .label("")
            .build();
        label.add_css_class("cell");
        //cell.add_css_class("cell");
        cell.append(&label);
        cell
    }
//    pub fn attach(&self, button: &impl IsA<Widget>, x: i32, y: i32) {
//        self.imp().grid.attach(button, x, y, 1, 1);
//    }
    
}

