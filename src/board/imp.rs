use self::glib::{BindingFlags, ParamSpec, ParamSpecInt, Value};
use gtk::glib;
use gtk::CompositeTemplate;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use gtk::glib::subclass::Signal;
use fastrand;

use gtk::prelude::GridExt;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

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
}

const SS_NEW_PIECE: u32 = 0x1;    // the current piece is a new one, draw all squares without checking if they are already on
const SS_PREVIEW:   u32 = 0x2;    // flag to do preview, simpler than getting it from he main structure
const SS_STARTED:   u32 = 0x4;

// This is just a dummy for initialization purposes
impl Default for Internal {
    fn default() -> Internal { Internal { piece: (Piece::random(), Piece::random()), xy: (0, 0), orientation: Orientation::North,
                                          score: (0, 0), piece_counts: [0; 7], bitmap: Vec::<u32>::new(), state: 0, }}
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
                 .param_types([u32::static_type(), u32::static_type()])
                 .build(),
                 Signal::builder("board-report")
                 .param_types([u32::static_type(), u32::static_type(), u32::static_type()])
                 .build(),
                 
            ]
        });
        SIGNALS.as_ref()
    }
}

impl WidgetImpl for Board {}
impl BoxImpl for Board {}

//////////////////////////////////////////////////////////////////
//
// Real code starts here
//
//////////////////////////////////////////////////////////////////

// command mask used to send to BOARD. All others are handled in the controller
use crate::{CMD_LEFT, CMD_RIGHT, CMD_DOWN, CMD_CLOCKWISE, CMD_COUNTERCLOCKWISE, CMD_CHEAT, CMD_CHEAT_END};

static PIECES: [Piece; 7] = [
    Piece {name: &"Bar",        points: [12, 1, 12, 1, ], masks: [0x00f0, 0x2222, 0x00f0, 0x2222, ], pos: 0, },
    Piece {name: &"Tee",        points: [ 6, 5,  2, 1, ], masks: [0x0270, 0x0232, 0x0072, 0x0262, ], pos: 1, },
    Piece {name: &"Square",     points: [ 4, 4,  4, 4, ], masks: [0x0660, 0x0660, 0x0660, 0x0660, ], pos: 2, },
    Piece {name: &"Zee",        points: [ 5, 3,  5, 3, ], masks: [0x0360, 0x0462, 0x0360, 0x0462, ], pos: 3, },
    Piece {name: &"ReverseZee", points: [ 5, 3,  5, 3, ], masks: [0x0630, 0x0264, 0x0630, 0x0264, ], pos: 4, },
    Piece {name: &"El",         points: [ 6, 6,  3, 3, ], masks: [0x0470, 0x0322, 0x0071, 0x0226, ], pos: 5, },
    Piece {name: &"ReverseEl",  points: [ 3, 3,  6, 6, ], masks: [0x0740, 0x2230, 0x0170, 0x0622, ], pos: 6, },
];

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
    
    pub fn prepare(&self) {
        let mut bitmap: Vec<u32> = vec![0xffffffff; (self.height() + 4) as usize];
        let mask = !(((0x1 << self.width()) - 1) << 2);
        for i in 0..bitmap.len() - 2 {
            bitmap[i] &= mask;
        }
        {
            let mut internal = self.internal.borrow_mut();
            internal.bitmap = bitmap;
            internal.state = if *self.show_preview_oc.get().unwrap() {SS_PREVIEW} else {0};
            // everything else initializes correctly by default, BITMAP relies on height and width
        }
        self.start_new_piece(true);
    }

    pub fn do_command(&self, bits: u32) {
        match bits {
            CMD_LEFT => self.translate_piece(1, 0),
            CMD_RIGHT => self.translate_piece(-1, 0),
            CMD_DOWN => self.translate_piece(0, 1),
            CMD_COUNTERCLOCKWISE => self.rotate_piece(CMD_COUNTERCLOCKWISE),
            CMD_CLOCKWISE => self.rotate_piece(CMD_CLOCKWISE),
            CMD_CHEAT..=CMD_CHEAT_END => (),
            _ => (),
        }
    }
        
    fn start_new_piece(&self, initial: bool) -> bool{
        // The first time through there is no old piece to record on the bitmap. In subsequent calls the old piece
        // needs to be transferred to the bitmap before loading a new one
        if !initial {
            //            self.add_piece_to_bitmap()
            // send scores
        }
        let mut show_preview = false;
        let mut old_pos = 0;
        {
            let internal = self.internal.borrow();
            old_pos = internal.piece.0.pos;
            if !self.can_move(internal.piece.0.mask(internal.orientation), internal.xy) {
                // send message to main program
                return false;
            }
        }
        {
            let mut internal = self.internal.borrow_mut();
            // prepare for next piece: reinitialize state for the new piece
            internal.piece_counts[old_pos] += 1;
            internal.piece = (internal.piece.1, Piece::random());
            internal.orientation = Orientation::North;
            internal.xy = ((self.width()/2 - 2) as i32, -1);
            show_preview = internal.state & SS_PREVIEW > 0;
            internal.state |= SS_NEW_PIECE;
        }
        if show_preview {
            self.draw_preview();
        }
        self.draw_moved_piece(0, 0, Orientation::North);
        true
    }
    
    fn translate_piece(&self, dx: i32, dy: i32) {
        let mut orientation = Orientation::North;
        {
            let internal = self.internal.borrow();
            let (x, y) = (internal.xy.0 + dx, internal.xy.1 + dy);
            let mask = internal.piece.0.mask(internal.orientation);
            if !self.can_move(mask, (x, y)) { return ; }
            orientation = internal.orientation;
        }
        self.draw_moved_piece(dx, dy, orientation);
    }
        
    fn rotate_piece(&self, rotate: u32) {
        let mut orientation = Orientation::North;
        {
            let internal = self.internal.borrow();
            orientation = internal.orientation.rotate(rotate);
            let mask = internal.piece.0.mask(orientation);
            if !self.can_move(mask, internal.xy) { return; }
        }
        self.draw_moved_piece(0, 0, orientation);
    }
    
    // see note above about different coordinate systems. Here is where they crash together.
    // BITMAP has padding of 2 bits on left, right, and bottom to make sure the mask always
    // is fully contained in the bitmap
    fn can_move(&self, mut mask: u16, xy: (i32, i32)) -> bool {
        let bitmap = &self.internal.borrow().bitmap;
        let mut row: usize = (xy.1 + 2) as usize;
        while mask != 0 {
            let row_bits: u16 = ((bitmap[row] >> xy.0 + 2) & 0xf) as u16;
            if row_bits & mask != 0 { return false; }
            mask >>= 4;
            row += 1;
        }
        true
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
        let (mut x1, mut y1): (i32, i32) = (0, 0);
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
    
    // lowest level functions
    // Drawing the pieces is done by setting css class for widgets in the board.playing_area object. This way I don't need
    // to handle any redrawing, and setting the colors is simple. If drawing turns out to be better then board.playing_area
    // needs to be replaced with a drawing area and these functions need to be redone
    fn cell_at(&self, xy: (i32, i32)) -> Option<gtk::Widget> {
        self.playing_area.child_at((self.width() as i32) - 1 - xy.0 as i32, xy.1)
    }

    fn set_cell_color(&self, xy: (i32, i32), piece_name: &str) {
        match self.cell_at(xy) {
            Some(cell) => cell.set_css_classes(&["cell", piece_name]),
            None => (),    // if the cell is off the visible board just don't draw it
        };
    }
}
    
