#![feature(mpsc_select)]

extern crate rustc_serialize;
extern crate toml;

use rustc_serialize::{Encodable, Decodable};
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::io;
use std::mem;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::vec;

extern crate libudev;

struct State {
    wheel: i32,
}

#[repr(C)]
#[derive(Debug, Clone)]
struct InputEvent {
    tv_sec: isize,
    tv_usec: isize,
    type_: u16,
    code: u16,
    value: i32,
}

#[derive(RustcEncodable,RustcDecodable)]
#[derive(Debug, Clone)]
struct ConfigGeneral {
    device: String,
}

#[derive(RustcEncodable,RustcDecodable)]
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
struct Config {
    general: ConfigGeneral,
    map: Vec<ConfigMap>,
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

fn after_ms(t: u32) -> mpsc::Receiver<()> {
    let (tx, rx) = mpsc::channel::<()>();
    thread::spawn(move || {
        thread::sleep_ms(t);
        tx.send(()).unwrap();
    });
    rx
}

fn every_ms(t: u32) -> mpsc::Receiver<()> {
    let (tx, rx) = mpsc::channel::<()>();
    thread::spawn(move || {
        loop {
            thread::sleep_ms(t);
            tx.send(()).unwrap();
        }
    });
    rx
}

// TODO: Avoid `config` clone + move.
fn background(rx: mpsc::Receiver<Event>, config: Config) -> () {
    thread::spawn(move || {
        let mut count = 0;
        let mut target = 0;

        // XXX: after_ms not working.
        let tick_rx = every_ms(10);

        loop {
            select! {
                e = rx.recv() => {
                    println!("t: {:?}", e);
                    if let Event::Shuttle{v} = e.unwrap() {
                        target = 10 / v;
                        // TODO: Detect zero.
                        if v.abs() == 1 {
                            target = 0;
                        }
                    }
                },
                // XXX: Strict syntax for macros?
                _ = tick_rx.recv() => {
                    if target != 0 {
                        println!("count: {:?}", count);
                        count += 1;
                        if count >= target.abs() {
                            // XXX: Use current_map.
                            let ref map = config.map[0];
                            let mut action_string = &Option::None;
                            if target > 0 {
                                action_string = &map.shuttle_down;
                            }
                            if target < 0 {
                                action_string = &map.shuttle_up;
                            }
                            count = 0;
                            if let &Some(ref a) = action_string {
                                // TODO: try!
                                exec(a);
                            }
                        }
                    }
                }
            }
        }
    });
}

fn perform() -> Result<(), Box<Error>> {
    let mut state = State{ wheel: 0 };

    let config_file_name = "/home/tzn/.wheel.toml";
    let config: Config = try!(load_config_from_file(config_file_name));
    println!("config: {:?}", config);

    let current_map = Box::new(&config.map[0]);

    let (tx, rx) = mpsc::channel::<Event>();
    background(rx, config.clone());

    let f = try!(File::open(&config.general.device));
    let mut r = io::BufReader::new(f);

    // mem::size_of::<InputEvent>();

    // TODO: Use sizeof.
    let mut buf = [0u8; 24];

    loop {
        try!(r.read(&mut buf));
        let input_event: InputEvent = unsafe { mem::transmute(buf) };
        let mut action_string = &Option::Some("".to_string()); // XXX
        let event = Event::from(&input_event);
        print!("{:?}\n", event);
        // XXX
        tx.send(event.clone()).unwrap();
        match event {
            Event::Unknown => (),
            Event::Jog{v} => {
                if v > state.wheel {
                    action_string = &current_map.jog_down
                }
                if v < state.wheel {
                    action_string = &current_map.jog_up
                }
                state.wheel = v;
            },
            Event::Shuttle{v} => {
                if v > 0 {
                    action_string = &current_map.shuttle_down
                }
                if v < 0 {
                    action_string = &current_map.shuttle_up
                }
            },
            Event::Button{v} => {
                match v {
                    269 => action_string = &current_map.button_left,
                    270 => action_string = &current_map.button_right,
                    _ => {},
                }
            },
        }
        if let &Some(ref a) = action_string {
            try!(exec(a));
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

#[derive(Copy, Clone, Debug)]
enum Event {
    Unknown,
    Button { v: u16 },
    Jog { v: i32 }, // Endless.
    Shuttle { v: i32 }, // Springy.
}

impl<'a> std::convert::From<&'a InputEvent> for Event {
    fn from(ie: &'a InputEvent) -> Self {
        println!("input: {:?}", ie);
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

