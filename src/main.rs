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
        let inputEvent: InputEvent = unsafe { mem::transmute(buf) };
        let &mut actionString = &Option::Some("ls".to_string()); // XXX
        let event = Event::from(&inputEvent);
        print!("{:?}\n", event);
        match event {
            Event::Unknown => (),
            Event::Jog{v} => {
                if v > state.wheel {
                    actionString = currentMap.jog_down
                }
                if v < state.wheel {
                    actionString = currentMap.jog_up
                }
                state.wheel = v;
            },
            Event::Shuttle{v} => {
                if v > 0 {
                    actionString = currentMap.shuttle_down
                }
                if v < 0 {
                    actionString = currentMap.shuttle_up
                }
            },
            Event::Button{v} => {
                match v {
                    269 => actionString = Some("xdotool key Home".to_string()),
                    270 => actionString = Some("xdotool key End".to_string()),
                    _ => {},
                }
            },
        }
        if let Some(ref a) = actionString {
            exec(a);
        }
    }
}

fn exec(a: &str) -> Result<(), Box<Error>> {
    let mut child = try!(std::process::Command::new("/bin/bash")
                         .arg("-c")
                         .arg(a)
                         .spawn());
    try!(child.wait());
    Ok(())
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

