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
    let mut f = io::BufReader::new(File::open("/dev/input/by-id/usb-Contour_Design_ShuttlePRO_v2-event-if00").unwrap());
    mem::size_of::<InputEvent>();

    // TODO: Use sizeof.
    let mut buf: [u8; 24] = unsafe { mem::zeroed() };

    while true {
        f.read(&mut buf);
        let e: InputEvent = unsafe { mem::transmute(buf) };
        print!("{:?}\n", e);
    }
}
