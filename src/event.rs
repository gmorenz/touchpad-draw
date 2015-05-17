use std;

use libc::c_int as int;
use std::sync::mpsc::{Receiver, channel};

// A temporary "hack" for convenience
const size_divisor: u32 = 2;

#[allow(non_camel_case_types, dead_code)]
enum EventType {
    Syn = 0x00,
    Key = 0x01,
    Rel = 0x02,
    Abs = 0x03,
    Msc = 0x04,
    Sw  = 0x05,
    Led = 0x11,
    Snd = 0x12,
    Rep = 0x14,
    Ff  = 0x15,
    fStatus = 0x17,
    Max = 0x1f,
    Cnt = 0x1f + 1,
}

#[derive(Debug)]
#[allow(dead_code)]
enum AbsEventCode {
    X = 0x00,
    Y = 0x01,
    Pressure = 0x18,
    ToolWidth = 0x1c,
    MtSlot = 0x2f,
    MtTrackingId = 0x39,
    MtPositionX = 0x35,
    MtPositionY = 0x36,
    MtPressure = 0x3a,
    Unknown,
}

impl AbsEventCode {
    pub fn from_raw( raw: u16 ) -> AbsEventCode {
        use self::AbsEventCode::*;
        match raw {
            0 => X,
            1 => Y,
            0x18 => Pressure,
            0x1c => ToolWidth,
            0x2f => MtSlot,
            0x39 => MtTrackingId,
            0x35 => MtPositionX,
            0x36 => MtPositionY,
            0x3a => MtPressure,
            _ => panic!("Unkmown code: {0:x}", raw)
        }
    }
}

impl EventType {
    pub fn from_raw( raw: u16 ) -> EventType {
        use std::mem::transmute as t;
        if ( raw > 0x5 && raw < 0x11 ) || ( raw == 0x13 ) || (raw == 0x16) 
            || (raw > 0x17 && raw < 0x1f) || ( raw > 0x1f + 0x1) {
            panic!("Invalid EventType: {}", raw);
        }
        
        unsafe{ t(raw as u8) }
    }
}

/* Emulating C structs */
#[repr(C)]
#[derive(Debug)]
struct timeval {
	tv_sec: i64,		/* seconds */
	tv_usec: i64,	/* microseconds */
}

#[repr(C)]
#[derive(Debug)]
struct InputEvent {
	time: timeval,
	tipe: u16,
	code: u16,
	value: i32,
}

#[repr(C)]
#[derive(Debug)]
struct input_absinfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32
}

/* Emulating C Macros */
#[allow(non_snake_case)]
fn _IOC( dir: int, typ: int, nr: int, size: int ) -> int {
    let _IOC_NRBITS = 8;
    let _IOC_TYPEBITS = 8;
    let _IOC_SIZEBITS = 14;
    //let _IOC_DIRBITS = 2;
    
    let _IOC_NRSHIFT = 0;
    let _IOC_TYPESHIFT = _IOC_NRSHIFT + _IOC_NRBITS;
    let _IOC_SIZESHIFT = _IOC_TYPESHIFT + _IOC_TYPEBITS;
    let _IOC_DIRSHIFT = _IOC_SIZESHIFT + _IOC_SIZEBITS;

    (dir << _IOC_DIRSHIFT) |
        (typ << _IOC_TYPESHIFT) |
        (nr << _IOC_NRSHIFT ) |
        (size << _IOC_SIZESHIFT)
}

#[allow(non_snake_case)]
fn _IOC_TYPECHECK<T>(_: T) -> int {
    std::mem::size_of::<T>() as int
}

#[allow(non_snake_case)]
fn _IOR<T>( typ: int, nr: int, size: T ) -> int {
    _IOC( 2 /* _IOC_READ */, typ, nr, _IOC_TYPECHECK(size))
}

#[allow(non_snake_case)]
fn EVIOCGBIT( ev: int, len: int ) -> int {
    _IOC( 2 /* _IOC_READ */, 'E' as int, 0x20 + ev, len)
}

#[allow(non_snake_case)]
unsafe fn EVIOCGABS( abs: int ) -> int {
    use std::mem::uninitialized;
    _IOR('E' as int, 0x40 + abs, uninitialized::<input_absinfo>())
}

extern {
    fn read(fd: int, buf: *mut u8, len: int ) -> int;
    fn perror(string: int /* *const u8 */); 
    fn open(filename: *const i8, options: int) -> int;
    /* EVIOCGBIT version of ioctl */
    fn ioctl(fd: int, request: int, bitfield: *mut u8) -> int;
}

unsafe fn has_abs(fd: int) -> bool {
    let mut ret: u8 = 0;
    let res = ioctl(fd, EVIOCGBIT( EventType::Abs as int, 1), &mut ret as *mut u8);
    if res < 0 {
        if ! /*Inapropriate device*/ 25 == std::io::Error::last_os_error().raw_os_error().unwrap() {
            perror(0);
            panic!("ioctl call failed: errno");
        }
    }

    ret != 0
}

fn read_input_event(fd: int) -> Option<InputEvent> {
    use std::mem::{transmute, uninitialized, size_of};
    unsafe {
        let v = uninitialized();
        let res = read(fd, transmute( &v ), size_of::<InputEvent>() as i32);
        if res != size_of::<InputEvent>() as i32 {
            println!("read failed with value: {}", res);
            None
        }
        else {
            Some(v)
        }
    }
}

pub fn open_event() -> int {
    use std::path::Path;

    unsafe {
        for file in std::fs::read_dir( Path::new("/dev/input") ).unwrap() {
            let file = file.unwrap();

            // Apparently event files aren't files according to .is_file, so just make sure
            // it isn't a directory
            if !file.file_type().unwrap().is_dir() {
                let path = file.path().into_os_string().to_cstring().unwrap();
                let fd = open( path.as_ptr(), 0 /* O_RDONLY */ );
                if fd < 0 {
                    perror(0);
                    panic!("Couldn't open {:?}", file.path());
                }
                if has_abs( fd ) {
                    return fd;
                }
            }
        }
        panic!("No input devices support abs events, do you even have a touchpad?");
    }
}

pub fn get_size( fd: int ) -> (i32, i32) {
    let mut info = input_absinfo {
        value: 0,
        minimum: 0,
        maximum: 0,
        flat: 0,
        fuzz: 0
    };

    unsafe {
        use std::mem::transmute as tran;

        if 0 > ioctl(fd, EVIOCGABS( AbsEventCode::X as int ), tran(&mut info) ) {
            perror(0);
            panic!("ioctl EVIOCGABS( EventCode::X ) failed");
        }
        let w = info.maximum;
        if 0 > ioctl(fd, EVIOCGABS( AbsEventCode::Y as int ), tran(&mut info) ) {
            perror(0);
            panic!("ioctl EVIOCGABS( EventCode::Y ) failed");
        }
        let h = info.maximum;

        (w, h)
    }
}

pub enum Event {
    Touch(u32, u32),
    FingerLifted,
}

pub fn init_input() -> (Receiver<Event>, (u32, u32)) {
    let fd = open_event();
    let size = get_size( fd );
    let (tx, rx) = channel();

    let tx_copy = tx.clone();

    // raw touchpad event thread
    std::thread::spawn( move || {
        use self::Event::*;
        let tx = tx_copy;

        let mut x = None;
        let mut y = None;

        loop {
            let event = read_input_event(fd).unwrap();
            match EventType::from_raw(event.tipe) {
                EventType::Syn => {
                    if let (Some(x), Some(y)) = (x, y) {
                        tx.send( Event::Touch(x/size_divisor, y/size_divisor) ).unwrap();
                    }
                    else {
                        tx.send( Event::FingerLifted ).unwrap();
                    }
                },
                EventType::Abs => {
                    use self::AbsEventCode::*;
                    let code = AbsEventCode::from_raw(event.code);
                    //println!("Abs, code: {:?}, val: {:?}", code, event.value);
                    match code {
                        X => {
                            if event.value < 0 {
                                println!("abs event < 0");
                                x = None;
                            }
                            else {
                                x = Some(event.value as u32);
                            }
                        }

                        Y => {
                            if event.value < 0 {
                                y = None;
                            }
                            else {
                                y = Some(event.value as u32);
                            }
                        },
                        MtTrackingId if event.value == -1 => {
                            x = None;
                            y = None;
                        }
                        _ => ()
                    }
                }
                 _ => ()
            }
        } 
    });

    (rx, (size.0 as u32 / size_divisor, size.1 as u32 / size_divisor))
}
