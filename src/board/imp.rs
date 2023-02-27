//use self::glib::{BindingFlags, ParamSpec, ParamSpecInt, Value};
use crate::board;
use fastrand;
use std::cell::RefCell;
use std::cell::Cell;
use std::rc::Rc;
use once_cell::sync::OnceCell;

use gtk::{Root, glib};
use gtk::CompositeTemplate;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib::subclass::Signal;
use gtk::glib::clone;
use gtk::prelude::GridExt;

//
// Boilerplate
//
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "board.ui")]
pub struct Board {
    pub id_oc:           OnceCell<u32>,
    pub width_oc:        OnceCell<u32>,
    pub height_oc:       OnceCell<u32>,
    pub show_preview_oc: OnceCell<bool>,
    internal:            Rc<RefCell<Internal>>,

    #[template_child]
    pub playing_area: TemplateChild<gtk::Grid>,
    #[template_child]
    pub preview: TemplateChild<gtk::Grid>,
    #[template_child]
    pub points: TemplateChild<gtk::Label>,
    #[template_child]
    pub lines: TemplateChild<gtk::Label>,
}

#[derive(Debug, )]
struct Internal {
    piece:        (&'static Piece, &'static Piece),  // (current piece, next piece)
    xy:           (i32, i32),
    orientation:  Orientation,
    score:        (u32, u32),      // points, levels
    piece_counts: [u32; 7],
    bitmap:       Vec<u32>,       // bitmap of board
    state:        u32,            // holds SS_ state bis
	timer:        Timer,
}

const SS_NEW_PIECE: u32 = 0x1;    // the current piece is a new one, draw all squares without checking if they are already on
const SS_PREVIEW:   u32 = 0x2;    // flag to do preview, simpler than getting it from he main structure
const SS_DROPPING:  u32 = 0x4;

const STARTING_TICK_MS: u32 = 500;
const DROP_RATIO: f64 = 0.1;
const SPEEDUP_RATIO: f64 = 0.9;
const LINES_BETWEEN_SPEEDUPS: u32 = 10;

// const SS_STARTED:   u32 = 0x4;

// This is just a dummy for initialization purposes
impl Default for Internal {
    fn default() -> Internal { Internal { state: 0,
										  piece: (Piece::random(), Piece::random()),
										  xy: (0, 0),
										  orientation: Orientation::North,
                                          score: (0, 0),
										  piece_counts: [0; 7],
										  bitmap: Vec::<u32>::new(),
										  timer: Timer::new(0, 0, 0),
	}}
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

impl ObjectImpl for Board {
    fn signals() -> &'static [Signal] {
        use once_cell::sync::Lazy;
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder("board-command")
                 // board ID, command mask (CMD_*)
                 .param_types([u32::static_type(), u32::static_type()])
                 .build(),
                 Signal::builder("mouse-click")
                 .param_types([u32::static_type(), u32::static_type()])
                 .build(),
            ]
        });
        SIGNALS.as_ref()
    }

    fn constructed(&self) {
        self.parent_constructed();
        let this = self;
		// Does this really need 3 separate handlers? The event doesn't seem to supply the button
        let gesture_left = gtk::GestureClick::new();
        gesture_left.connect_pressed(clone!(@weak this => move |gesture, _x, _y, _z| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            this.controller().emit_by_name::<()>("mouse-click", &[&this.id(), &0u32]);
        }));
        gesture_left.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        self.obj().add_controller(&gesture_left);

        let gesture_middle = gtk::GestureClick::new();
        gesture_middle.connect_pressed(clone!(@weak this => move |gesture, _x, _y, _z| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            this.controller().emit_by_name::<()>("mouse-click", &[&this.id(), &1u32]);
        }));
        gesture_middle.set_button(gtk::gdk::ffi::GDK_BUTTON_MIDDLE as u32);
        self.obj().add_controller(&gesture_middle);

        let gesture_right = gtk::GestureClick::new();
        gesture_right.connect_pressed(clone!(@weak this => move |gesture, _x, _y, _z| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            this.controller().emit_by_name::<()>("mouse-click", &[&this.id(), &2u32]);
        }));
        gesture_right.set_button(gtk::gdk::ffi::GDK_BUTTON_SECONDARY as u32);
        self.obj().add_controller(&gesture_right);
    }
}

impl WidgetImpl for Board {}
impl BoxImpl for Board {}

//////////////////////////////////////////////////////////////////
//
// app impl code starts here
//
//////////////////////////////////////////////////////////////////

// command mask used to send to BOARD. All others are handled in the controller
pub const CMD_LEFT: u32             = 1;
pub const CMD_RIGHT: u32            = 2;
pub const CMD_DOWN: u32             = 3;
pub const CMD_CLOCKWISE: u32        = 4;
pub const CMD_COUNTERCLOCKWISE: u32 = 5;
pub const CMD_SELECT: u32           = 6;
pub const CMD_DESELECT: u32         = 7;

pub const CMD_START: u32            = 8;
pub const CMD_STOP: u32             = 9;
pub const CMD_DROP: u32             = 10;

pub const CMD_CHEAT: u32            = 0x80000000;
pub const CMD_CHEAT_END: u32        = 0x80000100;

static PIECES: [Piece; 7] = [
    Piece {name: "Bar",        points: [12, 1, 12, 1, ], masks: [0x00f0, 0x2222, 0x00f0, 0x2222, ], },
    Piece {name: "Tee",        points: [ 6, 5,  2, 1, ], masks: [0x0270, 0x0232, 0x0072, 0x0262, ], },
    Piece {name: "Square",     points: [ 4, 4,  4, 4, ], masks: [0x0660, 0x0660, 0x0660, 0x0660, ], },
    Piece {name: "Zee",        points: [ 5, 3,  5, 3, ], masks: [0x0360, 0x0462, 0x0360, 0x0462, ], },
    Piece {name: "ReverseZee", points: [ 5, 3,  5, 3, ], masks: [0x0630, 0x0264, 0x0630, 0x0264, ], },
    Piece {name: "El",         points: [ 6, 6,  3, 3, ], masks: [0x0470, 0x0322, 0x0071, 0x0226, ], },
    Piece {name: "ReverseEl",  points: [ 3, 3,  6, 6, ], masks: [0x0740, 0x2230, 0x0170, 0x0622, ], },
];

#[derive(Debug)]
pub struct Piece {
    // NAME is used to identify the piece. It also is the name of the CSS class used to draw the piece.
    name: &'static str,
    // These arrays give the values for each piece. There are 4 for each - some pieces need fewer (BAR
    // needs 2, SQUARE needs 1), but rather than deal with different length vectors it is simpler just
    // to repeat the values until there are 4.
    // (NOTE: other implementations have used circular linked lists to manage this. Listing 4 rotations for
    // each object probably takes less code than handling the different cases individually)
    points: [u32; 4],
    masks: [u16; 4],
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Orientation {#[default] North, East, South, West, }

impl Orientation {
    pub fn rotate(&self, mask: u32) -> Orientation {
        match mask {
            CMD_CLOCKWISE => {
                match self {
                    Orientation::North => Orientation::East,
                    Orientation::East => Orientation::South,
                    Orientation::South => Orientation::West,
                    Orientation::West => Orientation::North,
                }},
            CMD_COUNTERCLOCKWISE => {
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
    // number of points for each piece in an orientation
    fn points(&self, orientation: Orientation) -> u32 { self.points[orientation.offset()] }

    // MASK is a u16 value interpreted as 4 lines of length 4 bits. This can encode all rotations of the pieces.
    fn mask(&self, orientation: Orientation) -> u16 { self.masks[orientation.offset()] }

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
    fn height(&self) -> u32 { *self.height_oc.get().unwrap() }
    fn width(&self) -> u32 { *self.width_oc.get().unwrap() }
    fn id(&self) -> u32 { *self.id_oc.get().unwrap() }
    fn show_preview(&self) -> bool { *self.show_preview_oc.get().unwrap() }
    // TODO: get via css ID?
	// Is this considered good practice?
//    fn controller(&self) -> Widget { self.obj().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap() }
    fn controller(&self) -> Root { self.obj().root().unwrap() }
    
    
    // Most initializes correctly by default, BITMAP relies on height and width
    pub fn prepare(&self) {
		let show_preview = self.show_preview();
        let mut bitmap: Vec<u32> = vec![0xffffffff; (self.height() + 4) as usize];
        let mask = !(((0x1 << self.width()) - 1) << 2);
        for i in 0..bitmap.len() - 2 {
            bitmap[i] &= mask;
        }
        {
            let mut internal = self.internal.borrow_mut();
            internal.bitmap = bitmap;
            internal.state = if show_preview {SS_PREVIEW} else {0};
        }
        self.start_new_piece(true);
    }

    fn start_new_piece(&self, initial: bool) -> bool{
        // The first time through there is no old piece to record on the bitmap. In subsequent calls the old piece
        // needs to be transferred to the bitmap before loading a new one
        if !initial {
            if !self.add_piece_to_bitmap() { return self.lose(); }
            self.update_score();
        }
        let show_preview = self.show_preview();
        let old_pos = 0;
		let delay = self.delay(false);
        {
            let mut internal = self.internal.borrow_mut();
            // prepare for next piece: reinitialize state for the new piece
            internal.piece_counts[old_pos] += 1;
            internal.piece = (internal.piece.1, Piece::random());
            internal.orientation = Orientation::North;
            internal.xy = ((self.width()/2 - 2) as i32, -1);
            internal.state |= SS_NEW_PIECE;
            internal.state &= !SS_DROPPING;
			internal.timer.stop();
			internal.timer = Timer::new(self.id(), delay, self.height());
			if !initial {internal.timer.start();}
        }
        {
            let internal = self.internal.borrow();
            if !self.can_move(internal.piece.0.mask(internal.orientation), internal.xy) {
				return self.lose();
            }
        }
        if show_preview {
            self.draw_preview();
        }
        self.draw_moved_piece(0, 0, Orientation::North);
        true
    }

    pub fn do_command(&self, bits: u32) {
        match bits {
            CMD_LEFT => self.translate_piece(1, 0),
            CMD_RIGHT => self.translate_piece(-1, 0),
            // ignore return value for everything but DOWN
            CMD_DOWN => self.translate_piece(0, 1) || self.start_new_piece(false),
            CMD_COUNTERCLOCKWISE => self.rotate_piece(CMD_COUNTERCLOCKWISE),
            CMD_CLOCKWISE => self.rotate_piece(CMD_CLOCKWISE),
			CMD_SELECT => { self.playing_area.add_css_class("selected"); true},
			CMD_DESELECT => { self.playing_area.remove_css_class("selected"); true},
			CMD_START => self.start(),
			CMD_STOP => {self.internal.borrow().timer.stop(); true},
			CMD_DROP => self.drop_piece(),
            CMD_CHEAT..=CMD_CHEAT_END => self.do_cheat(bits & 0xfff),
            _ => true,
        };
    }

    fn start(&self) -> bool{
		let delay = { self.delay(false) };
		let mut internal = self.internal.borrow_mut();
		internal.timer = Timer::new(self.id(), delay, self.height());
		internal.timer.start();
		true
	}

    fn translate_piece(&self, dx: i32, dy: i32) -> bool {
        let orientation;
        {
            let internal = self.internal.borrow();
            let (x, y) = (internal.xy.0 + dx, internal.xy.1 + dy);
            let mask = internal.piece.0.mask(internal.orientation);
            if !self.can_move(mask, (x, y)) {
                return false;
            }
            orientation = internal.orientation;
        }
        self.draw_moved_piece(dx, dy, orientation);
        true
    }
        
    fn rotate_piece(&self, rotate: u32) -> bool{
        let orientation;
        {
            let internal = self.internal.borrow();
            orientation = internal.orientation.rotate(rotate);
            let mask = internal.piece.0.mask(orientation);
            if !self.can_move(mask, internal.xy) { return false; }
        }
        self.draw_moved_piece(0, 0, orientation);
        true
    }

    fn delay(&self, dropping: bool) -> u32 {
		let lines = self.internal.borrow().score.1;
		let msecs: u32 = (STARTING_TICK_MS as i32 as f64 * f64::powf(SPEEDUP_RATIO, (lines / LINES_BETWEEN_SPEEDUPS).into())) as u32;
		if dropping { (msecs as i32 as f64 * DROP_RATIO) as u32} else { msecs }
	}
	
	fn drop_piece(&self) -> bool {
		let new_timer: Timer;
		{
			let internal = self.internal.borrow();
			if internal.state & SS_DROPPING != 0 { return false; }
			let old_timer = &internal.timer;
			let msecs = self.delay(true);
			new_timer = Timer::new(self.id(), msecs, old_timer.quit_count.get() as u32);
			old_timer.stop();
		}
		let mut internal = self.internal.borrow_mut();
		internal.state |= SS_DROPPING;
		new_timer.start();
		internal.timer = new_timer;
		true
	}

	fn lose(&self) -> bool {
		self.controller().emit_by_name::<()>("board-lost", &[&self.id(), ]);
		false
	}
		
    fn do_cheat(&self, code: u32) -> bool {
        match code {
            0..=8 => {
				{ self.internal.borrow_mut().piece.1 = &PIECES[code as usize]; }
				if self.show_preview() { self.draw_preview();};
			},
            10 => self.init_bitmap_to(&BITARRAY),
            11 => {
                println!("------------------------------------");
                self.internal.borrow().bitmap.iter().for_each(|x| { println!("| {:032b} |", x); });
                println!("------------------------------------");
            },
            12 => {self.internal.borrow().bitmap.iter().for_each(|x| { println!("0x{:x}", x); })},
            13 => self.remove_row(19),
            29 => { println!("remove row {}", code - 11); self.remove_row((code - 11) as i32);},
            _ => ()
        };
        true
    }
    
    fn update_score(&self) {
        let (lines, bonus) = self.completed_lines();
        let mut internal = self.internal.borrow_mut();
        let delta_score = internal.piece.0.points(internal.orientation) + bonus;
        internal.score.0 += delta_score;
        internal.score.1 += lines;
        self.points.set_label(&internal.score.0.to_string());
        self.lines.set_label(&internal.score.1.to_string());
        self.controller().emit_by_name::<()>("board-report", &[&self.id(), &delta_score, &lines]);
		
    }
    
    // see note above about different coordinate systems. Here is where they crash together.
    // BITMAP has padding of 2 bits on left, right, and bottom to make sure the mask always
    // is fully contained in the bitmap
    fn can_move(&self, mut mask: u16, xy: (i32, i32)) -> bool {
		if xy.1 > 0xffff {return false; }    // LOSE
        let bitmap = &self.internal.borrow().bitmap;
        let mut row: usize = (xy.1 + 2) as usize;
        while mask != 0 {
            let row_bits: u16 = ((bitmap[row] >> (xy.0 + 2)) & 0xf) as u16;
            if row_bits & mask != 0 { return false; }
            mask >>= 4;
            row += 1;
        }
        true
    }
	
    fn completed_lines(&self) -> (u32, u32) {
		// first compute the rows to remove
        let mut to_remove = Vec::<i32>::new();    // this holds the line numbers to remove in bitmap coordinates, in ascending order
		{
			let bitmap: &Vec<u32> = &self.internal.borrow().bitmap;
			// originally I used -1 here, as it is simpler. By making this mask I can use the leading bits to mark buffer rows for debugging
			let mask: u32 = (0x1 << (self.width() + 4)) - 1;
			for i in 2..(bitmap.len() as i32) - 2 {
//              if bitmap[i as usize] == 0xffffffff {
				if bitmap[i as usize] & mask == mask {
					to_remove.push(i - 2);
				} 
			}
		}
		// next redraw the board with the lines removed. This must be done top-to-bottom
        to_remove.iter().for_each(|x| self.remove_row(*x));

		// finally update the bitmap. This must be done bottom-to-top to maintain the offsets, and then add the new empty rows on top
		let bitmap = &mut self.internal.borrow_mut().bitmap;
		//let new_row_mask:u32 = 0xffffffff;
		let new_row_mask:u32 = !(((1 << self.width()) - 1) << 2);  // mask is -1 with the bits representing the playing area cleared
		for board_row in &to_remove {
			// working from top down, delete a row and replace it with a blank one on top
			// +2: move to bitmap coords
			bitmap.remove((board_row + 2) as usize);
			bitmap.insert(0, new_row_mask);
		}
		let len: u32 = to_remove.len() as u32;
		(len, len*len*5)    // (lines completed, completion bonus): bonus is 5 times completed lines squared (max of 100 pts)
    }
    
    fn remove_row(&self, row: i32) {
        // move down all cells above this row. Row is in board coords
		for y in (0..row + 1).rev() {
            for x in 0..self.width() as i32 {
                let upper_color = self.get_cell_color(x, y - 1);
                if upper_color != self.get_cell_color(x, y) {
                    self.set_cell_color((x, y), &upper_color);
                }
            }
        }
    }
    
    // All pieces in their North position are contained in rows 1 and 2, so the mask is of the form 0x0**0 and only the
    // middle 2 quartets are drawn
    fn draw_preview(&self) {
        let internal = self.internal.borrow();
        let mut mask = internal.piece.1.mask(Orientation::North) >> 4;
        let piece_type = &[internal.piece.1.name];
        let empty = &["empty"];
        for i in 0..8 {
            self.preview.child_at(3 - i%4, i/4).unwrap().set_css_classes( if mask & 1 > 0 {piece_type} else {empty});
            mask >>= 1;
        }
    }
    
    // Moves a piece to a new position. The new position should have already been checked, this
    // assumes the move can be made
    fn draw_moved_piece(&self, dx:i32, dy: i32, o1: Orientation) {
        let (x1, y1): (i32, i32);
        {
            let internal = self.internal.borrow();
            let piece = internal.piece.0;
            let (x0, y0) = internal.xy;
            (x1, y1) = (x0 + dx, y0 + dy);
            
            // set all cells which were off but will be on
            let name = piece.name;
            let mut set_mask = piece.big_mask(o1, 0, dy);
            if internal.state & SS_NEW_PIECE == 0 {
                set_mask &= !piece.big_mask(internal.orientation, -dx, 0);
            }
            let mut i = 0;
            while set_mask != 0 {
                if set_mask & 1 == 1 {
                    let row = i / 6;
                    let col = i % 6;
                    self.set_cell_color((x1 + col - 1, y1 + row - dy), name);
                }
                i += 1;
                set_mask >>= 1;
            }
            
            // clear all cells which do not remain on. If this is a new piece there are no cells to be cleared.
            if internal.state & SS_NEW_PIECE == 0 {
                let mut clear_mask = piece.big_mask(internal.orientation, 0, 0) & !piece.big_mask(o1, dx, dy);
                let mut i = 0;
                while clear_mask != 0 {
                    if clear_mask & 1 == 1 {
                        let row = i / 6;
                        let col = i % 6;
                        self.set_cell_color((x0 + col - 1, y0 + row), "empty");
                    }
                    i += 1;
                    clear_mask >>= 1;
                }
            }
        }
        let mut internal = self.internal.borrow_mut();
        internal.xy = (x1, y1);
        internal.orientation = o1;
        internal.state &= !SS_NEW_PIECE;
    }
    
    fn add_piece_to_bitmap(&self) -> bool {
        let mut internal = self.internal.borrow_mut();
		if internal.xy.1 < 0 {return false; }    // LOSE
        let mut mask = internal.piece.0.mask(internal.orientation) as u32;
        let mut row = internal.xy.1 as usize;
        while mask != 0 {
            internal.bitmap[row + 2] |= (mask & 0xf) << (internal.xy.0 + 2);
            mask >>= 4;
            row += 1;
        }
		true
    }
    
    // lowest level functions
    // Drawing the pieces is done by setting css class for widgets in the board.playing_area object. This way I don't need
    // to handle any redrawing, and setting the colors is simple. If drawing turns out to be better then board.playing_area
    // needs to be replaced with a drawing area and these functions need to be redone
    fn cell_at(&self, xy: (i32, i32)) -> Option<gtk::Widget> {
        self.playing_area.child_at((self.width() as i32) - 1 - xy.0, xy.1)
    }

    fn get_cell_color(&self, x: i32, y: i32) -> String {
        let cell_opt = self.cell_at((x, y));
        match cell_opt {
            None => "empty".to_string(),
            Some(widget) => { let class_names = widget.css_classes();
                              if !class_names.is_empty() {class_names[0].to_string()} else {"empty".to_string()}
            }
        }
    }
    
    fn set_cell_color(&self, xy: (i32, i32), piece_name: &str) {
		if let Some(cell) = self.cell_at(xy) { cell.set_css_classes(&[piece_name]); }
    }


    // debugging function: set BITARRAY to reconstruct position
    fn init_bitmap_to(&self, array: &[u32]) {
        {
            let mut internal = self.internal.borrow_mut();
            internal.bitmap = array.to_vec();
        }
        let internal = self.internal.borrow();
        for y in 2..self.height() + 2 {
            let mut bit = 0x1u32 << 2;     // skip the first 2 edge bits
            for x in 0..self.width() {
                if bit & internal.bitmap[y as usize] != 0 {
                    self.set_cell_color((x as i32, (y - 2) as i32), PIECES[(y as usize)%PIECES.len()].name);   // rows will be the same color
                }
                bit <<= 1;
            }
        }
    }
}

#[derive(Debug)]
struct Timer {
	board_id: u32,
	quit_count: Rc<Cell<i32>>,
	msecs: u32,
}
impl Timer {
	fn new(board_id: u32, msecs: u32, quit_count: u32) -> Timer {
		let quit_count_i32 = quit_count as i32;
		Timer {board_id, msecs, quit_count: Rc::new(Cell::new(quit_count_i32)), }
	}

	// be sure to stop the old timer when starting a new one
	fn start(&self) {
		let quit_count = Rc::clone(&self.quit_count);
		let board_id = self.board_id as usize;
		let f = move || -> glib::Continue {
			if quit_count.get() <= 0 { return glib::Continue(false); }
			board(board_id).imp().do_command(CMD_DOWN);
			quit_count.set(quit_count.get() - 1);
			glib::Continue(true)
		};
        glib::timeout_add_local(core::time::Duration::from_millis(self.msecs as u64), f);
	}
	fn stop(&self) { self.quit_count.set(0); }
}

// set this up and use with init_bitmap_to() for debugging special cases (get data from cheat 11)
/*
const BITARRAY: [u32; 24] = [
    0x007FF003,
    0x017FF003,
    0x027FF003,
    0x037FF003,
    0x047FF003,
    0x057FF003,
    0x067FF003,
    0x077FF003,
    0x087FF003,
    0x097FF003,
    0x0a7FF003,
    0x0b7FF003,
    0x0c7FF003,
    0x0d7FF003,
    0x0e7FF003,
    0x0f7FF003,
    0x107FF003,
    0x117FF003,
    0x127FFFF7,
    0x137FFFF7,
    0x147FFFF7,
    0x157FFFF7,
    0x167FFFFF,
    0x177FFFFF,
];
 */
const BITARRAY: [u32; 24] = [
    0xFFFF0003,
    0xFFFF0003,
    0xFFFF0003,
    0xFFFF0003,
    0xFFFF0003,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFF0FFF,
    0xFFFFFFFF,
    0xFFFFFFFF,
];
