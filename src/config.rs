
use std::env;
use clap::{Parser, value_parser};               // Command Line Argument Processing
use serde::{Deserialize, Serialize};            // config file parsing

//////////////////////////////////////////////////////////////////
//
// Config is the public structure for all configuration, the rest of the stuff is internal use
//
// The config comes from 3 places. First, the default values are set in the Config structure
// through crap macros. Next, the config file is read if it exists with those values overruling
// the defaults. Finally, the command line arguments overrule everything.
//
// That is the logic but not the implementation. CLAP merges the command line arguments with the
// defaults, and then the init file is parsed and the values used only if the matching command
// line switch is not set.
//
// I probably could implement this more generally with templates to make it reusable. Or maybe
// better yet find a crate that already does it. But this was goodp ractice in understanding
// ownership.
//
//////////////////////////////////////////////////////////////////

// default values, overridden by config file and command line switches
const BOARDS_DEFAULT:   u32 = 2;
const WIDTH_DEFAULT:    u32 = 10;
const HEIGHT_DEFAULT:   u32 = 20;
const CELLSIZE_DEFAULT: u16 = 20;
const DELAY_DEFAULT:    f64 = 0.05;
//const EXTENDED_DEFAULT: f64 = 0.03;
const PREVIEW_DEFAULT:  bool = true;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Config {
    #[clap(short, long, default_value_t = BOARDS_DEFAULT, value_parser=value_parser!(u32).range(0..5))]
    pub boards: u32,
    #[clap(short='W', long, default_value_t = WIDTH_DEFAULT, value_parser=value_parser!(u32).range(5..25))]
    pub width: u32,
    #[clap(short='H', long, default_value_t = HEIGHT_DEFAULT, value_parser=value_parser!(u32).range(10..40))]
    pub height: u32,
    #[clap(short='C', long, default_value_t = CELLSIZE_DEFAULT, value_parser=value_parser!(u16).range(10..50))]
    pub cell_size: u16,
    #[clap(short, long, default_value_t = DELAY_DEFAULT)]
    pub delay: f64,
//    #[clap(short, long, default_value_t = EXTENDED_DEFAULT)]
//    pub extended_chance: f64,
    #[clap(short, long, default_value_t = PREVIEW_DEFAULT)]
    pub preview: bool,
    #[clap(short, long, default_value_t = String::from("~/.tetrii"))]
    pub config_file: String,
    #[clap(short, long, default_value_t = String::from("style.css"))]
    pub style: String,
//    #[clap(short, long, default_value_t = 999)]
//    pub initial_piece: usize,
}

// Config is the configuration used for the game. It is immutable once fully initialized.
// Values can come from 3 places. Values set on the command line take priority; next are
// values from the (optional) config file, and finally there are the default values set
// in the Config structure.
impl Config {
    pub fn build_config() -> Config {
        // command line arguments 
        let mut config = Config::parse();
        ConfigOptions::merge_into(&mut config);
        config.check_values();
        config
    }

    pub fn save(&self, filename: &str) -> Result<String, String> {
        ConfigOptions::from_config(self).save(filename)
    }

    // CLAP coes check for these from the command line, but this checks config file as well.
    fn check_values(&self) {
        assert!(1 <= self.boards && self.boards <= 5, "Number of boards must be between 1 and 5");
        assert!(8 <= self.width && self.width <= 28, "Board width must be between 8 and 28");
        assert!(10 <= self.height && self.height <= 40, "Board height must be between 10 and 40");
    }
}

//////////////////////////////////////////////////////////////////
//
// Following are internal use
//
//////////////////////////////////////////////////////////////////

// This struct mirrors the Config structure, but is needed for accessing the config file. The Option is needed
// to handle values which might be missing or null. Also, mapping the names allows the yaml file arguments to
// be camel case, which I prefer. That is why the non_snake_case warning is disabled for this struct.
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct ConfigOptions {
    Boards:         Option<u32>,
    Width:          Option<u32>,
    Height:         Option<u32>,
    CellSize:       Option<u16>,
    Delay:          Option<f64>,
//    ExtendedChance: Option<f64>,
    Preview:        Option<bool>,
//    InitialPiece:   Option<usize>,
    Style:          Option<String>,
}

impl ConfigOptions {
    // creates a ConfigOptions from a Config. This is needed so that values written to the config file will
    // have the right names.
    fn from_config(config: &Config) -> ConfigOptions {
        ConfigOptions{ Boards:         Some(config.boards),
                       Width:          Some(config.width),
                       Height:         Some(config.height),
                       CellSize:       Some(config.cell_size),
                       Delay:          Some(config.delay),
//                       ExtendedChance: Some(config.extended_chance),
                       Preview:        Some(config.preview),
//                       InitialPiece:   Some(config.initial_piece),
                       Style:          Some(config.style.to_string()),
        }
    }

    // dumps a configuration to a file in yaml format
    fn save(&self, filename: &str) -> Result<String, String> {
        let expanded_name = expand_filename(filename);
        let file = match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&expanded_name) {
                Ok(f) => f,
                Err(err) => { return Err("Failed opening config file for write: ".to_string() + &expanded_name + ": " + &err.to_string()); },
            };
        match serde_yaml::to_writer(file, self) {
            Ok(_) => Ok("Wrote config file ".to_string() + &expanded_name),
            Err(err) => Err("Error writing config file ".to_string() + &expanded_name + ": " + &err.to_string())
        }
    }

    // Here the config structure is made from combining the command line arguments and the default values.
    // Values in the config file should replace default values but not affect values set from the command
    // line. This method replaces any default values in CONFIG with values from the yaml config file.
    fn merge_into(config: &mut Config) {
        // try reading yaml file if nonzero length. In case of error ignore config file and use defaults and command line config
        let mut config_file = expand_filename(&config.config_file);
        
        if !config_file.is_empty() {
            // read file as String
            let read_result = std::fs::read_to_string(&mut config_file);
            let read_string = match read_result {
                Ok(string) => string,
                Err(err) => {
                    eprintln!("Could not open config file `{}`: {}\nproceeding without it", &config_file, err);
                    return;
                },
            };
            let yaml_result:Result<ConfigOptions, serde_yaml::Error> = serde_yaml::from_str(&read_string);
            let mut yaml_options = match yaml_result {
                Ok(opts) => opts,
                Err(err) => {
                    eprintln!("Error parsing config file {}: {}\n   Ignoring config file", &config_file, err);
                    return;
                },
            };
            // merge the config file arguments into the config structure. If an arg is given on the command line ignore
            // the config file version, otherwise override the default value
            for arg in env::args() {
                match arg.as_str() {
                    "-b" | "--boards"          => yaml_options.Boards         = None,
                    "-W" | "--width"           => yaml_options.Width          = None,
                    "-H" | "--height"          => yaml_options.Height         = None,
                    "-C" | "--cell_size"       => yaml_options.CellSize       = None,
                    "-d" | "--delay"           => yaml_options.Delay          = None,
//                    "-e" | "--extended_chance" => yaml_options.ExtendedChance = None,
                    "-p" | "--preview"         => yaml_options.Preview        = None,
                    "-s" | "--style"           => yaml_options.Style          = None,
                    _                          => (),
                };
            }
            if yaml_options.Boards.is_some()         { config.boards          = yaml_options.Boards.unwrap(); }
            if yaml_options.Width.is_some()          { config.width           = yaml_options.Width.unwrap(); }
            if yaml_options.Height.is_some()         { config.height          = yaml_options.Height.unwrap(); }
            if yaml_options.CellSize.is_some()       { config.cell_size       = yaml_options.CellSize.unwrap(); }
            if yaml_options.Delay.is_some()          { config.delay           = yaml_options.Delay.unwrap(); }
//            if yaml_options.ExtendedChance.is_some() { config.extended_chance = yaml_options.ExtendedChance.unwrap(); }
            if yaml_options.Preview.is_some()        { config.preview         = yaml_options.Preview.unwrap(); }
//            if yaml_options.InitialPiece.is_some()   { config.initial_piece   = yaml_options.InitialPiece.unwrap(); }
            if yaml_options.Style.is_some()          { config.style           = expand_filename(&yaml_options.Style.unwrap()); }
        }
    }
}


// for now just move ~/xxx to ${HOME}/xxx
fn expand_filename(name: &str) -> String {
    // allow ~/ for home directory
    if name[0..2].eq("~/") {
        let home = dirs::home_dir();
        let mut home_str = home.unwrap().to_str().unwrap().to_string();
        home_str.push_str(&name[1..]);
        home_str
    } else {
        name.to_string()
    }
}

