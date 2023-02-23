mod imp;

use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::prelude::IsA;
use gtk::Widget;
use gtk::prelude::GridExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::BoxExt;
use once_cell::sync::OnceCell;
use gtk::glib::closure_local;
use gtk::prelude::ObjectExt;

glib::wrapper! {
    pub struct Board(ObjectSubclass<imp::Board>)
    @extends gtk::Widget, gtk::Box, @implements gio::ActionMap, gio::ActionGroup;
}

impl Board {
    pub fn new (id: u32, width: u32, height: u32, preview: bool) -> Self {
        let board: Board = glib::Object::builder().build();
        board.imp().width_oc.set(width);
        board.imp().height_oc.set(height);
        board.imp().show_preview_oc.set(preview);
        board.imp().id_oc.set(id);
        let this: &imp::Board = board.imp();
        this.playing_area.set_focusable(true);
        for x in 0..width {
            for y in 0..height {
                this.playing_area.attach(&Board::make_cell(), x as i32, y as i32, 1, 1);
            }
        }
        if preview {
            for x in 0..4 {
                for y in 0..2 {
                    this.preview.attach(&Board::make_cell(), x as i32, y as i32, 1, 1);
                }
            }
        }
        board.connect_closure(
            "board-command",
            false,
            closure_local!(|b: Board, id: u32, mask: u32| {
                b.imp().do_command(mask);
            }),
        );
        board.connect_closure(
            "mouse-click",
            false,
            closure_local!(|b: Board, board_num: u32| {
                println!("Mouse click from {}: {:#?}", board_num, b);
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

