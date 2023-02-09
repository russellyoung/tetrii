#![allow(unused)]
use crate::Config;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::collections::HashMap;
use std::any::type_name;

use fastrand;
use gtk::glib;
use gdk4::ModifierType;
use gtk::prelude::*;
use gtk::glib::clone;

fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}

// masks: the shapes are stored in a 4x4 grid. In each case the initial orientation leaves the top row
// empty, so the initial placement is at (width/2 - 2, -1). Rotation can use any of the 16 cells.
// The masks are designed so the first row (last hex digit) is always blank and the firstpiece starts
// in a horizontal as close to centered as possible, or towards the right if uneven (as most are).
// This way each piece can be drawn in its initial position by centering the X coord and setting the
// Y coord to -1
static PIECES: [Piece; 7] = [
    Piece {name: &"Bar",        points: [12, 1, 12, 1, ], masks: [0x00f0, 0x2222, 0x00f0, 0x2222, ], },
    Piece {name: &"Tee",        points: [ 6, 5,  2, 1, ], masks: [0x0270, 0x0232, 0x0072, 0x0262, ], },
    Piece {name: &"Square",     points: [ 4, 4,  4, 4, ], masks: [0x0660, 0x0660, 0x0660, 0x0660, ], },
    Piece {name: &"Zee",        points: [ 5, 3,  5, 3, ], masks: [0x0360, 0x0462, 0x0360, 0x0462, ], },
    Piece {name: &"ReverseZee", points: [ 5, 3,  5, 3, ], masks: [0x0630, 0x0264, 0x0630, 0x0264, ], },
    Piece {name: &"El",         points: [ 6, 6,  3, 3, ], masks: [0x0470, 0x0322, 0x0071, 0x0226, ], },
    Piece {name: &"ReverseEl",  points: [ 3, 3,  6, 6, ], masks: [0x0740, 0x2230, 0x0170, 0x0622, ], },
];

// default commands to set up the map. The CHEAT entries can have ad hoc stuff added, the current values
// set the next piece, which is useful for debugging (and also for getting out of tight spots)
// TODO: allow custom configurations in the config file
const COMMANDS:[(&str, Command); 16] =
    [(&"Right", Command::Right),
     (&"Left", Command::Left),
     (&"Down", Command::Down),
     (&"q", Command::RotateLeft),
     (&"q-Shift", Command::Left),
     (&"e", Command::RotateRight),
     (&"Mouse1", Command::Left),
     (&"Mouse2", Command::Down),
     (&"Mouse3", Command::Right),
     (&"Cheat(1)", Command::Cheat(1)),
     (&"Cheat(2)", Command::Cheat(2)),
     (&"Cheat(3)", Command::Cheat(3)),
     (&"Cheat(4)", Command::Cheat(4)),
     (&"Cheat(5)", Command::Cheat(5)),
     (&"Cheat(6)", Command::Cheat(6)),
     (&"Cheat(7)", Command::Cheat(7)),
];

// Coordinate systems:
//
// There are 2 coordinate systems in use in the program. One is used for the CSS Grid. This is
// numbered from (0, 0) in the upper right to (width - 1, height - 1) in the lower left.
// A separate system is used for the bitmap. In this there are borders of unused bits to avoid
// having to check for overrun or underrun. The bitmap has an extra row on top, 2 on each side,
// and one on the bottom, so bitmap lines and columns need to be adjusted to get gri

// if I have time and interest the commands will be configurable through the .tetrii file
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Command {Left, Right, Down, RotateRight, RotateLeft, Drop, Pause, Resume, TogglePause, SetBoard(i32), Cheat(usize), Nop, }
#[derive(Copy, Clone, Debug)]
pub enum Orientation {North, East, South, West, }

impl Orientation {
    pub fn rotate(&self, command: Command) -> Orientation {
        match command {
            Command::RotateRight => {
                match self {
                    Orientation::North => Orientation::East,
                    Orientation::East => Orientation::South,
                    Orientation::South => Orientation::West,
                    Orientation::West => Orientation::North,
                }},
            Command::RotateLeft => {
                match self {
                    Orientation::North => Orientation::West,
                    Orientation::East => Orientation::North,
                    Orientation::South => Orientation::East,
                    Orientation::West => Orientation::South,
                }},
            _ => *self
        }
    }

    pub fn offset(&self) -> usize {
        match self {
            Orientation::North => 0,
            Orientation::East => 1,
            Orientation::South => 2,
            Orientation::West => 3,
        }
    }
}

#[derive(Debug)]
pub struct Piece {
    // NAME is used to identify the piece. It also is the name of the CSS class used to draw the piece.
    name: &'static str,
    // These arrays give the values for each piece. There are 4 for each - some pieces need fewer (BAR
    // needs 2, SQUARE needs 1), but rather than deal with different length vectors it is simpler just
    // to repeat the values until there are 4.
    // (NOTE: other implementations have used circular linked lists to manage this. Listing 4 rotations for
    // each object probably takes less code than handling the different cases individually)
    points: [u8; 4],
    masks: [u16; 4],
}
impl Piece {
    fn points(&self, orientation: Orientation) -> u8 {
        self.points[orientation.offset()]
    }
    // MASK is a u16 value interpreted as 4 lines of length 4 bits. This can encode all rotations of the pieces.
    fn mask(&self, orientation: Orientation) -> u16 {
        self.masks[orientation.offset()]
    }
    // BIG_MASK is a u32 value that elso encodes the pieces. It is used in drawing pieces. The simplest way to
    // redraw pieces after a move is to erase them and redraw them in the new position. This works but is inefficient,
    // and the pieces seem to flicker as they all get redrawn. By embedding the 4x4 map into a 6x5 one the old
    // position and the new one can be compared, and cells only rewritten if they change.
    //
    //    . x x x x .     This is what the masks look like. dx can only be -1, 0, or 1, and dy can only be 0 or 1
    //    . x x x x .
    //    . x x x x .
    //    . x x x x .
    //    . . . . . .
    
    fn big_mask(&self, orientation: Orientation, dx: i32, dy: i32) -> u32 {
        // these catch programming errors
        assert!(dx == -1 || dx == 0 || dx == 1, "ERROR: bad dx value when moving piece");
        assert!(dy == 0 || dy == 1, "ERROR: bad dy value when moving piece");
        let mut big_mask:u32 = 0x0;
        let mut mask = self.mask(orientation);
        let mut shift = 1 + dx + 6*dy;
        while mask != 0 {
            let slice: u32 = (mask & 0xf) as u32;
            big_mask |= slice << shift;
            mask >>= 4;
            shift += 6;
        }
        big_mask
    }
    
    fn random() -> &'static Piece {
        let random = fastrand::usize(0..PIECES.len());
        &PIECES[random]
    }

}

#[derive(Debug, Clone)]
pub struct Board {
    // immutable
    num:           usize,
    width:         i32,
    height:        i32,
    window:        gtk::Window,
    grid:          gtk::Grid,
    command_hash:  HashMap<String, Command>,

    // mutable (maybe put these in a STATE struct?)
    x:           i32,
    y:           i32,
    orientation: Orientation,
    piece:       &'static Piece,
    next_piece:  &'static Piece,
    bitmap:      Vec<u32>,
}

impl Board {
    pub fn new(num: usize, app: &gtk::Application, config: &Config) -> Rc<RefCell<Board>> {
        let grid = gtk::Grid::builder().build();
        grid.set_focusable(true);
        grid.add_css_class("board");
        
        let mut bitmap = vec![0xffffffff; (config.height + 4) as usize];
        let mask = !(((0x1 << config.width) - 1) << 2);
        for i in 0..bitmap.len() - 2 {
            bitmap[i] &= mask;
        }
        let mut board = Board{num: num,
                              width: config.width as i32,
                              height: config.height as i32,
                              window: gtk::ApplicationWindow::new(app).into(),
                              grid: grid,
                              command_hash: Board::init_command_hash(),

                              bitmap: bitmap,
                              x: 0,
                              y: 0,
                              orientation: Orientation::North,
                              piece: &PIECES[0],     // initial piece is discarded
                              next_piece: if config.initial_piece < PIECES.len() { &PIECES[config.initial_piece]} else { Piece::random() },
        };
        // bitmap is a map of the board with 0 for empty spaces and 1 for filled. Initialize it so all bits representing
        // cells of the bitmap are 0 and all other bits are 1. To avoid having to worry about overflow or underflow there
        // is a border of at least 2 set bits on the left, right, and bottom of the bitmap. This means that the maximum
        // allowable width, using a 32-but mask, is 28 columns.
        // 
        for row in 0..board.width {
            for col in 0..board.height {
                let square = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .build();
                let label = gtk::Label::builder()
                    .label("")
                    .build();
                label.add_css_class("cell");
                square.append(&label);
                board.grid.attach(&square, row, col, 1, 1);
            }
        }
        board.window.set_title(Some(&["Board ", &board.num.to_string()].concat()));
        board.window.set_child(Some(&board.grid));
        board.start_new_piece();

        // add handlers
        let ref_board = RefCell::new(board);
        let rc_board = Rc::new(ref_board);
        let key_handler = gtk::EventControllerKey::new();
        rc_board.borrow().grid.add_controller(&key_handler);
        let rc_board_key_handler = Rc::clone(&rc_board);
        key_handler.connect_key_pressed(move |_ctlr, key, _code, state| {
            rc_board_key_handler.borrow_mut().keyboard_input(key, state);
            gtk::Inhibit(false)
        });

        let mouse_handler1 = gtk::GestureClick::builder().button(1).build();
        let rc_board_click1_handler = Rc::clone(&rc_board);
        rc_board.borrow().grid.add_controller(&mouse_handler1);
        mouse_handler1.connect_pressed(move |_, _, _, _ | {
            rc_board_click1_handler.borrow_mut().click_input("Mouse1");
        });
        let mouse_handler2 = gtk::GestureClick::builder().button(2).build();
        let rc_board_click2_handler = Rc::clone(&rc_board);
        rc_board.borrow().grid.add_controller(&mouse_handler2);
        mouse_handler2.connect_pressed(move |_, _, _, _ | {
            rc_board_click2_handler.borrow_mut().click_input("Mouse2");
        });
        let mouse_handler3 = gtk::GestureClick::builder().button(3).build();
        let rc_board_click3_handler = Rc::clone(&rc_board);
        rc_board.borrow().grid.add_controller(&mouse_handler3);
        mouse_handler3.connect_pressed(move |_, _, _, _ | {
            rc_board_click3_handler.borrow_mut().click_input("Mouse3");
        });
        rc_board
    }

    // builds the command hash from the array definition
    fn init_command_hash() -> HashMap<String, Command> {
        let mut command_hash: HashMap<String, Command> = HashMap::new();
        for desc in COMMANDS {
            command_hash.insert(desc.0.to_string(), desc.1);
        }
        command_hash
    }
    
    pub fn show(&self) { self.window.show(); }

    //////////////////////////////////////////////////////////////////
    //
    // Piece handling
    //
    //////////////////////////////////////////////////////////////////

    // called to load a new piece in the board. The drawing function might be able to be merged with draw_moved_piece()?
    fn start_new_piece(&mut self) {
        self.piece = self.next_piece;
        self.next_piece = Piece::random();
        self.orientation = Orientation::North;
        (self.x, self.y) = (self.width/2 - 2, -1);
        let mut mask = self.piece.mask(self.orientation);
        let name = self.piece.name.to_string();
        let mut i = 0;
        while mask != 0 {
            if mask & 1 == 1 {
                let row = i / 4;
                let col = i % 4;
                self.set_cell(self.x + col, self.y + row, &name);
            }
            i += 1;
            mask >>= 1;
        }
        self.grid.child_at(5, 5).unwrap().set_css_classes(&["cell", "bar"]);
    }

    fn click_input(&mut self, button: &str) {
        let command = *self.command_hash.get(button).unwrap_or(&Command::Nop);
        // not sure if I can get modifer keys here
        self.do_command(&command);
    }

    fn keyboard_input(&mut self, key: gdk4::Key, modifiers: ModifierType) {
        let mut key_string = key.to_lower().name().unwrap().to_string();
        let bits = modifiers.bits();
        if bits & ModifierType::SHIFT_MASK.bits() != 0 { key_string.push_str("-Shift"); }
        if bits & ModifierType::ALT_MASK.bits() != 0 { key_string.push_str("-Alt"); }
        if bits & ModifierType::CONTROL_MASK.bits() != 0 { key_string.push_str("-Ctrl"); }
        if bits & ModifierType::META_MASK.bits() != 0 { key_string.push_str("-Meta"); }
        let command = *self.command_hash.get(&key_string).unwrap_or(&Command::Nop);
        self.do_command(&command);
    }
    
    //fn mouse_input(&self, 
    fn do_command(&mut self, command: &Command) {
        let succeeded = match command {
            Command::Left => self.translate_piece(1, 0),
            Command::Right => self.translate_piece(-1, 0),
            Command::Down => self.translate_piece(0, 1),
            Command::RotateRight => self.rotate_piece(Command::RotateRight),
            Command::RotateLeft => self.rotate_piece(Command::RotateLeft),
            Command::Cheat(x) => self.cheat(*x),
            _ => true,
        };
        if !succeeded && *command == Command::Down {
            println!("down failed, piece ended");
        }
    }

    fn rotate_piece(&mut self, rotate: Command) -> bool {
        let orientation = self.orientation.rotate(rotate);
        let mask = self.piece.mask(orientation);
        if !self.can_move(mask, self.x, self.y) { return false; }
        self.draw_moved_piece(0, 0, orientation);
        true
    }
    
    fn translate_piece(&mut self, dx: i32, dy: i32) -> bool {
        // TODO: check if possible first
        let piece = self.piece;
        let (x, y) = (self.x + dx, self.y + dy);
        let mut mask = piece.mask(self.orientation);
        if !self.can_move(mask, x, y) { return false; }
        self.draw_moved_piece(dx, dy, self.orientation);
        true
    }

    // Cheat codes, mainly used for debugging but can be added to "for fun"
    // Initially set 1..8 to select the next piece
    fn cheat(&mut self, x: usize) -> bool {
        self.next_piece = &PIECES[x - 1];
        true
    }
    
    // see note above about different coordinate systems. Here is where they crash together.
    // BITMAP has padding of 2 bits on left, right, and bottom to make sure the mask always
    // is fully contained in the bitmap
    fn can_move(&self, mut mask: u16, x: i32, mut y: i32) -> bool {
        let mut row: usize = (y + 2) as usize;
        for i in 0..4 {
            let row_bits: u16 = ((self.bitmap[row] >> x + 2) & 0xf) as u16;
            if row_bits & mask != 0 { return false; }
            mask >>= 4;
            row += 1;
        }
        true
    }

    // Moves a piece to a new position. The new position should have already been checked, this
    // assumes the move can be made
    fn draw_moved_piece(&mut self, dx:i32, dy: i32, o1: Orientation) {
        let piece = self.piece;
        let (x0, y0) = (self.x, self.y);
        let (x1, y1) = (x0 + dx, y0 + dy);
        // clear all cells which do not remain on
        let big_mask0 = piece.big_mask(self.orientation, 0, 0);
        let big_mask1 = piece.big_mask(o1, dx, dy);
        let mut clear_mask = big_mask0 & !big_mask1;
        let empty = "empty".to_string();
        let mut i = 0;
        while clear_mask != 0 {
            if clear_mask & 1 == 1 {
                let row = i / 6;
                let col = i % 6;
                self.set_cell(x0 + col - 1, y0 + row, &empty);
            }
            i += 1;
            clear_mask >>= 1;
        }
        
        let name = piece.name.to_string();
        let big_mask0 = piece.big_mask(self.orientation, -dx, 0);
        let big_mask1 = piece.big_mask(o1, 0, dy);
        let mut set_mask = big_mask1 & !big_mask0;
        i = 0;
        while set_mask != 0 {
            if set_mask & 1 == 1 {
                let row = i / 6;
                let col = i % 6;
                self.set_cell(x1 + col - 1, y1 + row - dy, &name);
            }
            i += 1;
            set_mask >>= 1;
        }
        
        (self.x, self.y) = (x1, y1);
        self.orientation = o1;
    }

    // lowest level functions
    // Drawing the pieces is done by setting css class for widgets in the board.grid object. This way I don't need
    // to handle any redrawing, and setting the colors is simple. If drawing turns out to be better then board.grid
    // needs to be replaced with a drawing area and these functions need to be redone
    pub fn cell_at(&self, x: i32, y: i32) -> Option<gtk::Widget> {
        self.grid.child_at(self.width - 1 - x, y)
    }

    pub fn set_cell(&self, x: i32, y: i32, piece_name: &String) {
        match self.cell_at(x, y) {
            Some(cell) => if piece_name.eq("empty") {cell.set_css_classes(&["cell"])} else {cell.set_css_classes(&["cell", piece_name])},
            None => (),
        };
    }
}

