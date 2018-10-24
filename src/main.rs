#[macro_use]
extern crate chan;
extern crate failure;
extern crate libudev;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use failure::Error;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::mem;
use std::sync::Arc;
use std::thread;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ConfigGeneral {
    device: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ConfigMap {
    jog_up: Option<String>,
    jog_down: Option<String>,

    shuttle_up: Option<String>,
    shuttle_down: Option<String>,

    // First row.
    button_1: Option<String>,
    button_2: Option<String>,
    button_3: Option<String>,
    button_4: Option<String>,

    // Second row.
    button_5: Option<String>,
    button_6: Option<String>,
    button_7: Option<String>,
    button_8: Option<String>,
    button_9: Option<String>,

    button_left: Option<String>,
    button_right: Option<String>,

    // Bottom rows.
    button_10: Option<String>,
    button_11: Option<String>,
    button_12: Option<String>,
    button_13: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    general: ConfigGeneral,
    map: Vec<ConfigMap>,
}

fn load_config_from_file(config_file_name: &str) -> Result<Config, Error> {
    let mut config_file = File::open(config_file_name)?;
    let mut config_file_content = String::new();
    config_file.read_to_string(&mut config_file_content)?;
    let config: Config = toml::from_str(&config_file_content)?;
    Ok(config)
}

// TODO: Avoid `config` clone + move.
fn background(rx: chan::Receiver<Event>, config: Arc<Config>) -> () {
    thread::spawn(move || {
        let mut count = 0;
        let mut target = 0;

        // XXX: after_ms not working.
        let tick_rx = chan::tick_ms(10);

        loop {
            if target == 0 {
                // TODO: Find more elegant approach.
                chan_select! {
                    rx.recv() -> e => {
                        println!("t: {:?}", e);
                        if let Event::Shuttle{v} = e.unwrap() {
                            target = 10 / v;
                            // TODO: Detect zero.
                            if v.abs() == 1 {
                                target = 0;
                            }
                        }
                    },
                }
            } else {
                chan_select! {
                    rx.recv() -> e => {
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
                    tick_rx.recv() -> _ => {
                        if target != 0 {
                            print!("{:?}", count);
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
        }
    });
}

fn perform() -> Result<(), Error> {
    let mut state = State { wheel: 0 };

    // TODO: Handle errors.
    let home = std::env::home_dir().unwrap();
    let config_file_name = format!("{}/.wheel.toml", home.display());
    let config = Arc::new(load_config_from_file(&config_file_name)?);
    println!("config: {:?}", config);

    let mut current_map = Box::new(&config.map[0]);

    let (tx, rx) = chan::sync(0);
    background(rx, config.clone());

    let f = File::open(&config.general.device)?;
    let mut r = io::BufReader::new(f);

    // mem::size_of::<InputEvent>();

    // TODO: Use sizeof.
    let mut buf = [0u8; 24];

    loop {
        r.read(&mut buf)?;
        let input_event: InputEvent = unsafe { mem::transmute(buf) };
        let mut action_string = &Option::Some("".to_string()); // XXX
        let event = Event::from(&input_event);
        print!("{:?}\n", event);
        tx.send(event.clone());
        match event {
            Event::Unknown => (),
            Event::Jog { v } => {
                if v > state.wheel {
                    action_string = &current_map.jog_down
                }
                if v < state.wheel {
                    action_string = &current_map.jog_up
                }
                state.wheel = v;
            }
            Event::Shuttle { v } => {
                if v > 0 {
                    action_string = &current_map.shuttle_down
                }
                if v < 0 {
                    action_string = &current_map.shuttle_up
                }
            }
            Event::Button { v } => match v {
                256 => {
                    action_string = &current_map.button_1;
                    current_map = Box::new(&config.map[0]);
                }
                257 => {
                    action_string = &current_map.button_2;
                    current_map = Box::new(&config.map[1]);
                }
                258 => {
                    action_string = &current_map.button_3;
                    current_map = Box::new(&config.map[2]);
                }
                259 => {
                    action_string = &current_map.button_4;
                    current_map = Box::new(&config.map[3]);
                }

                260 => action_string = &current_map.button_5,
                261 => action_string = &current_map.button_6,
                262 => action_string = &current_map.button_7,
                263 => action_string = &current_map.button_8,
                264 => action_string = &current_map.button_9,

                265 => action_string = &current_map.button_10,
                266 => action_string = &current_map.button_11,
                267 => action_string = &current_map.button_12,
                268 => action_string = &current_map.button_13,

                269 => action_string = &current_map.button_left,
                270 => action_string = &current_map.button_right,
                _ => {}
            },
        }
        if let &Some(ref a) = action_string {
            exec(a)?;
        }
    }
}

fn exec(a: &str) -> Result<(), Error> {
    let mut child = std::process::Command::new("/bin/bash")
        .arg("-c")
        .arg(a)
        .spawn()?;
    child.wait()?;
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
    Jog { v: i32 },     // Endless.
    Shuttle { v: i32 }, // Springy.
}

impl<'a> std::convert::From<&'a InputEvent> for Event {
    fn from(ie: &'a InputEvent) -> Self {
        println!("input: {:?}", ie);
        match ie.type_ {
            1 => match ie.value {
                1 => Event::Button { v: ie.code },
                _ => Event::Unknown,
            },
            2 => match ie.code {
                7 => Event::Jog { v: ie.value },
                8 => Event::Shuttle { v: ie.value },
                _ => Event::Unknown,
            },
            _ => Event::Unknown,
        }
    }
}
