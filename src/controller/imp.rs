use gtk::prelude::*;
use gtk::subclass::prelude::*;
//use gtk::glib;
use gtk::prelude::GridExt;
use gtk::{glib, CompositeTemplate};
use gtk::glib::clone;
use crate::Board;
use gtk::glib::closure_local;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use once_cell::sync::OnceCell;
use std::cell::{RefCell, Cell};
use gdk4::ModifierType;
use std::rc::Rc;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {#[default] Paused, Running, Finished, }

//#[derive(Debug, Default)]
#[derive(Debug, Default, CompositeTemplate)]
#[template(file = "controller.ui")]
pub struct Controller {
    mut_vars: Rc<RefCell<MutVars>>,
    
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
struct MutVars {
    active: usize,
    score: (u32, u32),
    state: State,
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
        let x: i32 = 17;
        unsafe {
            controller.quit_buttonx.connect_clicked(clone!(@weak gcontroller => move |_| gcontroller.destroy()));
            let key_handler = gtk::EventControllerKey::new();
            controller.obj().add_controller(&key_handler);
            let mut_vars = Rc::clone(&self.mut_vars);
            key_handler.connect_key_pressed(move |_ctlr, key, _code, state| {
                do_command(&mut_vars, keyboard_input(key, state));
                gtk::Inhibit(false)
            });
        }
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
use crate::{CMD_LEFT, CMD_RIGHT, CMD_DOWN, CMD_CLOCKWISE, CMD_COUNTERCLOCKWISE, CMD_CHEAT};

impl Command {
    fn get_mask(&self) -> Option<u32> {
        match self {
            Command::Left             => Some(CMD_LEFT),
            Command::Right            => Some(CMD_RIGHT),
            Command::Down             => Some(CMD_DOWN), 
            Command::Clockwise        => Some(CMD_CLOCKWISE), 
            Command::CounterClockwise => Some(CMD_COUNTERCLOCKWISE),
            Command::Cheat(code)      => Some(CMD_CHEAT + code),
            _                         => None,
        }
    }
}

// default commands
const COMMANDS:[(&str, Command); 25] =
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

impl Controller {
    pub fn initialize(&self, board_count: u32, width: u32, height: u32, preview: bool) {
        unsafe {
            BOARDS.clear();
            let container = &self.boards_container;
            for i in 0..board_count {
                let b = Board::new(i, width, height, preview);
                b.connect_closure(
                    "board_report",
                    false,
                    closure_local!( |id: u32, score: u32, levels: u32| {
                        println!("board {} reports {} points, {} levels", id, score, levels);
                    }),
                );
                container.append(&b);
                BOARDS.push(b);
            }
        }
    }
}

fn do_command(mut_vars: &Rc<RefCell<MutVars>>, command: Command) {
    let mut id = { mut_vars.borrow().active};
    id = 1;
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
        Command::SetBoard(new_id) => { if new_id < board_count() { mut_vars.borrow_mut().active = new_id; }},
        Command::Nop => (),
    }
}

fn keyboard_input(key: gdk4::Key, modifiers: ModifierType) -> Command {
    let key_string = modifier_string(key.to_lower().name().unwrap().to_string(), modifiers.bits());
    unsafe {
        *COMMANDMAP.get(&key_string).unwrap_or(&Command::Nop)
    }
}

fn send_command(id: usize, mask: u32) {
    let id_u32 = id as u32;
    let extra = 4;
    if id < board_count() {
        unsafe {
            BOARDS[id].emit_by_name::<()>("board-command", &[&id_u32, &mask, ]);
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

fn board_count() -> usize {
    unsafe {
        BOARDS.len()
    }
}

