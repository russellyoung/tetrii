use crate::Board;
use std::rc::Rc;
use std::collections::HashMap;

use gtk::{glib, CompositeTemplate};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gdk4::ModifierType;

// this gives a warning as unused, but removing it breaks the Default for Internal
use std::cell::{RefCell, Cell};
use once_cell::sync::Lazy;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {#[default] Paused, Running, Finished, }

//#[derive(Debug, Default)]
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "controller.ui")]
pub struct Controller {
    internal: Rc<RefCell<Internal>>,
    
    #[template_child]
    pub boards_container: TemplateChild<gtk::Box>,
    #[template_child]
    pub total_points: TemplateChild<gtk::Label>,
    #[template_child]
    pub total_lines: TemplateChild<gtk::Label>,
    #[template_child]
    pub start_buttonx: TemplateChild<gtk::Button>,
    #[template_child]
    pub quit_buttonx: TemplateChild<gtk::Button>,
    //    pub grid: gtk::Grid,
}

#[derive(Debug, Default)]
struct Internal {
    active: usize,        // the board to direct commands to
    score: (u32, u32),    // (points, completed lines)
    state: State,
    dropping: u32,        // mask telling if a board is currently dropping a piece
}

#[glib::object_subclass]
impl ObjectSubclass for Controller {
    const NAME: &'static str = "Controller";
    type Type = super::Controller;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    // You must call `Widget`'s `init_template()` within `instance_init()`.
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

static mut BOARDS: Lazy<Vec<Board>> = Lazy::new(|| Vec::new());
static mut COMMANDMAP: Lazy<HashMap<String, Command>> = Lazy::new(|| {
    let mut hashmap: HashMap<String, Command> = HashMap::new();
    COMMANDS.iter().for_each(|desc| { hashmap.insert(desc.0.to_string(), desc.1); });
    hashmap
});

impl ObjectImpl for Controller {
    fn constructed(&self) {
        self.parent_constructed();
        let gcontroller = self.obj();
        let controller = self;
        controller.quit_buttonx.connect_clicked(clone!(@weak gcontroller => move |_| gcontroller.destroy()));
        
        let key_handler = gtk::EventControllerKey::new();
        controller.obj().add_controller(&key_handler);
        let internal = Rc::clone(&self.internal);
        key_handler.connect_key_pressed(move |_ctlr, key, _code, state| {
            do_command(&internal, keyboard_input(key, state));
            gtk::Inhibit(false)
        });
		/*
        let gesture = gtk::GestureClick::new();
        gesture.connect_pressed(|gesture, id, button| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        do_command(&internal, mouse_input(button));
        gtk::Inhibit(false)
    });
        controller.obj().add_controller(&gesture);
         */
    }

    fn signals() -> &'static [Signal] {
//        use once_cell::sync::Lazy;
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder("board-report")
                 // board id, points, lines
                 .param_types([u32::static_type(), u32::static_type(), u32::static_type(), ])
                 .build(),
                 Signal::builder("mouse-click")
                 // board id, which-mouse
                 .param_types([u32::static_type(), u32::static_type(), ])
                 .build(),
            ]
        });
        SIGNALS.as_ref()
    }
}

impl WidgetImpl for Controller {}
impl WindowImpl for Controller {}
impl ApplicationWindowImpl for Controller {}

//////////////////////////////////////////////////////////////////
//
// End of boilerplate, start of custom code
//
//////////////////////////////////////////////////////////////////

// if I have time and interest the commands will be configurable through the .tetrii file
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Command {Left,               // commands that are sent to the Boards
                  Right,
                  Down,
                  Clockwise,
                  CounterClockwise,
                  Drop,               // Commands that are processed locally
                  Pause,
                  Resume,
                  TogglePause,
                  SetBoard(usize),
                  Cheat(u32),
                  #[default] Nop,
}

// command mask used to send to BOARD. All others are handled locally
use crate::{CMD_LEFT, CMD_RIGHT, CMD_DOWN, CMD_CLOCKWISE, CMD_COUNTERCLOCKWISE, CMD_SELECT, CMD_DESELECT, CMD_CHEAT};

// default commands
const COMMANDS:[(&str, Command); 31] =
    [(&"Right",  Command::Right),
     (&"Left",   Command::Left),
     (&"Down",   Command::Down),
     (&"q",      Command::CounterClockwise),
     (&"q-Shift",Command::Left),
     (&"e",      Command::Clockwise),
     (&"space",  Command::Drop),
     (&"s",      Command::Resume),
     (&"t",      Command::TogglePause),
     (&"p",      Command::Pause),
     (&"Mouse1", Command::Left),
     (&"Mouse2", Command::Down),
     (&"Mouse3", Command::Right),
     (&"1",      Command::SetBoard(0)),
     (&"2",      Command::SetBoard(1)),
     (&"3",      Command::SetBoard(2)),
     (&"4",      Command::SetBoard(3)),
     (&"5",      Command::SetBoard(4)),
     (&"1-Ctrl", Command::Cheat(0)),   // force piece
     (&"2-Ctrl", Command::Cheat(1)),
     (&"3-Ctrl", Command::Cheat(2)),
     (&"4-Ctrl", Command::Cheat(3)),
     (&"5-Ctrl", Command::Cheat(4)),
     (&"6-Ctrl", Command::Cheat(5)),
     (&"7-Ctrl", Command::Cheat(6)),
     (&"b-Ctrl", Command::Cheat(10)),  // use fake bitmap: insert bitmap at BITARRAY and recompile
     (&"d-Shift",Command::Cheat(11)), // dump bitmap binary, easy to see current state
     (&"d-Ctrl", Command::Cheat(12)),  // dump bitmap hex, can paste into BITARRAY for debugging
     (&"p-Ctrl", Command::Cheat(13)),  
     (&"s-Ctrl", Command::Cheat(14)),  // print board substatus
     (&"9-Ctrl", Command::Cheat(29)), // remove second-to-las
];

impl Controller {
    pub fn initialize(&self, board_count: u32, width: u32, height: u32, preview: bool) {
        unsafe {
            BOARDS.clear();
            let container = &self.boards_container;
            for i in 0..board_count {
                let b = Board::new(i, width, height, preview);
                container.append(&b);
                BOARDS.push(b);
            }
        }
		send_command(0, CMD_SELECT);
    }

    pub fn piece_crashed(&self, board_id: u32, points: u32, lines: u32) {
        let mut internal = self.internal.borrow_mut();
        internal.dropping &= !0x1 << board_id;
        let old_score = internal.score;
        internal.score = (old_score.0 + points, old_score.1 + lines);
        self.total_points.set_label(&internal.score.0.to_string());
        self.total_lines.set_label(&internal.score.1.to_string());
    }

    pub fn mouse_click(&self, _id: u32, button: u32) {
        let internal = Rc::clone(&self.internal);
        do_command(&internal, mouse_input(button));
    }
}

fn do_command(internal: &Rc<RefCell<Internal>>, command: Command) {
    let id = { internal.borrow().active};
    match command {
        // board commands
        Command::Left => send_command(id, CMD_LEFT), 
        Command::Right => send_command(id, CMD_RIGHT),
        Command::Down => send_command(id, CMD_DOWN),
        Command::Clockwise => send_command(id, CMD_CLOCKWISE),
        Command::CounterClockwise => send_command(id, CMD_COUNTERCLOCKWISE),
        Command::Cheat(code) => send_command(id, CMD_CHEAT | code),
        // controller commands
        Command::Drop => (),
        Command::Pause => (),
        Command::Resume => (),
        Command::TogglePause => (),
        Command::SetBoard(new_id) => {internal.borrow_mut().active = set_board(id, new_id)},
        Command::Nop => (),
    }
}

fn set_board(old_id: usize, new_id: usize) -> usize {
	if new_id >= board_count() || new_id == old_id { return old_id; }
	send_command(old_id, CMD_DESELECT);
	send_command(new_id, CMD_SELECT);
	new_id
}

fn keyboard_input(key: gdk4::Key, modifiers: ModifierType) -> Command {
    let key_string = modifier_string(key.to_lower().name().unwrap().to_string(), modifiers.bits());
    unsafe {
        *COMMANDMAP.get(&key_string).unwrap_or(&Command::Nop)
    }
}

fn mouse_input(button: u32) -> Command {
    let button_string = format!("Mouse{}", button + 1);
    unsafe {
        *COMMANDMAP.get(&button_string).unwrap_or(&Command::Nop)
    }
}

fn send_command(id: usize, mask: u32) {
    let id_u32 = id as u32;
    if id < board_count() {
        board(id).emit_by_name::<()>("board-command", &[&id_u32, &mask, ]);
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

fn board_count() -> usize { unsafe { BOARDS.len() } }

fn board(which: usize) -> &'static Board { unsafe { &BOARDS[which] } }

fn tick(msec: u32, board_id: u32) {
	/*
    unsafe {
        let f = move || -> glib::Continue {
            glib::Continue(
				send_command(board_id, CMD_DOWN);
                glib::Continue(success)
				
                if mut_board.substate & SS_DROPPING != 0 { true }                      // continue the timer, but don't move the piece
                    else if mut_board.substate & (SS_PAUSED | SS_OVER) > 0 { false } // stop if paused or finished
                    else {
                        mut_board.do_command(&Command::Down) ||         // move down if possible...
                            mut_board.start_new_piece(false)            // ... or get new piece
                    })
            };
            glib::timeout_add_local(core::time::Duration::from_millis(msec), f);
        }
	 */
}
