pub mod imp;

use gtk::{gio, glib};
use gtk::glib::closure_local;
use gtk::subclass::prelude::*;
use gtk::prelude::{GridExt, WidgetExt, ObjectExt, BoxExt};

glib::wrapper! {
    pub struct Board(ObjectSubclass<imp::Board>)
    @extends gtk::Widget, gtk::Box, @implements gio::ActionMap, gio::ActionGroup;
}

impl Board {
    pub fn new (id: u32, width: u32, height: u32, preview: bool) -> Self {
        let board: Board = glib::Object::builder().build();
        let _ = board.imp().width_oc.set(width);
        let _ = board.imp().height_oc.set(height);
        let _ = board.imp().show_preview_oc.set(preview);
        let _ = board.imp().id_oc.set(id);
        let this: &imp::Board = board.imp();
        this.obj().set_focusable(true);
        for x in 0..width {
            for y in 0..height {
                this.playing_area.attach(&Board::make_cell(), x as i32, y as i32, 1, 1);
            }
        }
        if preview {
            for x in 0..4 {
                for y in 0..2 {
                    this.preview.attach(&Board::make_cell(), x, y, 1, 1);
                }
            }
        }
        board.connect_closure(
            "board-command",
            false,
            closure_local!(|b: Board, _id: u32, mask: u32| {
                b.imp().do_command(mask);
            }),
        );
        board.imp().prepare();
        board
    }

    // helper function to make a single cell
    fn make_cell() -> gtk::Box {
        let cell = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let label = gtk::Label::builder()
//            .label("")
            .build();
        label.add_css_class("cell");
        //cell.add_css_class("cell");
        cell.append(&label);
        cell
    }
}

