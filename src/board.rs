#![allow(unused)]
use crate::Config;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

use fastrand;
use gdk4::ModifierType;
use gtk::prelude::*;

/* for debugging
use std::any::type_name;
fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}
 */

/* TODO:
 * - timer for drop
 * - add score to boards
 * - modifiers for mouse click?
 * - 2 (multiple?) commands per key
 * - add preview to boards
 * - make main window (overall score, control buttons)
 * - finish implementng commands
 * - custom key assignments in yaml file
 * - animation
 */

//////////////////////////////////////////////////////////////////
//
// Coordinate systems:
//
// There are 3 coordinate systems in use in the program, used at different layers. The most useful is
// the logical board. This is a conceptual bitmap LENGTHxWIDTH with the origin at the upper right.
// This is what the logic uses through most of the program.
//
// The second system is the actual bitmap (board.bitmap). This contains the logical board but is bigger:
// it contains a border of 2 bits on all sides so that I don't have to worry about overruns. It too has
// the origin at th eupper right, but the origin of the logical board is located at (2, 2) in this
// system.
//
// Finally, there is the Grid widget used for display. It too is LENGTHxWIDTH but its origin is at
// the upper left, not right, so the X coordinate is flipped from the board's
//
//
// Masks
// 
// the shapes are stored in a 4x4 grid. In each case the initial orientation leaves the top row
// empty, so the initial placement is at (width/2 - 2, -1). Rotation can use any of the 16 cells.
// The masks are designed so the first row (last hex digit) is always blank and the firstpiece starts
// in a horizontal as close to centered as possible, or towards the right if uneven (as most are).
// This way each piece can be drawn in its initial position by centering the X coord and setting the
// Y coord to -1. The masks are used to check if a piece can fit in a new position and to draw it if
// it does fit.
//
// There is a second mask used also, called big_mask. This is logically a 5x6 grid with the 4x4 mask
// embedded in it:
//
//    . x x x x .     
//    . x x x x .
//    . x x x x .
//    . x x x x .
//    . . . . . .
//
// This way when a cell is moved one unit in any of the 3 directions (LEFT, DOWN, RIGHT) the bigmask
// of the old position can be ANDed with the bigmask of the new position to find the cells which need
// to be redrawn.
//
//////////////////////////////////////////////////////////////////

// ratio between the clock tick rate and the drop clock tick rate
const DROP_DELAY_SPEEDUP: u64 = 8;

// default commands to set up the map. The CHEAT entries can have ad hoc stuff added, the current values
// set the next piece, which is useful for debugging (and also for getting out of tight spots)
// TODO: allow custom configurations in the config file
const COMMANDS:[(&str, Command); 20] =
    [(&"Right",  Command::Right),
     (&"Left",   Command::Left),
     (&"Down",   Command::Down),
     (&"q",      Command::RotateLeft),
     (&"q-Shift",Command::Left),
     (&"e",      Command::RotateRight),
     (&"space",  Command::Drop),
     (&"s",      Command::Resume),
     (&"t",      Command::TogglePause),
     (&"p",      Command::Pause),
     (&"Mouse1", Command::Left),
     (&"Mouse2", Command::Down),
     (&"Mouse3", Command::Right),
     (&"1",      Command::Cheat(1)),
     (&"2",      Command::Cheat(2)),
     (&"3",      Command::Cheat(3)),
     (&"4",      Command::Cheat(4)),
     (&"5",      Command::Cheat(5)),
     (&"6",      Command::Cheat(6)),
     (&"7",      Command::Cheat(7)),
];

// builds the command hash from the array definition
fn init_command_hash() -> HashMap<String, Command> {
    let mut command_hash: HashMap<String, Command> = HashMap::new();
    COMMANDS.iter().for_each(|desc| { command_hash.insert(desc.0.to_string(), desc.1); });
    command_hash
}
    
// I know this is frowned on. The problem is that I need to be able to signal boards from within callbacks,
// from keys, mouse clicks, and timer events. Those all require 'static lifetime, and I couldn't find any
// way to "fool the compiler" to make it work. Is there a better way?
pub static mut BOARDS: Vec<Rc<RefCell<Board>>> = Vec::new();

static PIECES: [Piece; 7] = [
    Piece {name: &"Bar",        points: [12, 1, 12, 1, ], masks: [0x00f0, 0x2222, 0x00f0, 0x2222, ], pos: 0, },
    Piece {name: &"Tee",        points: [ 6, 5,  2, 1, ], masks: [0x0270, 0x0232, 0x0072, 0x0262, ], pos: 1, },
    Piece {name: &"Square",     points: [ 4, 4,  4, 4, ], masks: [0x0660, 0x0660, 0x0660, 0x0660, ], pos: 2, },
    Piece {name: &"Zee",        points: [ 5, 3,  5, 3, ], masks: [0x0360, 0x0462, 0x0360, 0x0462, ], pos: 3, },
    Piece {name: &"ReverseZee", points: [ 5, 3,  5, 3, ], masks: [0x0630, 0x0264, 0x0630, 0x0264, ], pos: 4, },
    Piece {name: &"El",         points: [ 6, 6,  3, 3, ], masks: [0x0470, 0x0322, 0x0071, 0x0226, ], pos: 5, },
    Piece {name: &"ReverseEl",  points: [ 3, 3,  6, 6, ], masks: [0x0740, 0x2230, 0x0170, 0x0622, ], pos: 6, },
];

// if I have time and interest the commands will be configurable through the .tetrii file
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Command {Left, Right, Down, RotateRight, RotateLeft, Drop, Pause, Resume, TogglePause, SetBoard(i32), Cheat(usize), Nop, }

#[derive(Copy, Clone, Debug)]
pub enum Orientation {North, East, South, West, }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State { Paused, Running, Finished, }

#[derive(Debug)]
pub struct Piece {
    // NAME is used to identify the piece. It also is the name of the CSS class used to draw the piece.
    name: &'static str,
    pos: usize,
    // These arrays give the values for each piece. There are 4 for each - some pieces need fewer (BAR
    // needs 2, SQUARE needs 1), but rather than deal with different length vectors it is simpler just
    // to repeat the values until there are 4.
    // (NOTE: other implementations have used circular linked lists to manage this. Listing 4 rotations for
    // each object probably takes less code than handling the different cases individually)
    points: [u8; 4],
    masks: [u16; 4],
}

#[derive(Debug, Clone)]
pub struct Board {
    // immutable
    num:          usize,      // for ID purpose
    width:        i32,
    height:       i32,
    window:       gtk::Window,
    playing_area: gtk::Grid,       
    command_hash: HashMap<String, Command>,
    // Following are mutable. I asked on rust-lang and they suggested the approach of making the whole struct mutable
    state:        State,
    dropping:     bool,           // flag set for when piece is dropping. It could be made a substate of Running, but this is simpler
    x:            i32,
    y:            i32,
    orientation:  Orientation,
    score:        u32,
    piece_count:  [u32; 7],
    piece:        &'static Piece,
    next_piece:   &'static Piece,
    delay:        u64,            // initial msec between ticks
    bitmap:       Vec<u32>,       // bitmap of board
}


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

impl State {
    fn toggle(&self) -> State {
        match self {
            State::Finished => State::Finished,
            State::Paused => State::Running,
            State::Running => State::Paused,
        }
    }
}

impl Piece {
    fn points(&self, orientation: Orientation) -> u8 {
        self.points[orientation.offset()]
    }
    // MASK is a u16 value interpreted as 4 lines of length 4 bits. This can encode all rotations of the pieces.
    fn mask(&self, orientation: Orientation) -> u16 {
        self.masks[orientation.offset()]
    }
    // BIG_MASKis a 5x6 array (fitting in 32 bits) used to map a piece and the rows to its left, right, and bottom
    fn big_mask(&self, orientation: Orientation, dx: i32, dy: i32) -> u32 {
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
        &PIECES[fastrand::usize(0..PIECES.len())]
    }
}

impl Board {
    pub fn new(num: usize, app: &gtk::Application, config: &Config) -> Rc<RefCell<Board>> {
        let mut container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let mut board = Board{num: num,
                              width: config.width as i32,
                              height: config.height as i32,
                              window: gtk::ApplicationWindow::new(app).into(),
                              playing_area: gtk::Grid::builder().build(),
                              command_hash: init_command_hash(),
                              bitmap: vec![0xffffffff; (config.height + 4) as usize],
                              state: State::Paused,
                              dropping: false,
                              delay: 500,
                              x: 0,
                              y: 0,
                              orientation: Orientation::North,
                              score: 0,
                              piece_count: [0; 7],
                              piece: &PIECES[0],     // initial piece is discarded
                              next_piece: Piece::random(),
        };
        if config.preview {
            container.append(&board.make_preview(config));
        }
        container.append(board.make_playing_area(config));   // return type is different from the others because Grid already exists
        container.append(&board.make_scoreboard(config));
        board.window.set_child(Some(&container));
        Board::add_handlers(board)
    }

    fn add_handlers(board: Board) -> Rc<RefCell<Board>> {
        // add handlers
        let ref_board = RefCell::new(board);
        let rc_board = Rc::new(ref_board);
        let key_handler = gtk::EventControllerKey::new();
        rc_board.borrow().playing_area.add_controller(&key_handler);
        let rc_board_key = Rc::clone(&rc_board);
        key_handler.connect_key_pressed(move |_ctlr, key, _code, state| {
            rc_board_key.borrow_mut().keyboard_input(key, state);
            gtk::Inhibit(false)
        });
        let focus_handler = gtk::EventControllerFocus::new();
        let rc_board_focus = Rc::clone(&rc_board);
        rc_board.borrow().playing_area.add_controller(&focus_handler);
        focus_handler.connect_contains_focus_notify(move |event| {
            // this breaks if borrow_mut(), but fortunately mutability is not needed
            rc_board_focus.borrow().focus_change(event.contains_focus());
        });
        /* I'd like to implement this (select on mouse over) but mac doesn't generate the events
        let enter_handler = gtk::EventControllerFocus::new();
        let rc_board_enter = Rc::clone(&rc_board);
        rc_board.borrow().playing_area.add_controller(&enter_handler);
        enter_handler.connect_contains_focus_notify(move |event| {
            println!("enter: {}", event.contains_focus());
            // this breaks if borrow_mut(), but fortunately mutability is not needed
            if event.contains_focus() {
                println!("setting it here");
                rc_board_enter.borrow().window.grab_focus();
            }
        });
         */
        // Do I really need a different handler for each button to know which one was pressed?
        // And is there any way to get modifier keys, short of keeping track of the presses?
        let mouse_handler1 = gtk::GestureClick::builder().button(1).build();
        let rc_board_click1 = Rc::clone(&rc_board);
        rc_board.borrow().playing_area.add_controller(&mouse_handler1);
        mouse_handler1.connect_pressed(move |_, _, _, _ | {
            rc_board_click1.borrow_mut().click_input("Mouse1");
        });
        let mouse_handler2 = gtk::GestureClick::builder().button(2).build();
        let rc_board_click2 = Rc::clone(&rc_board);
        rc_board.borrow().playing_area.add_controller(&mouse_handler2);
        mouse_handler2.connect_pressed(move |_, _, _, _ | {
            rc_board_click2.borrow_mut().click_input("Mouse2");
        });
        let mouse_handler3 = gtk::GestureClick::builder().button(3).build();
        let rc_board_click3 = Rc::clone(&rc_board);
        rc_board.borrow().playing_area.add_controller(&mouse_handler3);
        mouse_handler3.connect_pressed(move |_, _, _, _ | {
            rc_board_click3.borrow_mut().click_input("Mouse3");
        });
        rc_board
    }

    fn make_preview(&mut self, config: &Config) -> gtk::Grid {
        gtk::Grid::builder().build()
    }
    fn make_scoreboard(&mut self, config: &Config) -> gtk::Grid {
        gtk::Grid::builder().build()
    }
    fn make_playing_area(&mut self, config: &Config) -> &gtk::Grid {
        self.playing_area.set_focusable(true);
        self.playing_area.add_css_class("board");

        let mask = !(((0x1 << config.width) - 1) << 2);
        for i in 0..self.bitmap.len() - 2 {
            self.bitmap[i] &= mask;
        }
        // bitmap is a map of the board with 0 for empty spaces and 1 for filled. Initialize it so all bits representing
        // cells of the bitmap are 0 and all other bits are 1. To avoid having to worry about overflow or underflow there
        // is a border of at least 2 set bits on the left, right, and bottom of the bitmap. This means that the maximum
        // allowable width, using a 32-but mask, is 28 columns.
        // 
        for row in 0..self.width {
            for col in 0..self.height {
                let cell = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .build();
                let label = gtk::Label::builder()
                    .label("")
                    .build();
                label.add_css_class("cell");
                cell.append(&label);
                self.playing_area.attach(&cell, row, col, 1, 1);
            }
        }
        self.window.set_title(Some(&["Board ", &self.num.to_string()].concat()));
        if config.initial_piece < PIECES.len() {
            self.next_piece = &PIECES[config.initial_piece];
        }
        self.start_new_piece(true);
        &self.playing_area
    }        
        
    pub fn show(&self) { self.window.show(); }

    //////////////////////////////////////////////////////////////////
    //
    // Piece handling
    //
    //////////////////////////////////////////////////////////////////

    // called to load a new piece in the board. The drawing function might be able to be merged with draw_moved_piece()?
    fn start_new_piece(&mut self, initial: bool) -> bool{
        // The first time through there is no old piece to record on the bitmap. In subsequent calls the old piece
        // needs to be transferred to the bitmap before loading a new one
        if !initial {
            self.add_piece_to_bitmap()
        }
        
        self.piece = self.next_piece;
        self.next_piece = Piece::random();
        self.orientation = Orientation::North;
        (self.x, self.y) = (self.width/2 - 2, -1);
        if !self.can_move(self.piece.mask(self.orientation), self.x, self.y) {
            self.change_state(State::Finished);
            return false;
        }
        self.piece_count[self.piece.pos] += 1;
        let name = self.piece.name.to_string();
        let mut mask = self.piece.mask(self.orientation);
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
        true
    }

    fn click_input(&mut self, button: &str) {
        let command = *self.command_hash.get(button).unwrap_or(&Command::Nop);
        // not sure if I can get modifer keys here
        self.do_command(&command);
    }

    fn focus_change(&self, focus_in: bool) {
        if focus_in { self.playing_area.add_css_class("selected"); }
        else {self.playing_area.remove_css_class("selected");}
    }

    fn keyboard_input(&mut self, key: gdk4::Key, modifiers: ModifierType) {
        let key_string = modifier_string(key.to_lower().name().unwrap().to_string(), modifiers.bits());
        let command = *self.command_hash.get(&key_string).unwrap_or(&Command::Nop);
        self.do_command(&command);
    }

    //////////////////////////////////////////////////////////////////
    //
    // Command implementations
    //
    //////////////////////////////////////////////////////////////////
    fn do_command(&mut self, command: &Command) -> bool {
        if self.state == State::Running || command == &Command::TogglePause || command == &Command::Resume {
            match command {
                Command::Left => self.translate_piece(1, 0),
                Command::Right => self.translate_piece(-1, 0),
                Command::Down => self.translate_piece(0, 1),
                Command::Drop => self.do_drop(),
                Command::RotateRight => self.rotate_piece(Command::RotateRight),
                Command::RotateLeft => self.rotate_piece(Command::RotateLeft),
                Command::Cheat(x) => self.cheat(*x),
                Command::Resume => self.control_all(State::Running),
                Command::Pause => self.control_all(State::Paused),
                Command::TogglePause => self.control_all(self.state.toggle()),
                _ => true,
            }
        } else {
            true
        }
    }

    fn control_all(&mut self, new_state: State, ) -> bool {
        // This accesses all the boards. The problem is the current one is already owned as mut so its pointer in
        // BOARDS cannot be referenced. It is, however, available as SELF, so it gets handled differently.
        // Again, since this all runs in a single thread, there is no danger in accessing the static values.
        unsafe {
            for i in 0..BOARDS.len() {
                if i == self.num - 1 { self.change_state(new_state); }
                else { BOARDS[i].borrow_mut().change_state(new_state); }
            }
        }
        true
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
        let mask = piece.mask(self.orientation);
        if !self.can_move(mask, x, y) { return false; }
        self.draw_moved_piece(dx, dy, self.orientation);
        true
    }

    fn do_drop(&mut self) -> bool {
        if !self.dropping && self.do_command(&Command::Down) {
            self.dropping = true;
            self.drop_tick();
        }
        true
    }
    
    // Cheat codes, mainly used for debugging but can be added to "for fun"
    // Initially set 1..8 to select the next piece
    fn cheat(&mut self, x: usize) -> bool {
        self.next_piece = &PIECES[x - 1];
        true
    }
    
    fn change_state(&mut self, new_state: State) -> bool {
        if self.state == State::Finished || new_state == self.state {return true; }
        self.state = new_state;
        match self.state {
            State::Running => self.tick(),
            State::Finished => (),  // TODO: signal end to all boards
            State::Paused => (),
        }
        true
    }

    // see note above about different coordinate systems. Here is where they crash together.
    // BITMAP has padding of 2 bits on left, right, and bottom to make sure the mask always
    // is fully contained in the bitmap
    fn can_move(&self, mut mask: u16, x: i32, y: i32) -> bool {
        let mut row: usize = (y + 2) as usize;
        while mask != 0 {
            let row_bits: u16 = ((self.bitmap[row] >> x + 2) & 0xf) as u16;
            if row_bits & mask != 0 { return false; }
            mask >>= 4;
            row += 1;
        }
        true
    }

    fn add_piece_to_bitmap(&mut self) {
        let mut mask = self.piece.mask(self.orientation) as u32;
        let mut row = self.y as usize;
        while mask != 0 {
            self.bitmap[row + 2] |= (mask & 0xf) << (self.x + 2);
            mask >>= 4;
            row += 1;
        }
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
    // Drawing the pieces is done by setting css class for widgets in the board.playing_area object. This way I don't need
    // to handle any redrawing, and setting the colors is simple. If drawing turns out to be better then board.playing_area
    // needs to be replaced with a drawing area and these functions need to be redone
    pub fn cell_at(&self, x: i32, y: i32) -> Option<gtk::Widget> {
        self.playing_area.child_at(self.width - 1 - x, y)
    }

    pub fn set_cell(&self, x: i32, y: i32, piece_name: &String) {
        match self.cell_at(x, y) {
            Some(cell) => if piece_name.eq("empty") {cell.set_css_classes(&["cell"])} else {cell.set_css_classes(&["cell", piece_name])},
            None => (),    // if the cell is off the visible board just don't draw it
        };
    }

    fn tick(&self) {
        let msec = self.delay;
        unsafe {
            let p_board = &BOARDS[self.num - 1];
            let f = move || -> glib::Continue {
                let mut mut_board = p_board.borrow_mut(); 
                glib::Continue(
                    if mut_board.dropping { true }                      // continue the timer, but don't move the piece
                    else if mut_board.state != State::Running { false } // stop if paused or finished
                    else {
                        mut_board.do_command(&Command::Down) ||         // move down if possible...
                            mut_board.start_new_piece(false)            // ... or get new piece
                    })
            };
            glib::timeout_add_local(core::time::Duration::from_millis(msec), f);
        }
    }

    fn drop_tick(&self) {
        unsafe {
            let msec = self.delay/DROP_DELAY_SPEEDUP;
            let p_board = &BOARDS[self.num - 1];
            let f = move || -> glib::Continue {
                let mut mut_board = p_board.borrow_mut();
                let mut success = true;
                if !mut_board.do_command(&Command::Down) {
                    mut_board.dropping = false;
                    mut_board.start_new_piece(false);
                    success = false;
                }
                glib::Continue(success)
            };
            glib::timeout_add_local(core::time::Duration::from_millis(msec), f);
        }
    }
}

// add modifier suffixes to the key
fn modifier_string(mut key: String, bits: u32) -> String {
    if bits & ModifierType::SHIFT_MASK.bits() != 0 { key.push_str("-Shift"); }
    if bits & ModifierType::ALT_MASK.bits() != 0 { key.push_str("-Alt"); }
    if bits & ModifierType::CONTROL_MASK.bits() != 0 { key.push_str("-Ctrl"); }
    if bits & ModifierType::META_MASK.bits() != 0 { key.push_str("-Meta"); }
    key
}


