use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::BlendMode;
use std::cmp::{max, min};
use std::collections::VecDeque;
use std::mem::swap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

const RT_OUTPUT_W: u32 = 1280;
const RT_OUTPUT_H: u32 = 720;
const RT_THREADS: u32 = 8;
const RT_TILE_SIZE: u32 = 16;

struct Raytracer {
    output: Vec<AtomicU8>,
    output_sz: (u32, u32),
    stop: AtomicBool,
    tiles: Mutex<VecDeque<Tile>>,
}

impl Raytracer {
    fn new() -> Self {
        // TODO load objects and settings from scene file
        let output_sz = (RT_OUTPUT_W, RT_OUTPUT_H);

        Self {
            output: vec![0u8; (4 * output_sz.0 * output_sz.1) as usize].into_iter().map(AtomicU8::new).collect(),
            output_sz: output_sz,
            stop: AtomicBool::new(false),
            tiles: Mutex::new(VecDeque::new()),
        }
    }

    fn start(self: Arc<Self>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("Raytracer".to_string())
            .spawn(move || {
                self.generate_tiles();

                println!("Render starting");
                let start = Instant::now();

                let threads = (0..RT_THREADS)
                    .map(|i| self.clone().start_worker(i + 1))
                    .collect::<Vec<JoinHandle<()>>>();
                threads.into_iter().for_each(|t| t.join().unwrap());

                if self.stop.load(Ordering::Relaxed) {
                    println!("Render cancelled");
                } else {
                    let end = Instant::now();
                    let d = end - start;
                    println!("Render completed in {}.{:03}s", d.as_secs(), d.subsec_millis());
                }
            })
            .unwrap()
    }

    fn start_worker(self: Arc<Self>, i: u32) -> JoinHandle<()> {
        thread::Builder::new()
            .name(format!("Raytracer-Worker-{i}").to_string())
            .spawn(move || {
                loop {
                    if self.stop.load(Ordering::Relaxed) {
                        break;
                    }
                    match self.tiles.lock().unwrap().pop_front() {
                        Some(tile) => self.work(tile),
                        None => break,
                    }
                    //thread::sleep(Duration::from_millis(50));
                }
            })
            .unwrap()
    }

    fn stop(self: &Arc<Self>) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn generate_tiles(&self) {
        // Tile output using the Hilbert Spiral algorithm from Blender's Cycles engine
        // https://github.com/blender/blender/blob/blender-v2.93-release/intern/cycles/render/tile.cpp#L198

        let mut tiles = self.tiles.lock().unwrap();
        tiles.clear();

        let tile_sz = RT_TILE_SIZE;
        // Size of blocks in tiles, must be a power of 2
        let hilbert_sz = if tile_sz <= 12 { 8 } else { 4 };
        let block_sz = tile_sz * hilbert_sz;

        // Number of blocks to fill the output
        let block_cnt = (
            if block_sz >= self.output_sz.0 { 1 } else { (self.output_sz.0 + block_sz - 1) / block_sz },
            if block_sz >= self.output_sz.1 { 1 } else { (self.output_sz.1 + block_sz - 1) / block_sz },
        );

        // Side length of the spiral (must be odd)
        let n = max(block_cnt.0, block_cnt.1) | 0x1;
        // Offset of spiral (to keep it centered)
        let offset = (
            (self.output_sz.0 as i32 - (n * block_sz) as i32) / 2 / tile_sz as i32 * tile_sz as i32,
            (self.output_sz.1 as i32 - (n * block_sz) as i32) / 2 / tile_sz as i32 * tile_sz as i32,
        );

        let mut block = (0, 0);
        let mut dir = 0; // 0: UP, 1: LEFT, 2: DOWN, 3: RIGHT
        let mut prev_dir = 0;
        let mut i = 0;
        loop {
            // Generate the tiles in the current block
            for hilbert_index in 0..hilbert_sz * hilbert_sz {
                // Convert Hilbert index to position using black magic
                let mut d = hilbert_index;
                let mut s = 1;
                let mut r = (0, 0);
                let mut hilbert_pos = (0, 0);
                while s < hilbert_sz {
                    r.0 = (d >> 1) & 1;
                    r.1 = (d ^ r.0) & 1;
                    if r.1 == 0 {
                        if r.0 != 0 {
                            hilbert_pos = (s - 1 - hilbert_pos.0, s - 1 - hilbert_pos.1);
                        }
                        swap(&mut hilbert_pos.0, &mut hilbert_pos.1);
                    }
                    hilbert_pos = (hilbert_pos.0 + r.0 * s, hilbert_pos.1 + r.1 * s);
                    d >>= 2;
                    s *= 2;
                }

                // Rotate block according to spiral direction
                let tile = if dir == 0 && prev_dir == 0 {
                    (hilbert_pos.1, hilbert_pos.0)
                } else if dir == 1 || prev_dir == 1 {
                    (hilbert_pos.0, hilbert_pos.1)
                } else if dir == 2 {
                    (hilbert_sz - 1 - hilbert_pos.1, hilbert_sz - 1 - hilbert_pos.0)
                } else {
                    (hilbert_sz - 1 - hilbert_pos.0, hilbert_sz - 1 - hilbert_pos.1)
                };

                // Push tile to queue
                let tile_pos = (
                    (block.0 * block_sz + tile.0 * tile_sz) as i32 + offset.0,
                    (block.1 * block_sz + tile.1 * tile_sz) as i32 + offset.1,
                );
                if  tile_pos.0 >= 0 && tile_pos.0 < self.output_sz.0 as i32 &&
                    tile_pos.1 >= 0 && tile_pos.1 < self.output_sz.1 as i32 {
                    let tile_pos = (tile_pos.0 as u32, tile_pos.1 as u32);
                    tiles.push_front(Tile {
                        left: tile_pos.0,
                        top: tile_pos.1,
                        right: tile_pos.0 + min(tile_sz, self.output_sz.0 - tile_pos.0),
                        bottom: tile_pos.1 + min(tile_sz, self.output_sz.1 - tile_pos.1),
                    });
                }
            }

            // Stop when the spiral has reached the center
            if block.0 == (n - 1) / 2 && block.1 == (n - 1) / 2 {
                break;
            }

            // Advance to next block
            prev_dir = dir;
            match dir {
                0 => {
                    block.1 += 1;
                    if block.1 == n - i - 1 {
                        dir += 1;
                    }
                }
                1 => {
                    block.0 += 1;
                    if block.0 == n - i - 1 {
                        dir += 1;
                    }
                }
                2 => {
                    block.1 -= 1;
                    if block.1 == i {
                        dir += 1;
                    }
                }
                3 => {
                    block.0 -= 1;
                    if block.0 == i + 1 {
                        dir = 0;
                        i += 1;
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn work(self: &Arc<Self>, tile: Tile) {
        for y in tile.top..tile.bottom {
            for x in tile.left..tile.right {
                let off = 4 * (x + y * self.output_sz.0) as usize;
                self.output[off].store(255, Ordering::Relaxed);
                self.output[off + 1].store((256 * y / self.output_sz.1) as u8, Ordering::Relaxed);
                self.output[off + 2].store((256 * x / self.output_sz.0) as u8, Ordering::Relaxed);
                self.output[off + 3].store((255 - 256 * x / self.output_sz.0) as u8, Ordering::Relaxed);
            }
        }
    }

    // TODO :D
    // fn raytrace(&self, x: u32, y: u32) {
    // }
}

struct Tile {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

fn main() {
    let raytracer = Arc::new(Raytracer::new());
    let render_thread = raytracer.clone().start();

    let sdl = sdl2::init().unwrap();
    let sdl_video = sdl.video().unwrap();

    let window = sdl_video
        .window("Crusty", 1280, 720)
        .maximized()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGBA8888,
            raytracer.output_sz.0,
            raytracer.output_sz.1,
        )
        .unwrap();
    texture.set_blend_mode(BlendMode::Blend);

    let mut window_sz = canvas.output_size().unwrap();
    let mut pan = (0., 0.);
    let mut zoom = 0.;

    // Main thread window event loop / drawing
    let mut event_pump = sdl.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::MouseMotion { mousestate, xrel, yrel, ..} => {
                    if mousestate.left() {
                        pan.0 += xrel as f64;
                        pan.1 += yrel as f64;
                    }
                }
                Event::MouseWheel { precise_y, .. } => {
                    let old_zoom = zoom;
                    zoom = (zoom + precise_y as f64 / 4.).clamp(-4., 4.);

                    let delta = 2f64.powf(zoom) - 2f64.powf(old_zoom);
                    pan.0 -= delta / 2. * window_sz.0 as f64;
                    pan.1 -= delta / 2. * window_sz.1 as f64;
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    pan = (0., 0.);
                    zoom = 0.;
                }
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } |
                Event::Quit { .. } => {
                    break 'running;
                }
                Event::Window { win_event: WindowEvent::SizeChanged(..), .. } => {
                    window_sz = canvas.output_size().unwrap();
                }
                _ => {}
            }
        }

        // TODO: it would be more efficient to use texture.with_lock / texture streaming, but this is good enough™ for now
        texture
            .update(
                None,
                unsafe { &*(raytracer.output.as_slice() as *const [AtomicU8] as *const [u8]) },
                4 * raytracer.output_sz.0 as usize,
            )
            .unwrap();

        // Calculate the sizes and offsets to fit the texture to the window size (preserving the aspect ratio).
        let window_sz = (window_sz.0 as f64, window_sz.1 as f64);
        let output_sz = (raytracer.output_sz.0 as f64, raytracer.output_sz.1 as f64);
        let display_sz = if window_sz.0 / window_sz.1 > output_sz.0 / output_sz.1 {
            (output_sz.0 * window_sz.1 / output_sz.1, window_sz.1)
        } else {
            (window_sz.0, output_sz.1 * window_sz.0 / output_sz.0)
        };
        let display_pan = (
            (window_sz.0 - display_sz.0) / 2.,
            (window_sz.1 - display_sz.1) / 2.,
        );
        let r = Rect::new(
            (pan.0 + display_pan.0) as i32,
            (pan.1 + display_pan.1) as i32,
            (2f64.powf(zoom) * display_sz.0) as u32,
            (2f64.powf(zoom) * display_sz.1) as u32,
        );

        // Draw and present frame
        canvas.set_draw_color(Color::RGB(64, 64, 64)); // background
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 255, 255)); // border
        canvas.draw_rect(r).unwrap();
        canvas.copy(&texture, None, r).unwrap();
        canvas.present();
    }

    raytracer.stop();
    render_thread.join().unwrap();
}