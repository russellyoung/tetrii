// Was src/controller/imp.rs

pub mod summary;

use crate::BOARDS;
use crate::Board;
use crate::CONTROLLER;
use crate::controller_inst;
use crate::controller::imp::summary::Summary as SummaryWidget;

use std::rc::Rc;
use std::collections::HashMap;

use gtk::{glib, CompositeTemplate};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gdk4::ModifierType;
// this gives a warning as unused, but removing it breaks the Default for Internal
use std::cell::{Cell, RefCell};
use once_cell::sync::Lazy;

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
pub fn board(which: u32) -> &'static Board { unsafe { &BOARDS[which as usize] } }
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

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum State {#[default] Initial, Paused, Running, Finished, }

//#[derive(Debug, Default)]
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "controller.ui")]
pub struct Controller {
    pub internal: Rc<RefCell<Internal>>,
    
    #[template_child]
    pub boards_container: TemplateChild<gtk::Box>,
    #[template_child]
    pub total_points: TemplateChild<gtk::Label>,
    #[template_child]
    pub total_lines: TemplateChild<gtk::Label>,
    #[template_child]
    pub time_disp: TemplateChild<gtk::Label>,
    #[template_child]
    pub start_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub quit_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub options_button: TemplateChild<gtk::Button>,
    //    pub grid: gtk::Grid,
}

#[derive(Debug, Default)]
pub struct Internal {
    active: u32,        // the board to direct commands to
    score: (u32, u32),    // (points, completed lines)
    state: State,
	modifier_bits: u32,
	seconds: u32,
	clock: Clock,
    pub summary: Option<SummaryWidget>,
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
        self.quit_button.connect_clicked(clone!(@weak gcontroller => move |_| { gcontroller.exit(); }));
        self.options_button.connect_clicked( |_button| { Controller::options(true); });
        self.start_button.connect_clicked( |_button| { controller_inst().toggle_state(); });
        let key_handler = gtk::EventControllerKey::new();
        self.obj().add_controller(&key_handler);
        key_handler.connect_key_pressed(move |_ctlr, key, _code, _mods| {
			set_modifier(key, true);
            controller_inst().do_command(keyboard_input(key));
            gtk::Inhibit(true)
        });
        key_handler.connect_key_released(move |_ctlr, key, _code, _mods| {
			set_modifier(key, false);
        });
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder("board-report")
                 // board id, points, lines
                 .param_types([u32::static_type(), u32::static_type(), u32::static_type(), u32::static_type(), ])
                 .build(),
				 Signal::builder("board-lost")
                 // board id
                 .param_types([u32::static_type(), ])
                 .build(),
                 Signal::builder("mouse-click")
                 // board id, which-mouse
                 .param_types([u32::static_type(), u32::static_type(), ])
                 .build(),
                 Signal::builder("select")
                 // board id, 
                 .param_types([u32::static_type(), ])
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
                  SetBoard(u32),
                  Cheat(u32),
                  #[default] Nop,
}

impl Command {
	fn allowed(&self, state: &State) -> bool {
		if state == &State::Running { true }
		else if state == &State::Finished { false }
		else { matches!(self, Command::Resume | Command::TogglePause) || matches!(self, Command::Cheat(_code))}
	}
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
const COMMANDS:[(&str, Command); 48] =
    [("Right",       Command::Right),
     ("Left",        Command::Left),
	 ("Right-Ctrl",  Command::Clockwise),
     ("Left-Ctrl",   Command::CounterClockwise),
     ("Down",        Command::Down),
     ("q",           Command::CounterClockwise),
     ("q-Shift",     Command::Left),
     ("e",           Command::Clockwise),
     ("space",       Command::Drop),
     ("s",           Command::Resume),
     ("t",           Command::TogglePause),
     ("p",           Command::Pause),
     ("Mouse1",      Command::Left),
     ("Mouse2",      Command::Drop),
     ("Mouse3",      Command::Right),
     ("Mouse1-Ctrl", Command::CounterClockwise),
     ("Mouse3-Ctrl", Command::Clockwise),
     ("1",           Command::SetBoard(0)),
     ("2",           Command::SetBoard(1)),
     ("3",           Command::SetBoard(2)),
     ("4",           Command::SetBoard(3)),
     ("5",           Command::SetBoard(4)),
     ("0-Ctrl",      Command::Cheat(0)),   // force piece
     ("1-Ctrl",      Command::Cheat(1)),   // force piece
     ("2-Ctrl",      Command::Cheat(2)),
     ("3-Ctrl",      Command::Cheat(3)),
     ("4-Ctrl",      Command::Cheat(4)),
     ("5-Ctrl",      Command::Cheat(5)),
     ("6-Ctrl",      Command::Cheat(6)),
     ("7-Ctrl",      Command::Cheat(7)),
     ("8-Ctrl",      Command::Cheat(8)),
     ("9-Ctrl",      Command::Cheat(9)),
     ("b-Ctrl",      Command::Cheat(10)),  // use fake bitmap: insert bitmap at BITARRAY and recompile
     ("d-Shift",     Command::Cheat(11)),  // dump bitmap binary, easy to see current state
     ("d-Ctrl",      Command::Cheat(12)),  // dump bitmap hex, can paste into BITARRAY for debugging
     ("p-Ctrl",      Command::Cheat(13)),  
     ("s-Ctrl",      Command::Cheat(14)),  // print board substatus
     ("9-Ctrl",      Command::Cheat(15)),  // remove second-to-last row
	 // cheat codes 0-20 are forwarded to the active board, higher codes are handled on the controller in controller_cheat()
     ("0-Meta",       Command::Cheat(20)),
     ("1-Meta",       Command::Cheat(21)),
     ("2-Meta",       Command::Cheat(22)),
     ("3-Meta",       Command::Cheat(23)),
     ("4-Meta",       Command::Cheat(24)),
     ("5-Meta",       Command::Cheat(25)),
     ("6-Meta",       Command::Cheat(26)),
     ("7-Meta",       Command::Cheat(27)),
     ("8-Meta",       Command::Cheat(28)),
     ("9-Meta",       Command::Cheat(29)),
];

impl Controller {
	fn active_id(&self) -> u32 { self.internal.borrow().active }
    pub fn initialize(&self, board_count: u32, width: u32, height: u32, preview: bool) {
		self.set_state(State::Initial);
        boards_reset();
        let container = &self.boards_container;
		while let Some(row) = container.last_child() {
			container.remove(&row);
		}
        for i in 0..board_count {
            let b = Board::new(i, width, height, preview);
            container.append(&b);
            boards_add(b);
        }
        self.summary_init(board_count);
        
		{ self.internal.borrow_mut().score = (0, 0); }
        self.total_points.set_label("0");
        self.total_lines.set_label("0");
		self.send_command(CMD_SELECT);
    }
	
	fn reinit(&self) {
		let rep = board(0).imp();
		self.initialize(boards_len() as u32, rep.width(), rep.height(), rep.show_preview());
	}
	
	fn toggle_state(&self) {
		let state = { self.internal.borrow().state };
		match state {
			State::Initial | State::Paused => self.set_state(State::Running),
			State::Running => self.set_state(State::Paused),
			State::Finished => self.reinit(),
		}
	}

	pub fn destroy(&self) { self.obj().destroy(); }
	
	fn set_state(&self, state: State) {
		if self.internal.borrow().state == state { return; }
		match state {
			State::Initial => {
				self.start_button.set_label("Start");
				self.options_button.show();
			},
			State::Paused => {
				self.options_button.hide();
				self.start_button.set_label("Continue");
				send_command_all(CMD_STOP);
				{ self.internal.borrow().clock.stop(); }
			},
			State::Running => {
				self.options_button.hide();
				self.start_button.set_label("Pause");
				send_command_all(CMD_START);
				{ self.internal.borrow().clock.start(); }
			},
			State::Finished => {
				self.options_button.show();
				send_command_all(CMD_STOP);
				self.start_button.set_label("New game");
				{ self.internal.borrow().clock.stop(); }
                self.summary_show();
			}
		}
		// in case Button grabbed it
		self.obj().grab_focus();
		self.internal.borrow_mut().state = state;
	}

    pub fn board_lost(&self, _board_id: u32) { self.set_state(State::Finished); }

    pub fn piece_crashed(&self, id: u32, points: u32, lines: u32, piece_num: u32) {
        {
            let mut internal = self.internal.borrow_mut();
            let old_score = internal.score;
            internal.score = (old_score.0 + points, old_score.1 + lines);
            self.total_points.set_label(&internal.score.0.to_string());
            self.total_lines.set_label(&internal.score.1.to_string());
        }
        self.summary_update(id, points, lines, piece_num);
    }

    pub fn mouse_click(&self, _id: u32, button: u32) { self.do_command(mouse_input(button)); }

	fn do_command(&self, command: Command) {
		{
			if !command.allowed(&self.internal.borrow().state) { return; }
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
				Command::SetBoard(new_id) => self.set_board(new_id),
				Command::Nop => (),
				Command::Cheat(code) => { if code < 20 {self.send_command(CMD_CHEAT | code)} else { self.controller_cheat(code); }},
			}
		}
	}

	pub fn set_board(&self, new_id: u32) {
		let old_id = self.active_id();
		if new_id >= boards_len() as u32 || new_id == old_id { return; }
		send_command_to(old_id, CMD_DESELECT);
		send_command_to(new_id, CMD_SELECT);
		self.internal.borrow_mut().active = new_id;
	}

	fn controller_cheat(&self, code: u32) {
		match code {
            21 => self.summary_show(),
			_ => println!("cheat code {}", code),
		}
	}

	fn send_command(&self, mask: u32) {
		let id = self.active_id();
		if id < boards_len() as u32 {
			let id_u32 = self.active_id();
			board(id).emit_by_name::<()>("board-command", &[&id_u32, &mask, ]);
		}
	}

	fn options(show: bool) {
		if show { crate::options_inst().show(); }
			else { crate::options_inst().hide(); }
	}

	fn tick(&self) {
        let mut internal = self.internal.borrow_mut();
		internal.seconds += 1;
		let time_str = format!("{:02}:{:02}", internal.seconds/60, internal.seconds % 60);
		self.time_disp.set_label(&time_str);
	}

    // accessors for Summary: I'd like to have a single accessor to the object summary(), but can't figure out how to
    // get ownership of a ref to the object. 
    fn summary_update(&self, id: u32, points: u32, lines: u32, piece_num: u32) {
        let internal = self.internal.borrow();
        internal.summary.as_ref().unwrap().imp().update_entry(id, points, lines, piece_num);
    }

    fn summary_init(&self, count: u32) {
        let internal = self.internal.borrow();
        internal.summary.as_ref().unwrap().imp().initialize(count);
    }
    fn summary_show(&self) {
        let internal = self.internal.borrow();
        internal.summary.as_ref().unwrap().imp().build_display();
        internal.summary.as_ref().unwrap().show();
    }
}

fn send_command_all(mask: u32) { for id in 0..boards_len() as u32 {send_command_to(id, mask); } }

fn send_command_to(id: u32, mask: u32) {
    if id < boards_len() as u32 {
        board(id).emit_by_name::<()>("board-command", &[&id, &mask, ]);
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

fn keyboard_input(key: gdk4::Key) -> Command {
    let key_string = modifier_bits_string(key.to_lower().name().unwrap().to_string());
//    println!("{:#?}", key_string);
    command_map_get(&key_string)
}

fn modifier_bits_string(mut key: String) -> String {
	let bits = controller_inst().internal.borrow().modifier_bits;
	if bits & ModifierType::SHIFT_MASK.bits() != 0 { key.push_str("-Shift"); }
	if bits & ModifierType::ALT_MASK.bits() != 0 { key.push_str("-Alt"); }
	if bits & ModifierType::CONTROL_MASK.bits() != 0 { key.push_str("-Ctrl"); }
	if bits & ModifierType::META_MASK.bits() != 0 { key.push_str("-Meta"); }
    key
}

// Alt doesn't work well, it changes the key. Maybe I should work with codes, but that would make customization harder
fn set_modifier(key: gdk4::Key, pressed: bool) {
	let name = key.to_lower().name().unwrap().to_string();
	let mask = match &name[..] {
		"Shift_L"   | "Shift_R"   => ModifierType::SHIFT_MASK.bits(),
		"Control_L" | "Control_R" => ModifierType::CONTROL_MASK.bits(),
		"Alt_L"     | "Alt_R"     => ModifierType::ALT_MASK.bits(),
		"Meta_L"    | "Meta_R"    => ModifierType::META_MASK.bits(),
		_ => return,
	};
	let mut internal = controller_inst().internal.borrow_mut();
	if pressed { internal.modifier_bits  |= mask; }
	else { internal.modifier_bits &= !mask; }
}

#[derive(Debug, Default)]
struct Clock {
	caller_count: Rc<Cell<u32>>,
}

impl Clock {
	fn new() -> Clock { Clock {caller_count: Rc::new(Cell::new(0)),}}

	fn start(&self) {
		let expected = self.caller_count.get();
		let caller_count = Rc::clone(&self.caller_count);
		let f = move || -> glib::Continue {
			if caller_count.get() > expected { return glib::Continue(false); }
			controller_inst().tick();
			glib::Continue(true)
		};
        glib::timeout_add_local(core::time::Duration::from_secs(1), f);
	}
	fn stop(&self) { self.caller_count.set(self.caller_count.get() + 1); }
}
