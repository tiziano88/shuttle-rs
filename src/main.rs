use std::default;
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

fn perform(mut state: State) -> Result<(), Box<Error>> {
    let f = try!(File::open("/dev/input/by-id/usb-Contour_Design_ShuttlePRO_v2-event-if00"));
    let mut r = io::BufReader::new(f);

    // mem::size_of::<InputEvent>();

    // TODO: Use sizeof.
    let mut buf = [0u8; 24];

    loop {
        r.read(&mut buf);
        let e: InputEvent = unsafe { mem::transmute(buf) };
        let d = Event::from(&e);
        match d {
            Event::Unknown => (),
            Event::Jog{v} => {
                if v > state.wheel {
                    xdotool(Action::ScrollDown);
                }
                if v < state.wheel {
                    xdotool(Action::ScrollUp);
                }
                state.wheel = v;
                print!("{:?}\n", d);
            },
            _ => {
                print!("{:?}\n", d);
            }
        }
    }
}

fn main() {
    let mut state = State{ wheel: 0 };
    if let Err(e) = perform(state) {
        write!(io::stderr(), "{}", e).unwrap();
    }
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
}

fn xdotool(a: Action) {
    let arg = match a {
        Action::ScrollUp => "4",
        Action::ScrollDown => "5",
    };
    let mut child = std::process::Command::new("xdotool")
        .arg("click")
        .arg(arg)
        .spawn()
        .unwrap();
    child.wait();
}
