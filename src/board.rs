#![allow(unused)]
use crate::Config;
use crate::controller::State;
use crate::Controller;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::fmt;

use fastrand;
use gdk4::ModifierType;
use gtk::prelude::*;

/* for debugging
use std::any::type_name;
fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}
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
//
// Bitmap
// Each cell on the board os represented by a bit in the bitmap. It is a vec with entries being a u32
// word for each row, one mask per row. There are 32 bits in a mask, but in order to keep pieces from
// going off the board the outside 2 bits on each side and the bottom are set (2 bits because rotating
// the bar can reac 2 cells off the board, and making the border 2 means there is no problem with overflow.
// It does mean the maximum width is 28 rather than 32. So the bitmap looks like this:
//
//  11000...0011
//  11000...0011
//  11000...0011
//  .
//  .
//  .
//  11000...0011
//  11111...1111
//  11111...1111
//
//
// PLAYING:
//
// There are many configuration options you can use to adjust the game. Controls can be set on the command
// or in a config file, which by default is ~/.tetrii, but can be changed with a command switch. The
// command line options can be found by passing the "-h" switch on the command line. (you need to add a
// "--" switch before it to Rust knows the arguments are not its own). Through these you can control
// the dimensions, size and number of game boards. The look can also be adjusted by editing or replacing
// the style.css file in the run directory. This file is needed, the program will not run without it.
//
//
// RULES:
//
// Initial drop time is .5 seconds, every 10 rows completed this is multiplied by 0.9. Pieces score points
// based on their shape and orientation, as set up in the Piece struct. There is a 1 point per layer bonus
// for dropping, and a bonus of 5*(number of levels filled) when levels are filled. (that is, one level is
// 5 points, 2 is 20, 3 45, 4 80.
//
//
// TODO:
//
// This is not finished. It needs to have a main scoreboard to hold the combined scores and have buttons
// to start, stop, pause, and quit. THe purpose of the program is for me to learn about Rust, and for that
// it has already been successful. One of the things I learned was that the overall design is wrong - in
// other languages I've written this in running separate windows worked fine, in Rust it seems to make more
// sense to put them in one big window. I also learned that keeping track of integer types is a problem:
// it is not good to choose whatever seems best for each and then expect them to combine. I have a lot of
// forced casts which are (I think) all OK, but not particularly clean.
//
// The point of that is that I am probably going to rework this with what I've learned to make it a better
// Rust program, and there is no need to finish all the bells and whistles before starting that.
//
// That said, there are a bunch of features that could be added:
// - first the main control panel, as described above.
// - custom control keys: I'd like to add an interactive way to do it, but at least I want to add it to
//   the config file
// - modifiers for mouse clicks: again, on other platforms being able to use ctrl and shift with the mouse
//   seemed the most flexible way to play. I have not found how to do that here. (it will be easier if there
//   is just one window)
// - Again for mouse control, being able to bind 2 commands to a keystroke is helpful - it lets a click
//   select a new window and perform a command there. This way each hand could work a different board
//   without needing to switch them.
// - Features from other versions: adding some extra pieces with different shapes (even ones that morph as
//   they are rotated, or require a 720 degree rotation to return to the original state), or starting the
//   game with random cells filled in and you win by cleaning them all up. But those depend on how much
//   time I care to spend.
//
//////////////////////////////////////////////////////////////////

// initial tick time for each board
const INITIAL_TICK_MSEC: u64 = 500;
// factor to use to increase speed as the game progresses
const TICK_SPEEDUP_RATIO: f64 = 0.9;
// ratio between the clock tick rate and the drop clock tick rate
const DROP_DELAY_SPEEDUP: u64 = 8;

// bits in the BOARD.SUBSTATE mask
const SS_STARTED:   u16 = 0x1;
const SS_PAUSED:    u16 = 0x2;
const SS_OVER:      u16 = 0x4;
const SS_DROPPING:  u16 = 0x8;     // a piece is dropping
const SS_NEW_PIECE: u16 = 0x10;     // the piece being drawn is new, it has no cells previously set
const SS_PREVIEW:   u16 = 0x20;    // the PREVIEW option is on
const SS_FASTER:    u16 = 0x40;    // The tick time has been changed
// default commands to set up the map. The CHEAT entries can have ad hoc stuff added, the current values
// set the next piece, which is useful for debugging (and also for getting out of tight spots)
// TODO: allow custom configurations in the config file
const COMMANDS:[(&str, Command); 25] =
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
     (&"1",      Command::Cheat(0)),   // force piece
     (&"2",      Command::Cheat(1)),
     (&"3",      Command::Cheat(2)),
     (&"4",      Command::Cheat(3)),
     (&"5",      Command::Cheat(4)),
     (&"6",      Command::Cheat(5)),
     (&"7",      Command::Cheat(6)),
     (&"b-Ctrl", Command::Cheat(10)),  // print bitmap binary, good for viewing
     (&"b-Shift", Command::Cheat(11)), // print bitmap hex, can paste into BITARRAY for debugging
     (&"d-Ctrl", Command::Cheat(12)),  // print Board
     (&"p-Ctrl", Command::Cheat(13)),  // use fake bitmap: insert bitmap at BITARRAY and recompile
     (&"s-Ctrl", Command::Cheat(14)),  // print board substatus
];

// builds the command hash from the array definition
fn init_command_hash() -> HashMap<String, Command> {
    let mut command_hash: HashMap<String, Command> = HashMap::new();
    COMMANDS.iter().for_each(|desc| { command_hash.insert(desc.0.to_string(), desc.1); });
    command_hash
}
    
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
    points: [u32; 4],
    masks: [u16; 4],
}

#[derive(Clone)]
pub struct Board {
    // immutable
    env_rc:        Rc<RefCell<Controller>>,
    num:           usize,      // for ID purpose
    width:         i32,
    height:        i32,
    window:        gtk::Window,
    playing_area:  gtk::Grid,
    preview:       gtk::Grid,
    score_widgets: (gtk::Label, gtk::Label),
    command_hash:  HashMap<String, Command>,
    // Following are mutable. I asked on rust-lang and they suggested the approach of making the whole struct mutable
    substate:      u16,            // see SS_* consts defined above
    xy:            (i32, i32),
    orientation:   Orientation,
    piece:         (&'static Piece, &'static Piece),  // (current piece, next piece)
    score:         (u32, u32),     // points and levels
    piece_count:   [u32; 7],
    delay:         u64,            // initial msec between ticks
    bitmap:        Vec<u32>,       // bitmap of board
}

impl Command {
    // which commands are enabled in PAUSED state
    fn always(&self) -> bool {
        match self {
            Command::TogglePause => true,
            Command::Resume => true,
            Command::Cheat(_) => true,
            _ => false
        }
    }
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

impl Piece {
    fn points(&self, orientation: Orientation) -> u32 {
        self.points[orientation.offset()]
    }
    // MASK is a u16 value interpreted as 4 lines of length 4 bits. This can encode all rotations of the pieces.
    fn mask(&self, orientation: Orientation) -> u16 {
        self.masks[orientation.offset()]
    }
    // BIG_MASK is a 5x6 array, see intro above
    fn big_mask(&self, orientation: Orientation, dx: i32, dy: i32) -> u32 {
        let mut big_mask: u32 = 0x0;
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
//    fn env(&self) -> &Controller { &self.env_rc.borrow() }

    pub fn new_ref(num: usize, env_rc: Rc<RefCell<Controller>>) -> Rc<RefCell<Board>> {
        let env_clone  = Rc::clone(&env_rc);
        let env2 = env_clone.borrow();
        let app: &gtk::Application = &env2.app_rc();
        let mut board = Board{
            env_rc: env_rc,
            num: num,
            width: env2.prop_u16("width") as i32,
            height: env2.prop_u16("height") as i32,
            window: gtk::ApplicationWindow::new(app).into(),
            playing_area: gtk::Grid::builder().row_homogeneous(true).column_homogeneous(true).build(),
            preview: gtk::Grid::builder().build(),
            score_widgets: (gtk::Label::builder().label("0").build(), gtk::Label::builder().label("0").build()),
            command_hash: init_command_hash(),
            bitmap: vec![0xffffffff; (env2.prop_u16("height") + 4) as usize],
            substate: SS_PAUSED,
            delay: INITIAL_TICK_MSEC,
            xy: (0, 0),
            orientation: Orientation::North,
            score: (0, 0),
            piece_count: [0; 7],
            piece: (&PIECES[0], Piece::random()),     // initial piece is discarded
        };
        board.window.set_title(Some(&["Board ", &board.num.to_string()].concat()));
        let mut container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        if env2.prop_bool("preview") {
            container.append(board.make_preview());
        }
        container.append(board.make_playing_area());   // return type is different from the others because Grid already exists
        container.append(&board.make_scoreboard());
        board.window.set_child(Some(&container));
        Board::add_handlers(board)
    }

    pub fn show(&self) { self.window.show(); }

    fn make_preview(&mut self) -> &gtk::Grid {
        self.substate |= SS_PREVIEW;
        self.preview.set_halign(gtk::Align::Center);
        self.preview.add_css_class("preview");
        for i in 0..8 {   // preview pane is 4x2
            self.preview.attach(&self.make_cell(), i%4, i/4, 1, 1);
        }
        &self.preview
    }
    
    fn make_scoreboard(&mut self) -> gtk::Box {
        let mut scoreboard = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).build();
        scoreboard.append(&gtk::Label::builder().label("Score: ").build());
        scoreboard.append(&self.score_widgets.0);
        scoreboard.append(&gtk::Label::builder().label("   Levels: ").build());
        scoreboard.append(&self.score_widgets.1);
        scoreboard.add_css_class("scoreboard");
        scoreboard
    }

    fn make_playing_area(&mut self) -> &gtk::Grid {
        self.playing_area.set_focusable(true);
        self.playing_area.add_css_class("board");

        // bitmap is initialized as all 1s, clear all bits representing the playing area
        let mask = !(((0x1 << self.width) - 1) << 2);
        for i in 0..self.bitmap.len() - 2 {
            self.bitmap[i] &= mask;
        }
        for x in 0..self.width {
            for y in 0..self.height {
                self.playing_area.attach(&self.make_cell(), x, y, 1, 1);
            }
        }
        self.start_new_piece(true);
        &self.playing_area
    }        

    // helper function to make a single cell
    fn make_cell(&self) -> gtk::Box {
        let cell = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let label = gtk::Label::builder()
            .label("")
            .build();
        label.add_css_class("cell");
        cell.append(&label);
        cell
    }
    
    fn add_handlers(board: Board) -> Rc<RefCell<Board>> {
        // add handlers
        let board_ref = RefCell::new(board);
        let board_rc = Rc::new(board_ref);
        let key_handler = gtk::EventControllerKey::new();
        board_rc.borrow().playing_area.add_controller(&key_handler);
        let board_key_rc = Rc::clone(&board_rc);
        key_handler.connect_key_pressed(move |_ctlr, key, _code, state| {
            board_key_rc.borrow_mut().keyboard_input(key, state);
            gtk::Inhibit(false)
        });
        /*
        let focus_handler = gtk::EventControllerFocus::new();
        let board_focus_rc = Rc::clone(&board_rc);
        board_rc.borrow().playing_area.add_controller(&focus_handler);
        focus_handler.connect_contains_focus_notify(move |event| {
            // this breaks if borrow_mut(), but fortunately mutability is not needed
            board_focus_rc.borrow().focus_change(event.contains_focus());
        });
         */
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
        let board_click1_rc = Rc::clone(&board_rc);
        board_rc.borrow().playing_area.add_controller(&mouse_handler1);
        mouse_handler1.connect_pressed(move |_, _, _, _ | {
            board_click1_rc.borrow_mut().click_input("Mouse1");
        });
        let mouse_handler2 = gtk::GestureClick::builder().button(2).build();
        let board_click2_rc = Rc::clone(&board_rc);
        board_rc.borrow().playing_area.add_controller(&mouse_handler2);
        mouse_handler2.connect_pressed(move |_, _, _, _ | {
            board_click2_rc.borrow_mut().click_input("Mouse2");
        });
        let mouse_handler3 = gtk::GestureClick::builder().button(3).build();
        let board_click3_rc = Rc::clone(&board_rc);
        board_rc.borrow().playing_area.add_controller(&mouse_handler3);
        mouse_handler3.connect_pressed(move |_, _, _, _ | {
            board_click3_rc.borrow_mut().click_input("Mouse3");
        });
        board_rc
    }

    fn update_score(&self) {
        self.score_widgets.0.set_text(&self.score.0.to_string());
        self.score_widgets.1.set_text(&self.score.1.to_string());
    }
    
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
            self.add_piece_to_bitmap();
            self.score.0 += self.piece.0.points(self.orientation) + self.completed_lines();
            self.update_score();
        }
        self.piece.0 = self.piece.1;
        self.piece.1 = Piece::random();

        self.substate |= SS_NEW_PIECE;
        if self.substate & SS_PREVIEW > 0 {
            self.draw_preview();
        }
        self.orientation = Orientation::North;
        self.xy = (self.width/2 - 2, -1);
        if !self.can_move(self.piece.0.mask(self.orientation), self.xy) {
            self.to_controller(State::Finished);
            return false;
        }
        self.piece_count[self.piece.0.pos] += 1;
        self.draw_moved_piece(0, 0, self.orientation);
        true
    }

    fn completed_lines(&mut self) -> u32 {
        let mut cleared_lines = 0;
        let len = self.bitmap.len();
        let mut i = self.bitmap.len() - 3;   // start from the bottom of the board (skip over 2 rows of set bits)
        while i >= 2 {                       // and ignore the extra 2 lines above the visible board
            if self.bitmap[i] == 0xffffffff {
                self.remove_row((i - 2) as i32);  // change from bitmap coords to board coords
                cleared_lines += 1;
            } else {
                i -= 1;
            }
        }
        self.score.1 += cleared_lines;
        // completion bonus: 5 times completed lines squared (max of 100 pts)
        cleared_lines*cleared_lines*5
    }
        
    fn remove_row(&mut self, row: i32) {
        // move down all cells above this row
        for y in (0..row + 1).rev() {
            for x in 0..self.width {
                let lc = self.get_cell_color(x, y);
                let upper_color = self.get_cell_color(x, y - 1);
                if upper_color != self.get_cell_color(x, y) {
                    self.set_cell_color(x, y, &upper_color);
                }
            }
        }
        // now update the bitmap by removing the completed row and adding an empty one on top
        self.bitmap.remove((row + 2) as usize);
        let new_row_mask:u32 = 0xffffffff & !(((1 << self.width) - 1) << 2);  // mask is -1 with the bits representing the playing area cleared
        self.bitmap.insert(2, new_row_mask);
        
    }
    // Moves a piece to a new position. The new position should have already been checked, this
    // assumes the move can be made
    fn draw_moved_piece(&mut self, dx:i32, dy: i32, o1: Orientation) {
        let piece = self.piece.0;
        let (x0, y0) = self.xy;
        let (x1, y1) = (x0 + dx, y0 + dy);

        // set all cells which were off but will be on
        let name = piece.name;
        let mut set_mask = piece.big_mask(o1, 0, dy);
        if self.substate & SS_NEW_PIECE == 0 {
            set_mask &= !piece.big_mask(self.orientation, -dx, 0);
        }
        let mut i = 0;
        while set_mask != 0 {
            if set_mask & 1 == 1 {
                let row = i / 6;
                let col = i % 6;
                self.set_cell_color(x1 + col - 1, y1 + row - dy, name);
            }
            i += 1;
            set_mask >>= 1;
        }
        
        // clear all cells which do not remain on. If this is a new piece there are no cells to be cleared.
        if self.substate & SS_NEW_PIECE != 0 {
            self.substate &= !SS_NEW_PIECE;
        } else {
            let mut clear_mask = piece.big_mask(self.orientation, 0, 0) & !piece.big_mask(o1, dx, dy);
            let mut i = 0;
            while clear_mask != 0 {
                if clear_mask & 1 == 1 {
                    let row = i / 6;
                    let col = i % 6;
                    self.set_cell_color(x0 + col - 1, y0 + row, "empty");
                }
                i += 1;
                clear_mask >>= 1;
            }
        }
        self.xy = (x1, y1);
        self.orientation = o1;
    }

    // lowest level functions
    // Drawing the pieces is done by setting css class for widgets in the board.playing_area object. This way I don't need
    // to handle any redrawing, and setting the colors is simple. If drawing turns out to be better then board.playing_area
    // needs to be replaced with a drawing area and these functions need to be redone
    fn cell_at(&self, x: i32, y: i32) -> Option<gtk::Widget> {
        self.playing_area.child_at(self.width - 1 - x, y)
    }

    fn set_cell_color(&self, x: i32, y: i32, piece_name: &str) {
        match self.cell_at(x, y) {
            Some(cell) => cell.set_css_classes(&["cell", piece_name]),
            None => (),    // if the cell is off the visible board just don't draw it
        };
    }

    // all cells have class Cell, any with contents also have the class name of the piece. Currently I am not using the class
    // Empty, but there is no harm in returning it - maybe it can be used in the future.
    fn get_cell_color(&self, x: i32, y: i32) -> String {
        let cell_opt = self.cell_at(x, y);
        if cell_opt == None { return "empty".to_string(); }
        let classes = cell_opt.unwrap().css_classes();
        if classes.len() < 2 {"empty".to_string()}
        else if classes[0] == "cell" {classes[1].to_string()}
        else {classes[0].to_string()}
    }

    // All pieces in their North position are contained in rows 1 and 2, so the mask is of the form 0x0**0 and only the
    // middle 2 quartets are drawn
    fn draw_preview(&self) {
        let mut mask = self.piece.1.mask(Orientation::North) >> 4;
        let piece_type = &[self.piece.1.name];
        let empty = &["empty"];
        for i in 0..8 {
            self.preview.child_at(3 - i%4, i/4).unwrap().set_css_classes( if mask & 1 > 0 {piece_type} else {empty});
            mask >>= 1;
        }
    }

    //////////////////////////////////////////////////////////////////
    //
    // Command implementations
    //
    //////////////////////////////////////////////////////////////////
    fn do_command(&mut self, command: &Command) -> bool {
        if self.substate & SS_OVER > 0 { return false; }
        if self.substate & SS_PAUSED == 0 || command.always() {
            match command {
                Command::Left        => self.translate_piece(1, 0),
                Command::Right       => self.translate_piece(-1, 0),
                Command::Down        => self.translate_piece(0, 1),
                Command::Drop        => self.do_drop(),
                Command::RotateRight => self.rotate_piece(Command::RotateRight),
                Command::RotateLeft  => self.rotate_piece(Command::RotateLeft),
                Command::Cheat(x)    => self.cheat(*x),
                Command::Resume      => self.to_controller(State::Running),
                Command::Pause       => self.to_controller(State::Paused),
                Command::TogglePause => self.to_controller(if self.substate & SS_PAUSED > 0 { State::Running } else { State::Paused }),
                _ => true,
            }
        } else { true }
    }

    fn click_input(&mut self, button: &str) {
        let command = *self.command_hash.get(button).unwrap_or(&Command::Nop);
        // not sure if I can get modifer keys here
        self.do_command(&command);
    }

    fn focus_change(&self, focus_in: bool) {
        if focus_in { self.playing_area.add_css_class("selected"); }
        else { self.playing_area.remove_css_class("selected"); }
    }

    fn keyboard_input(&mut self, key: gdk4::Key, modifiers: ModifierType) {
        let key_string = modifier_string(key.to_lower().name().unwrap().to_string(), modifiers.bits());
        let command = *self.command_hash.get(&key_string).unwrap_or(&Command::Nop);
        self.do_command(&command);
    }

    fn to_controller(&self, new_state: State) -> bool { self.env_rc.borrow_mut().set_state(new_state); true }
/*
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
*/    
    fn rotate_piece(&mut self, rotate: Command) -> bool {
        let orientation = self.orientation.rotate(rotate);
        let mask = self.piece.0.mask(orientation);
        if !self.can_move(mask, self.xy) { return false; }
        self.draw_moved_piece(0, 0, orientation);
        true
    }
    
    fn translate_piece(&mut self, dx: i32, dy: i32) -> bool {
        let (x, y) = (self.xy.0 + dx, self.xy.1 + dy);
        let mask = self.piece.0.mask(self.orientation);
        if !self.can_move(mask, (x, y)) { return false; }
        self.draw_moved_piece(dx, dy, self.orientation);
        true
    }

    fn do_drop(&mut self) -> bool {
        if self.substate & SS_DROPPING == 0 && self.do_command(&Command::Down) {
            self.substate |= SS_DROPPING;
            self.drop_tick();
        }
        true
    }
    
    // Cheat codes, mainly used for debugging but can be added to "for fun"
    fn cheat(&mut self, cheat: usize) -> bool {
        match cheat {
            // 0..8 to select the next piece
            0..= 7 => self.piece.1 = &PIECES[cheat],
            // print out bitmap in binary
            10 => self.bitmap.iter().for_each(|x| { println!("{:032b}", x); }),
            // print out bitmap in hex (to be used with BITARRAY)
            11 => self.bitmap.iter().for_each(|x| { println!("0X{:X},", x); }),
            // print the whols Board struct
            12 => println!("{:?}", self),
            // use the bitmap in BITARRAY to replace the current array (used for debugging)
            13 => self.init_bitmap_to(&BITARRAY),
            14 => println!("substate for board {}: {}", self.num, substate_to_str(self.substate)),
            _ => println!("unrecognized cheat code"),
        }
        true
    }


    pub fn change_state(&mut self, new_state: State) -> bool {
        if self.substate & SS_OVER > 0 { return false; }
        let mut ret = true;
        match new_state {
            State::Running => {if self.substate & SS_PAUSED > 0 {self.substate &= !SS_PAUSED;  self.tick();} true},
            State::Finished => {self.substate |= SS_OVER; false},
            State::Paused => { self.substate |= SS_PAUSED; true },
            State::Paused => { self.substate |= SS_PAUSED; true },
            State::Setup => panic!("Boards should not be built during SETUP state"),
        }
    }

    // see note above about different coordinate systems. Here is where they crash together.
    // BITMAP has padding of 2 bits on left, right, and bottom to make sure the mask always
    // is fully contained in the bitmap
    fn can_move(&self, mut mask: u16, xy: (i32, i32)) -> bool {
        let (x, y) = xy;
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
        let mut mask = self.piece.0.mask(self.orientation) as u32;
        let mut row = self.xy.1 as usize;
        while mask != 0 {
            self.bitmap[row + 2] |= (mask & 0xf) << (self.xy.0 + 2);
            mask >>= 4;
            row += 1;
        }
    }
    
    fn tick(&self) {
        let msec = self.delay;
        let p_env = self.env_rc.borrow();
        let p_board = p_env.board_ref(self.num - 1);    //&BOARDS[self.num - 1];
        let rc_board = Rc::clone(p_board);
        unsafe {
            let f = move || -> glib::Continue {
                let mut mut_board = rc_board.borrow_mut(); 
                glib::Continue(
                    if mut_board.substate & SS_DROPPING != 0 { true }                      // continue the timer, but don't move the piece
                    else if mut_board.substate & (SS_PAUSED | SS_OVER) > 0 { false } // stop if paused or finished
                    else {
                        mut_board.do_command(&Command::Down) ||         // move down if possible...
                            mut_board.start_new_piece(false)            // ... or get new piece
                    })
            };
            glib::timeout_add_local(core::time::Duration::from_millis(msec), f);
        }
    }

    fn drop_tick(&self) {
        let msec = self.delay/DROP_DELAY_SPEEDUP;
        let p_env = self.env_rc.borrow();
        let p_board = p_env.board_ref(self.num - 1);    //&BOARDS[self.num - 1];
        let rc_board = Rc::clone(p_board);
        unsafe {
            let f = move || -> glib::Continue {
                let mut mut_board = rc_board.borrow_mut();
                mut_board.score.0 += 1;     // drop bonus
                let mut success = true;
                if !mut_board.do_command(&Command::Down) {
                    mut_board.substate &= !SS_DROPPING;
                    mut_board.start_new_piece(false);
                    success = false;
                }
                glib::Continue(success)
            };
            glib::timeout_add_local(core::time::Duration::from_millis(msec), f);
        }
    }

    // debugging function: set BITARRAY to reconstruct position
    fn init_bitmap_to(&mut self, array: &[u32]) {
        self.bitmap = array.to_vec();
        for y in 2..self.height + 2 {
            let mut bit = 0x1u32 << 2;     // skip the first 2 edge bits
            for x in 0..self.width {
                if bit & self.bitmap[y as usize] != 0 {
                    self.set_cell_color(x, y - 2, &PIECES[(y as usize)%PIECES.len()].name);   // rows will be the same color
                }
                bit <<= 1;
            }
        }
    }
}

const SUBSTATE_NAMES:[&str; 7] = ["Started", "Paused", "Over", "Dropping", "New_piece", "Preview", "Faster"];
fn substate_to_str(mut mask: u16) -> String {
    let mut states: Vec<&str> = Vec::new();
    let mut i = 0;
    while mask != 0 {
        if mask & 1 > 0 { states.push(&SUBSTATE_NAMES[i]); }
        i += 1;
        mask >>= 1;
    }
    states.join(", ")
}
    
// derive doesn't work for raw structs like Rc and Window
impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex_substate = format!("{:x}", self.substate);
        let lines: Vec<String> = self.bitmap.iter().map(|row| { let str = format!("0X{:X},", row); str }).collect();
        let bitmap = lines.join("\n");
            
        f.debug_struct("Point")
            .field("env.rc", &"Rc<Controller>")
            .field("num", &self.num)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("window", &"gtk::Window")
            .field("playing_area", &"gtk::Grid")
            .field("preview", &"gtk::Grid")
            .field("command_hash", &"HashTable")
//            .field("state", &self.state)
            .field("substate", &substate_to_str(self.substate))
            .field("xy", &self.xy)
            .field("orientation", &self.orientation)
            .field("piece", &format!("({}, {})", self.piece.0.name, self.piece.1.name))
            .field("score", &self.score)
            .field("piece_count", &self.piece_count)
            .field("delay", &self.delay)
            .field("bitmap", &bitmap)
            .finish()
    }
}

// set this up and use with init_bitmap_to() for debugging special cases (get data from cheat 11)
const BITARRAY: [u32; 24] = [
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFF003,
    0xFFFFFFF7,
    0xFFFFFFF7,
    0xFFFFFFF7,
    0xFFFFFFF7,
    0xFFFFFFFF,
    0xFFFFFFFF,
];
// add modifier suffixes to the key
fn modifier_string(mut key: String, bits: u32) -> String {
    if bits & ModifierType::SHIFT_MASK.bits() != 0 { key.push_str("-Shift"); }
    if bits & ModifierType::ALT_MASK.bits() != 0 { key.push_str("-Alt"); }
    if bits & ModifierType::CONTROL_MASK.bits() != 0 { key.push_str("-Ctrl"); }
    if bits & ModifierType::META_MASK.bits() != 0 { key.push_str("-Meta"); }
    key
}


