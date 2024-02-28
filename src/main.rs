// Purpose: Main file for the project.
use std::io;
use pixels::{Pixels, SurfaceTexture};
use std::time::{Duration, Instant};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 32;

struct State {
    memory: [u8; 4096],
    pc: u16,
    // stack: Vec<u16>,
    i: u16,
    // delay_timer: u8,
    // sound_timer: u8,
    v: [u8; 16],
}

impl State {
    fn new() -> Self {
        Self {
            memory: [0; 4096],
            pc: 0x0000,
            // stack: Vec::new(),
            i: 0x0000,
            // delay_timer: 0x00,
            // sound_timer: 0x00,
            v: [0; 16],
        }
    }
}

fn clear_screen(pixels: &mut Pixels) {
    let frame = pixels.frame_mut();
    for pixel in frame.chunks_exact_mut(4) {
        // RGBA to black
        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    }
}

fn jump_to(state: &mut State, opcode: u16) {
    // opcode is 0x1NNN, mask with 0x0FFF to get NNN
    state.pc = opcode & 0x0FFF;
}

fn set_vx(state: &mut State, opcode: u16) {
    // mask second nibble with 0x0F00 and shift right 8 bits to get x
    let x = opcode & 0x0F00 >> 8;
    // mask last byte with 0x00FF to get value
    let value = (opcode & 0x00FF) as u8;
    state.v[x as usize] = value;
}

fn add_vx(state: &mut State, opcode: u16) {
    // mask second nibble with 0x0F00 and shift right 8 bits to get x
    let x = (opcode & 0x0F00) >> 8;
    // mask last byte with 0x00FF to get value
    let value = (opcode & 0x00FF) as u8;
    state.v[x as usize] += value;
}

fn set_i(state: &mut State, opcode: u16) {
    let value = opcode & 0x0FFF;
    state.i = value;
}

fn display(state: &mut State, pixels: &mut Pixels, opcode: u16) {
    let frame = pixels.frame_mut();
    let x = (state.v[((opcode & 0x0F00) >> 8) as usize] % WIDTH as u8) as usize;
    let y = (state.v[((opcode & 0x00F0) >> 4) as usize] % HEIGHT as u8) as usize;
    let rows = opcode & 0x000F;
    let stride = WIDTH as usize * 4;

    state.v[0xF] = 0;

    for row in 0..rows {
        let sprite_y = y + row as usize;
        let sprite_row = state.memory[(state.i + row) as usize];
        for col in 0..8 {
            let sprite_x = x + col;
            // let sprite_pixel = (sprite_row >> col) & 1;
            let sprite_pixel = (sprite_row >> (7 - col)) & 1;
            if sprite_pixel == 0 {
                continue;
            }
            let pixel_index = (sprite_y * stride) + (sprite_x * 4);
            let pixel = &mut frame[pixel_index..pixel_index + 4];
            // each pixel in here is RGBA
            if pixel == [0, 0, 0, 0] {
                pixel[0] = 0xFF; // R
                pixel[1] = 0xFF; // G
                pixel[2] = 0xFF; // B
                pixel[3] = 0xFF; // A
                state.v[0xF] = 1;
            } else {
                pixel[0] = 0x00; // R
                pixel[1] = 0x00; // G
                pixel[2] = 0x00; // B
                pixel[3] = 0xFF; // A
            }
            if sprite_x == WIDTH as usize {
                continue;
            }
        }
        if sprite_y == HEIGHT as usize {
            continue;
        }
    }
}

fn fetch_opcode(state: &mut State) -> u16 {
    let pc = state.pc as usize;
    let opcode = (state.memory[pc] as u16) << 8 | state.memory[pc + 1] as u16;
    state.pc += 2;
    opcode
}

fn execute_opcode(opcode: u16, state: &mut State, pixels: &mut Pixels) {
    // match opcode category
    match opcode & 0xF000 {
        // 0 category
        0x0000 => match opcode & 0x000F {
            // 0 category with last nibble to 0 is clear screen
            0x0000 => clear_screen(pixels),
            _ => unimplemented!(),
        },
        // 1 category is jump to address
        0x1000 => jump_to(state, opcode),
        // 6 category is set vx to nn
        0x6000 => set_vx(state, opcode),
        // 7 category is add vx to nn
        0x7000 => add_vx(state, opcode),
        // A category is set i to nnn
        0xA000 => set_i(state, opcode),
        // D category is draw
        0xD000 => display(state, pixels, opcode),
        _ => unimplemented!(),
    }
}

fn read_program_into_memory(filepath: &str, state: &mut State) -> io::Result<()> {
    let program = std::fs::read(filepath)?;
    let mut pc = 0x200;
    for byte in program {
        state.memory[pc] = byte;
        pc += 1;
    }

    Ok(())
}

fn run(mut state: State) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = {
        let size = LogicalSize::new(WIDTH, HEIGHT);
        WindowBuilder::new()
            .with_title("Chip-8 Emulator")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
    };
    let target_frame_time = Duration::from_secs_f64(1.0 / 60.0);

    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run(move |event, elwt| {
        let frame_start = Instant::now();

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::RedrawRequested => {
                    let opcode = fetch_opcode(&mut state);
                    execute_opcode(opcode, &mut state, &mut pixels);

                    if pixels
                        .render()
                        .map_err(|e| println!("pixels.render() failed: {}", e))
                        .is_err()
                    {
                        elwt.exit();
                        return;
                    }
                }
                WindowEvent::CloseRequested => {
                    println!("Closing the window");
                    elwt.exit();
                    return;
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key: Key::Named(NamedKey::Escape),
                            state: ElementState::Released,
                            ..
                        },
                    ..
                } => {
                    println!("Closing the window");
                    elwt.exit();
                    return;
                }
                _ => (),
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => (),
        }

        let frame_duration = frame_start.elapsed();

        if frame_duration < target_frame_time {
            // If the frame finished early, wait the remaining time
            elwt.set_control_flow(ControlFlow::WaitUntil(frame_start + target_frame_time));
        } else {
            // If the frame took too long, continue immediately
            elwt.set_control_flow(ControlFlow::Poll);
        }
    });
}

fn main() {
    let mut state = State::new();
    match read_program_into_memory("./ibm.ch8", &mut state) {
        Ok(()) => run(state),
        Err(e) => println!("Failed to read file: {}", e)
    }
}
