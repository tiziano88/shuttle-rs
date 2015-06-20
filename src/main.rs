extern crate rustc_serialize;
extern crate toml;

use rustc_serialize::{Encodable, Decodable};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io;
use std::io::Write;
use std::mem;

extern crate libudev;

struct State {
    wheel: i32,
}

#[repr(C)]
#[derive(Debug)]
struct InputEvent {
    tv_sec: isize,
    tv_usec: isize,
    type_: u16,
    code: u16,
    value: i32,
}

#[derive(RustcEncodable,RustcDecodable)]
#[derive(Debug)]
struct ConfigGeneral {
    device: String,
}

#[derive(RustcEncodable,RustcDecodable)]
#[derive(Debug)]
struct ConfigMap {
    jog_up: Option<String>,
    jog_down: Option<String>,
    shuttle_up: Option<String>,
    shuttle_down: Option<String>,
    button_left: Option<String>,
    button_right: Option<String>,
    button_1: Option<String>,
    button_2: Option<String>,
}

#[derive(RustcEncodable,RustcDecodable)]
#[derive(Debug)]
struct Config {
    general: ConfigGeneral,
    map: [ConfigMap; 2],
}

fn load_config_from_file(config_file_name: &str) -> Result<Config, Box<Error>> {
    let mut config_file = try!(File::open(config_file_name));
    let mut config_file_content = String::new();
    try!(config_file.read_to_string(&mut config_file_content));

    let config_table = toml::Value::Table(toml::Parser::new(&config_file_content).parse().unwrap());
    println!("{:?}", config_table);

    let mut d = toml::Decoder::new(config_table);
    let config: Config = try!(Decodable::decode(&mut d));
    Ok(config)
}

fn perform() -> Result<(), Box<Error>> {
    let mut state = State{ wheel: 0 };

    let config_file_name = "/home/tzn/.wheel.toml";
    let config: Config = try!(load_config_from_file(config_file_name));
    println!("config: {:?}", config);

    let mut currentMap = &config.map[0];

    let f = try!(File::open(config.general.device));
    let mut r = io::BufReader::new(f);

    // mem::size_of::<InputEvent>();

    // TODO: Use sizeof.
    let mut buf = [0u8; 24];

    loop {
        try!(r.read(&mut buf));
        let e: InputEvent = unsafe { mem::transmute(buf) };
        let d = Event::from(&e);
        match d {
            Event::Unknown => (),
            Event::Jog{v} => {
                if v > state.wheel {
                    // XXX
                    action(currentMap.jog_down.unwrap_or("".to_string()));
                    try!(xdotool(Action::ScrollDown));
                }
                if v < state.wheel {
                    try!(xdotool(Action::ScrollUp));
                }
                state.wheel = v;
                print!("{:?}\n", d);
            },
            Event::Button{v} => {
                match v {
                    269 => try!(xdotool(Action::Home)),
                    270 => try!(xdotool(Action::End)),
                    _ => {},
                }
                print!("{:?}\n", d);
            },
            _ => {
                print!("{:?}\n", d);
            }
        }
    }
}

fn action(a: &str) {
}

fn main() {
    perform()
        .or_else(|e| write!(io::stderr(), "{}", e))
        .unwrap();
}

#[derive(Debug)]
enum Event {
    Unknown,
    Button { v: u16 },
    Jog { v: i32 }, // Endless.
    Shuttle { v: i32 }, // Springy.
}

impl<'a> std::convert::From<&'a InputEvent> for Event {
    fn from(ie: &'a InputEvent) -> Self {
        match ie.type_ {
            1 => match ie.value {
                1 => Event::Button{v: ie.code},
                _ => Event::Unknown,
            },
            2 => match ie.code {
                7 => Event::Jog{v: ie.value},
                8 => Event::Shuttle{v: ie.value},
                _ => Event::Unknown,
            },
            _ => Event::Unknown,
        }
    }
}

enum Action {
    ScrollUp,
    ScrollDown,
    Home,
    End,
}

fn xdotool(a: Action) -> Result<(), Box<Error>> {
    let args = match a {
        Action::ScrollUp => ["click", "4"],
        Action::ScrollDown => ["click", "5"],
        Action::Home => ["key", "Home"],
        Action::End => ["key", "End"],
    };
    let mut child = try!(std::process::Command::new("xdotool")
        .args(&args)
        .spawn());
    try!(child.wait());
    Ok(())
}
