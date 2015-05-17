#![feature(libc,file_type,dir_entry_ext,convert)]
#![allow(non_upper_case_globals)]
extern crate libc;
extern crate time;

extern crate sdl2;
extern crate image;

use event::init_input;
use image::ImageBuffer;

mod event;

struct AppState {
    w: u32,
    h: u32,
    draw_color: image::Rgb<u8>,
    img: ImageBuffer<image::Rgb<u8>, Vec<u8>>
}

enum Mode {
    Pencil{ prev: Option<(u32, u32)> },
    Line{ start: Option<(u32, u32)>, lifted: bool },
    Selection,
}

impl Mode {
    fn handle_input( self, state: &mut AppState, ev: event::Event ) -> Mode {
        use Mode::*;
        use event::Event::*;

        match self {
            Pencil{ prev } => {
                match ev {
                     Touch( x, y ) if x < state.w && y < state.h => {
                        if let Some( start ) = prev {
                            draw_line(state, start, (x, y)); 
                        }
                        Pencil{ prev: Some((x, y)) }
                   }
                    Touch( _, _ ) => {
                        println!("Dumb thing isn't restriced to it's bounds");
                        Pencil{ prev: None }
                    }
                    FingerLifted => {
                        Pencil{ prev: None }
                    }
                }     
            },
            
            Line{ start: None, lifted: false } => {
                match ev {
                    Touch( x, y ) if x < state.w && y < state.h => {
                        Line{ start: Some((x, y)), lifted: false }
                    }
                    _ => self,
                }
            }
            Line{ start: Some(start), lifted: false } => {
                match ev {
                    FingerLifted => {
                        Line{ start: Some(start), lifted: true }
                    }
                    _ => self,
                }
            }
            Line{ start: Some(start), lifted: true } => {
                match ev {
                    Touch( x, y ) if x < state.w && y < state.h => {
                        draw_line(state, start, (x, y));
                        Line{ start: None, lifted: false }
                    }
                    _ => self,
                }
            }
            Line{ start: None, lifted: true } => {
                panic!("Invalid state, drawing a line, have lifted finger, but don't have a starting point");
            }

            Selection => Selection,
        }
    }
}


fn draw_line( state: &mut AppState, mut start: (u32, u32), end: (u32, u32) ) {
    let pixel = state.draw_color;

    state.img.put_pixel( start.0, start.1, pixel );

    fn towards( x: u32, y: u32 ) -> u32 {
        if x > y {
            x - 1
        }
        else {
            x + 1
        }
    }

    // Deal with vertical lines
    if start.0 == end.0 {
        while start != end {
            if start.1 > end.1 {
                start.1 -= 1;
            }
            else{ start.1 += 1; }

            state.img.put_pixel( start.0, start.1, pixel );
        }

        return;
    }
    
    // Bresenham's algorithm from wikipedia
    
    let dx = end.0 as i32 - start.0 as i32;
    let dy = end.1 as i32 - start.1 as i32;
    let mut err = 0.0;
    let derr = (dy as f32 / dx as f32).abs();

    let mut y = start.1;
    let mut x = start.0;
    while x != end.0 {
        x = towards( x, end.0 );
        state.img.put_pixel( x, y, pixel );
        err += derr;
        while err >= 0.5 {
            state.img.put_pixel(x, y, pixel );
            y = towards(y, end.1);
            err -= 1.0;
        }
    }
}

fn main() {
    use sdl2::video::{WindowPos, Window, OPENGL, INPUT_GRABBED};
    use sdl2::surface::Surface;

    let (rx, (w, h)) = init_input();
    
    let ctx = sdl2::init(sdl2::INIT_EVERYTHING).unwrap();
    let window = Window::new( &ctx, "touchpad-draw", WindowPos::PosCentered, 
                              WindowPos::PosCentered, w as i32, h as i32, OPENGL | INPUT_GRABBED )
        .unwrap();
    let mut renderer = sdl2::render::Renderer::from_window(
        window, sdl2::render::RenderDriverIndex::Auto, sdl2::render::ACCELERATED)
        .unwrap();
    let mut events = ctx.event_pump();
    sdl2::mouse::show_cursor( false );

    let filename_o = std::env::args_os().nth(1);
    let file = if let Some( ref filename ) = filename_o {
        std::path::Path::new( filename)
    }
    else {
        panic!("Usage: touchpad-draw filename");
    };

    let img = image::ImageBuffer::from_pixel(
        w as u32, h as u32,
        image::Rgb{ data: [255, 255, 255] }
    );

    let mut state = AppState {
        w: w,
        h: h,
        img: img,
        draw_color: image::Rgb{ data: [0, 0, 0] },
    };


    let mut last_update = 0.0;
    let mut mode = Mode::Pencil{ prev: None };

    'mainloop : loop {
        use std::sync::mpsc::TryRecvError::*;

        // Ideally this would be in another thread, but SDL2
        // needs to do pretty much everything on the main thread :(
        //
        // TODO: Make Glutin threadsafe and switch back to that
        // (In the future, far in the future)
        for ev in events.poll_iter() {
            use sdl2::event::Event::*;
            use sdl2::keycode::KeyCode::*;

            match ev {
                Quit{ .. } => break 'mainloop, 
                KeyDown{ keycode: Space, .. } => mode = Mode::Selection,
                KeyDown{ keycode: I, .. } => mode = Mode::Pencil{ prev: None },
                KeyDown{ keycode: L, .. } => mode = Mode::Line{ start: None, lifted: false },
                KeyDown{ keycode: Num1, .. } => state.draw_color = image::Rgb{ data: [0, 0, 0] },
                KeyDown{ keycode: Num2, .. } => state.draw_color = image::Rgb{ data: [255, 0, 0] },
                KeyDown{ keycode: Num3, .. } => state.draw_color = image::Rgb{ data: [0, 255, 0] },
                KeyDown{ keycode: Num4, .. } => state.draw_color = image::Rgb{ data: [0, 0, 255] },
                _ => (),
            }
        }

        match rx.try_recv() {
            Ok( ev ) => {
                mode = mode.handle_input(  &mut state, ev ); 
                continue;
            }
            Err(Empty) => (),
            Err(Disconnected) => break,
        }
        
        

        if time::precise_time_s() - 0.030 > last_update {
            last_update = time::precise_time_s();

            let surface = Surface::from_data( &mut state.img, w as i32, h as i32, 24, w as i32*3, 0, 0, 0, 0).unwrap();
            let tex = renderer.create_texture_from_surface(&surface).unwrap();
            
            let mut drawer = renderer.drawer();
            drawer.clear();
            drawer.copy(&tex, None, None);
            drawer.present();
        }
        else {
            // No events, not time to render, so wait a bit
            std::thread::sleep_ms(2);
        }
    }
    drop( renderer );
    
    println!("Writing file");
    state.img.save(&file).unwrap();
    println!("Done writing file");
}
