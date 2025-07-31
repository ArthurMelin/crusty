use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::BlendMode;

struct Raytracer {
    output: Vec<AtomicU8>,
    output_sz: (u32, u32),
    stop: AtomicBool,
}

impl Raytracer {
    fn new() -> Self {
        let width = 1280;
        let height = 720;

        Self {
            output: vec![0u8; (4 * width * height) as usize].into_iter().map(AtomicU8::new).collect(),
            output_sz: (width, height),
            stop: AtomicBool::new(false),
        }
    }

    fn start(self: Arc<Self>) -> JoinHandle<()> {
        thread::spawn(move || {
            println!("Render started");
            for y in 0..self.output_sz.1 {
                for x in 0..self.output_sz.0 {
                    if self.stop.load(Ordering::Relaxed) {
                        println!("Render cancelled");
                        return;
                    }

                    let off = 4 * (x + y * self.output_sz.0) as usize;
                    self.output[off].store(255, Ordering::Relaxed);
                    self.output[off + 1].store((256 * y / self.output_sz.1) as u8, Ordering::Relaxed);
                    self.output[off + 2].store((256 * x / self.output_sz.0) as u8, Ordering::Relaxed);
                    self.output[off + 3].store((255 - 256 * x / self.output_sz.0) as u8, Ordering::Relaxed);
                }
                thread::sleep(Duration::from_millis(5));
            }
            println!("Render finished");
        })
    }

    fn stop(self: Arc<Self>) {
        self.stop.store(true, Ordering::Relaxed);
    }

    // TODO :D
    // fn raytrace(&self, x: u32, y: u32) {
    // }
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
    let mut dragging = false;

    // Main thread window event loop / drawing
    let mut event_pump = sdl.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
                    dragging = false;
                }
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, .. } => {
                    dragging = true;
                }
                Event::MouseMotion { xrel, yrel, ..} => {
                    if dragging {
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
        texture.update(
            None,
            unsafe { &*(raytracer.output.as_slice() as *const [AtomicU8] as *const [u8]) },
            4 * raytracer.output_sz.0 as usize,
        ).unwrap();

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