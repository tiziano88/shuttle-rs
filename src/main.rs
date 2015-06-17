use std::default;
use std::fs::File;
use std::io::Read;
use std::io;
use std::mem;

extern crate libudev;

#[derive(Default)]
#[repr(C)]
#[packed]
#[derive(Debug)]
struct InputEvent {
    tv_sec: isize,
    tv_usec: isize,
    type_: u16,
    code: u16,
    value: i32,
}

fn main() {
    let f = File::open("/dev/input/by-id/usb-Contour_Design_ShuttlePRO_v2-event-if00").unwrap();
    let mut r = io::BufReader::new(f);
    mem::size_of::<InputEvent>();

    // TODO: Use sizeof.
    let mut buf: [u8; 24] = unsafe { mem::zeroed() };

    loop {
        r.read(&mut buf);
        let e: InputEvent = unsafe { mem::transmute(buf) };
        let d = Event::from(&e);
        print!("{:?}\n", e);
        print!("{:?}\n", d);
    }
}

#[derive(Debug)]
enum Event {
    Button,
    Jog { v: i32 },
    Wheel { v: i32 },
}

impl<'a> std::convert::From<&'a InputEvent> for Event {
    fn from(ie: &'a InputEvent) -> Self {
        return match ie.code {
            7 => return Event::Jog{v: ie.value},
            8 => Event::Wheel{v: ie.value},
            _ => Event::Button,
        }
    }
}
