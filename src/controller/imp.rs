use crate::Board;
use std::rc::Rc;
use std::collections::HashMap;

use gtk::{glib, CompositeTemplate};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gdk4::ModifierType;
use gtk::Window;
// this gives a warning as unused, but removing it breaks the Default for Internal
use std::cell::{RefCell, Cell};
use once_cell::sync::Lazy;

use crate::BOARDS;
use crate::CONTROLLER;

// CONTROLLER accessors
pub(super) fn has_instance() -> bool { unsafe { CONTROLLER.is_some() }}
pub(super) fn set_instance(controller: crate::controller::Controller) { unsafe { CONTROLLER = Some(controller); }}
pub(super) fn controller_full() -> &'static crate::controller::Controller { unsafe { CONTROLLER.as_ref().unwrap() }}
fn controller<'a>() -> &'a Controller {
	unsafe {
		if CONTROLLER.is_none() {
			panic!("Request for controller when none is set");
		}
		CONTROLLER.as_ref().unwrap().imp()
	}
}

// BOARD accessors
fn boards_len() -> usize { unsafe { BOARDS.len() } }
pub fn board(which: usize) -> &'static Board { unsafe { &BOARDS[which] } }
fn boards_reset() { unsafe { BOARDS.clear(); }}
fn boards_add(board: Board) { unsafe { BOARDS.push(board); }}

//
// end STATIC MUTS
//

static COMMANDMAP: Lazy<HashMap<String, Command>> = Lazy::new(|| {
    let mut hashmap: HashMap<String, Command> = HashMap::new();
    COMMANDS.iter().for_each(|desc| { hashmap.insert(desc.0.to_string(), desc.1); });
    hashmap
});
fn command_map_get(key: &String) -> Command { *COMMANDMAP.get(key).unwrap_or(&Command::Nop)}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {#[default] Initial, Paused, Running, Finished, }

const STARTING_TICK_MS: u32 = 500;
const DROP_RATIO: u32 = 10;
const SLOWDOWN_RATIO: f64 = 0.9;

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
	modifier_bits: u32,
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

impl ObjectImpl for Controller {
    fn constructed(&self) {
        self.parent_constructed();
        let gcontroller = self.obj();
        self.quit_buttonx.connect_clicked(clone!(@weak gcontroller => move |_| {
			println!("quit button");
			gcontroller.destroy();
		}));
        self.start_buttonx.connect_clicked( |button| { controller().toggle_state(); });
        let key_handler = gtk::EventControllerKey::new();
        self.obj().add_controller(&key_handler);
        let internal = Rc::clone(&self.internal);
        key_handler.connect_key_pressed(move |_ctlr, key, _code, mods| {
			set_modifier(key, true);
            controller().do_command(keyboard_input(key, mods));
            gtk::Inhibit(true)
        });
        key_handler.connect_key_released(move |_ctlr, key, _code, mods| {
			set_modifier(key, false);
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
				 Signal::builder("board-lost")
                 // board id
                 .param_types([u32::static_type(), ])
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
use crate::board::imp::{CMD_LEFT,
						CMD_RIGHT,
						CMD_DOWN,
						CMD_CLOCKWISE,
						CMD_COUNTERCLOCKWISE,
						CMD_SELECT,
						CMD_DESELECT,
						CMD_CHEAT,
						CMD_START,
						CMD_STOP,
						CMD_DROP,
};

// default commands
const COMMANDS:[(&str, Command); 46] =
    [(&"Right",       Command::Right),
     (&"Left",        Command::Left),
	 (&"Right-Ctrl",  Command::Clockwise),
     (&"Left-Ctrl",   Command::CounterClockwise),
     (&"Down",        Command::Down),
     (&"q",           Command::CounterClockwise),
     (&"q-Shift",     Command::Left),
     (&"e",           Command::Clockwise),
     (&"space",       Command::Drop),
     (&"s",           Command::Resume),
     (&"t",           Command::TogglePause),
     (&"p",           Command::Pause),
     (&"Mouse1",      Command::Left),
     (&"Mouse2",      Command::Down),
     (&"Mouse3",      Command::Right),
     (&"1",           Command::SetBoard(0)),
     (&"2",           Command::SetBoard(1)),
     (&"3",           Command::SetBoard(2)),
     (&"4",           Command::SetBoard(3)),
     (&"5",           Command::SetBoard(4)),
     (&"0-Ctrl",      Command::Cheat(0)),   // force piece
     (&"1-Ctrl",      Command::Cheat(1)),   // force piece
     (&"2-Ctrl",      Command::Cheat(2)),
     (&"3-Ctrl",      Command::Cheat(3)),
     (&"4-Ctrl",      Command::Cheat(4)),
     (&"5-Ctrl",      Command::Cheat(5)),
     (&"6-Ctrl",      Command::Cheat(6)),
     (&"7-Ctrl",      Command::Cheat(7)),
     (&"8-Ctrl",      Command::Cheat(8)),
     (&"9-Ctrl",      Command::Cheat(9)),
     (&"b-Ctrl",      Command::Cheat(10)),  // use fake bitmap: insert bitmap at BITARRAY and recompile
     (&"d-Shift",     Command::Cheat(11)),  // dump bitmap binary, easy to see current state
     (&"d-Ctrl",      Command::Cheat(12)),  // dump bitmap hex, can paste into BITARRAY for debugging
     (&"p-Ctrl",      Command::Cheat(13)),  
     (&"s-Ctrl",      Command::Cheat(14)),  // print board substatus
     (&"9-Ctrl",      Command::Cheat(15)),  // remove second-to-last row
	 // cheat codes 0-20 are forwarded to the active board, higher codes are handled on the controller in controller_cheat()
     (&"0-Alt",       Command::Cheat(20)),
     (&"1-Alt",       Command::Cheat(21)),
     (&"2-Alt",       Command::Cheat(22)),
     (&"3-Alt",       Command::Cheat(23)),
     (&"4-Alt",       Command::Cheat(24)),
     (&"5-Alt",       Command::Cheat(25)),
     (&"6-Alt",       Command::Cheat(26)),
     (&"7-Alt",       Command::Cheat(27)),
     (&"8-Alt",       Command::Cheat(28)),
     (&"9-Alt",       Command::Cheat(29)),
];

impl Controller {
	fn active_id(&self) -> usize { self.internal.borrow().active }
    pub fn initialize(&self, board_count: u32, width: u32, height: u32, preview: bool) {
		self.set_state(State::Initial);
        boards_reset();
		
        let container = &self.boards_container;
        for i in 0..board_count {
            let b = Board::new(i, width, height, preview);
            container.append(&b);
            boards_add(b);
        }
		self.send_command(CMD_SELECT);
    }

	fn toggle_state(&self) {
		let state = { self.internal.borrow().state };
		match state {
			State::Initial | State::Paused => self.set_state(State::Running),
			State::Running => self.set_state(State::Paused),
			State::Finished => ()
		}
	}

	fn set_state(&self, state: State) {
		match state {
			State::Initial => {
				self.start_buttonx.set_visible(true);
				self.start_buttonx.set_label("Start");
			},
			State::Paused => {
				self.start_buttonx.set_label("Continue");
				send_command_all(CMD_STOP);
			},
			State::Running => {
				self.start_buttonx.set_label("Pause");
				send_command_all(CMD_START);
			},
			State::Finished => {
				self.start_buttonx.set_visible(false);
				send_command_all(CMD_STOP);
			}
		}
		// in case Button grabbed it
		self.obj().grab_focus();
		self.internal.borrow_mut().state = state;
	}

    pub fn board_lost(&self, board_id: u32) { self.set_state(State::Finished); }

    pub fn piece_crashed(&self, board_id: u32, points: u32, lines: u32) {
		let board_id: usize = board_id as usize;
        let mut internal = self.internal.borrow_mut();
        internal.dropping &= !0x1 << board_id;
        let old_score = internal.score;
        internal.score = (old_score.0 + points, old_score.1 + lines);
        self.total_points.set_label(&internal.score.0.to_string());
        self.total_lines.set_label(&internal.score.1.to_string());
    }

    pub fn mouse_click(&self, _id: u32, button: u32) {
        let internal = Rc::clone(&self.internal);
        self.do_command(mouse_input(button));
    }

	fn do_command(&self, command: Command) {
		match command {
			// board commands
			Command::Left => self.send_command(CMD_LEFT), 
			Command::Right => self.send_command(CMD_RIGHT),
			Command::Down => self.send_command(CMD_DOWN),
			Command::Clockwise => self.send_command(CMD_CLOCKWISE),
			Command::CounterClockwise => self.send_command(CMD_COUNTERCLOCKWISE),
			// controller commands
			Command::Drop => self.send_command(CMD_DROP),
			Command::Pause => (),
			Command::Resume => (),
			Command::TogglePause => (),
			Command::SetBoard(new_id) => {self.internal.borrow_mut().active = self.set_board(new_id)},
			Command::Nop => (),
			Command::Cheat(code) => { if code < 20 {self.send_command(CMD_CHEAT | code)} else { self.controller_cheat(code); }},
		}
	}

	fn set_board(&self, new_id: usize) -> usize {
		let old_id = self.active_id();
		if new_id >= boards_len() || new_id == old_id { return old_id; }
		send_command_to(old_id, CMD_DESELECT);
		send_command_to(new_id, CMD_SELECT);
		new_id
	}

	fn controller_cheat(&self, code: u32) {
		match code {
			_ => (),
		}
	}

	fn send_command(&self, mask: u32) {
		let id = self.active_id();
		if id < boards_len() {
			let id_u32 = self.active_id() as u32;
			board(id).emit_by_name::<()>("board-command", &[&id_u32, &mask, ]);
		}
	}
}

fn send_command_all(mask: u32) { for id in 0..boards_len() {send_command_to(id, mask); } }
fn send_command_to(id: usize, mask: u32) {
    let id_u32 = id as u32;
    if id < boards_len() {
        board(id).emit_by_name::<()>("board-command", &[&id_u32, &mask, ]);
    }
}


//////////////////////////////////////////////////////////////////
//
// Handle user input
//
// Keeps track of the modifier keys and maps input to commands
//
//////////////////////////////////////////////////////////////////
fn mouse_input(button: u32) -> Command {
    let button_string = modifier_bits_string(format!("Mouse{}", button + 1));
    command_map_get(&button_string)
}

fn keyboard_input(key: gdk4::Key, modifiers: ModifierType) -> Command {
    let key_string = modifier_bits_string(key.to_lower().name().unwrap().to_string());
    command_map_get(&key_string)
}

fn modifier_bits_string(mut key: String) -> String {
	let bits = controller().internal.borrow().modifier_bits;
	if bits & ModifierType::SHIFT_MASK.bits() != 0 { key.push_str("-Shift"); }
	if bits & ModifierType::ALT_MASK.bits() != 0 { key.push_str("-Alt"); }
	if bits & ModifierType::CONTROL_MASK.bits() != 0 { key.push_str("-Ctrl"); }
	if bits & ModifierType::META_MASK.bits() != 0 { key.push_str("-Meta"); }
    key
}

fn set_modifier(key: gdk4::Key, pressed: bool) {
	let name = key.to_lower().name().unwrap().to_string();
	let mask = match &name[..] {
		"Shift_L"   | "Shift_R"   => ModifierType::SHIFT_MASK.bits(),
		"Control_L" | "Control_R" => ModifierType::CONTROL_MASK.bits(),
		"Alt_L"     | "Alt_R"     => ModifierType::ALT_MASK.bits(),
		"Meta_L"    | "Meta_R"    => ModifierType::META_MASK.bits(),
		_ => return,
	};
	let mut internal = controller().internal.borrow_mut();
	if pressed { internal.modifier_bits  |= mask; }
	else { internal.modifier_bits &= !mask; }
}


