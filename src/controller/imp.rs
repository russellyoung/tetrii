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

//////////////////////////////////////////////////////////////////
//
// STATICS
//
// I don't like statics in any language - they often are a lazy man's solution. Still, sometimes they are needed,
// and these are the ones that either I can't avoid or are too much work to avoid (LMS). I also don't like using
// UNSAFE. The name gives the impression, I'm sure intentionally, that it is not encouraged. So, here also are
// accessor functions to move all the UNSAFEs out of the rest of the code.
//
// By using statics here for stuff that really belongs in the Controller I'm able to use functions for callbacks
// rather than pass the controller around everywhere, getting "needs static lifetime" and "can't borrow" errors.
// The accessors do no checking, they are for internal use.
//
//////////////////////////////////////////////////////////////////
static mut CONTROLLER: Option<crate::controller::Controller> = None;

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

static mut BOARDS: Lazy<Vec<Board>> = Lazy::new(|| Vec::new());
// Timer handling is a little trick, since each board can have two running at once, and more can be started before one or both of them has
// ended. The TickTimer object has its own flag to signal quit, and the Drop timer links to its Step timer so both stop.
static mut OLD_TIMERS: Option<Vec<Timer>> = None;
static mut TIMERS: Option<Vec<StepTimer>> = None;

static mut COMMANDMAP: Lazy<HashMap<String, Command>> = Lazy::new(|| {
    let mut hashmap: HashMap<String, Command> = HashMap::new();
    COMMANDS.iter().for_each(|desc| { hashmap.insert(desc.0.to_string(), desc.1); });
    hashmap
});

// BOARD accessors
fn boards_len() -> usize { unsafe { BOARDS.len() } }
fn board(which: usize) -> &'static Board { unsafe { &BOARDS[which] } }
fn boards_reset() { unsafe { BOARDS.clear(); }}
fn boards_add(board: Board) { unsafe { BOARDS.push(board); }}

fn command_map_get(key: &String) -> Command { unsafe {*COMMANDMAP.get(key).unwrap_or(&Command::Nop)}}

fn timers(board_id: usize) { //-> &'static mut StepTimer {
	unsafe {
		//&TIMERS.as_mut().unwrap().as_mut_slice()[board_id as usize]
	}
}
fn timers_add(mut timer: StepTimer) { unsafe { TIMERS.as_mut().unwrap().push(timer);}}
fn timers_reset() {
	old_timers_reset();
	unsafe { TIMERS = Some(Vec::new()); }
}

fn old_timers_add(timer: Timer) { unsafe { OLD_TIMERS.as_mut().unwrap().push(timer); }}
fn old_timers_reset() { unsafe {if OLD_TIMERS.is_none() { OLD_TIMERS = Some(Vec::new()); }}}
// my very own GC
fn old_timers_clean() {
	unsafe {
		let old_timers = OLD_TIMERS.as_mut().unwrap();
		let len = old_timers.len();
		for i in (0..len).rev() {
			if !old_timers[i].running() {
				old_timers.swap_remove(i);
			}
		}
	}
}

//
// end STATIC
//

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
        self.quit_buttonx.connect_clicked(clone!(@weak gcontroller => move |_| gcontroller.destroy()));
        self.start_buttonx.connect_clicked( |button| { controller().toggle_state(); });
        let key_handler = gtk::EventControllerKey::new();
        self.obj().add_controller(&key_handler);
        let internal = Rc::clone(&self.internal);
        key_handler.connect_key_pressed(move |_ctlr, key, _code, mods| {
			set_modifier(key, true);
            controller().do_command(keyboard_input(key, mods));
            gtk::Inhibit(false)
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
use crate::{CMD_LEFT, CMD_RIGHT, CMD_DOWN, CMD_CLOCKWISE, CMD_COUNTERCLOCKWISE, CMD_SELECT, CMD_DESELECT, CMD_CHEAT};

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
	fn start(&self) {
		println!("hi");
	}
    pub fn initialize(&self, board_count: u32, width: u32, height: u32, preview: bool) {
		self.set_state(State::Initial);
        boards_reset();
		timers_reset();
		
        let container = &self.boards_container;
        for i in 0..board_count {
            let b = Board::new(i, width, height, preview);
            container.append(&b);
            boards_add(b);
			timers_add(StepTimer::new(i, STARTING_TICK_MS, height));
        }
		self.send_command(CMD_SELECT);
    }

	fn toggle_state(&self) {
		match self.internal.borrow().state {
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
				StepTimer::stop_all();
			},
			State::Running => {
				self.obj().grab_focus();
				self.start_buttonx.set_label("Pause");
				StepTimer::start_all();
			},
			State::Finished => self.start_buttonx.set_visible(false),
		}
		self.internal.borrow_mut().state = state;
	}
    pub fn board_lost(&self, board_id: u32) { StepTimer::stop_all(); }

    pub fn piece_crashed(&self, board_id: u32, points: u32, lines: u32) {
		let board_id: usize = board_id as usize;
		unsafe {
			TIMERS.as_mut().unwrap()[board_id].stop();
			//			timers(i).start();
		}
        let mut internal = self.internal.borrow_mut();
        internal.dropping &= !0x1 << board_id;
        let old_score = internal.score;
        internal.score = (old_score.0 + points, old_score.1 + lines);
        self.total_points.set_label(&internal.score.0.to_string());
        self.total_lines.set_label(&internal.score.1.to_string());
		unsafe {
			TIMERS.as_mut().unwrap()[board_id].start();
			//			timers(i).start();
		}
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
			Command::Drop => self.do_drop(),
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

	fn do_drop(&self) {
		unsafe { TIMERS.as_mut().unwrap()[self.active_id()].drop(); } }

	fn send_command(&self, mask: u32) {
		let id = self.active_id();
		if id < boards_len() {
			let id_u32 = self.active_id() as u32;
			board(id).emit_by_name::<()>("board-command", &[&id_u32, &mask, ]);
		}
	}
}

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

//////////////////////////////////////////////////////////////////
//
// Timers
//
//////////////////////////////////////////////////////////////////




struct Timer {
	id: usize,
	quit_count: Rc<Cell<i32>>,
}

impl Timer {
	fn new(board_id: u32, quit_count: i32) -> Timer { Timer {id: board_id as usize, quit_count: Rc::new(Cell::new(quit_count)), }}
	fn start(&self, msecs: u32) {
		let quit_count = Rc::clone(&self.quit_count);
		let id = self.id;
		let f = move || -> glib::Continue {
			if quit_count.get() <= 0 { return glib::Continue(false); }
			send_command_to(id, CMD_DOWN);
			quit_count.set(quit_count.get() - 1);
			glib::Continue(true)
		};
        glib::timeout_add_local(core::time::Duration::from_millis(msecs as u64), f);
	}

	fn stop(&self) { self.quit_count.set(0); }
	fn running(&self) -> bool { self.quit_count.get() > 0 }
}

struct StepTimer {
	board_id: u32,
	step_timer_v: Vec<Timer>,
	drop_timer_v: Vec<Timer>,
	msecs: u32,
	watchdog: i32,
}

impl StepTimer {
	fn start_all() {
		for i in 0..boards_len() {
			unsafe {
				TIMERS.as_mut().unwrap()[i].start();
				//			timers(i).start();
			}
		}
	}
	fn stop_all() {
		for i in 0..boards_len() {
			unsafe {
				TIMERS.as_mut().unwrap()[i].stop();
				//			timers(i).start();
			}
		}
	}
	
	fn new(board_id: u32, msecs: u32, height: u32) -> StepTimer {
		StepTimer {board_id: board_id,
				   msecs: msecs,
				   watchdog: height as i32,
				   step_timer_v: Vec::new(),
				   drop_timer_v: Vec::new(), }
	}

	fn stop(&mut self) {
		self.stop_step_timer();
		self.stop_drop_timer();
	}

	fn stop_drop_timer(&mut self) {
		if self.drop_timer_v.len() > 0 {
			if self.drop_timer_v[0].running() {
				self.drop_timer_v[0].stop();
			}
			old_timers_add(self.drop_timer_v.pop().unwrap());
		}
	}
	
	fn stop_step_timer(&mut self) {
		if self.step_timer_v.len() > 0 {
			if self.step_timer_v[0].running() {
				self.step_timer_v[0].stop();
			}
			old_timers_add(self.step_timer_v.pop().unwrap());
		}
	}

	fn start(&mut self) {
		self.stop_step_timer();
		self.stop_drop_timer();
		self.step_timer_v.push(Timer::new(self.board_id, self.watchdog));
		self.step_timer_v[0].start(self.msecs);
	}

	fn drop(&mut self) {
		self.stop_step_timer();
		self.stop_drop_timer();
		self.drop_timer_v.push(Timer::new(self.board_id, self.watchdog));
		self.drop_timer_v[0].start(self.msecs/DROP_RATIO);
	}

	fn speedup(&mut self) { self.msecs = (self.msecs as f64 * SLOWDOWN_RATIO) as u32; }
}

