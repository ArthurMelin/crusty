mod raytracer;

use raytracer::Raytracer;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, ScaleMode};

fn main() {
    let raytracer = Raytracer::new();
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
            raytracer.output_sz().0,
            raytracer.output_sz().1,
        )
        .unwrap();
    texture.set_blend_mode(BlendMode::Blend);
    texture.set_scale_mode(ScaleMode::Linear);

    let mut window_sz = canvas.output_size().unwrap();
    let mut pan = (0.0, 0.0);
    let mut zoom = 0.0;

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
                    zoom = (zoom + precise_y as f64 / 4.0).clamp(-4.0, 4.0);

                    let delta = 2f64.powf(zoom) - 2f64.powf(old_zoom);
                    pan.0 -= delta / 2.0 * window_sz.0 as f64;
                    pan.1 -= delta / 2.0 * window_sz.1 as f64;
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    pan = (0.0, 0.0);
                    zoom = 0.0;
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
            .update(None, raytracer.output(), 4 * raytracer.output_sz().0 as usize)
            .unwrap();

        // Calculate the sizes and offsets to fit the texture to the window size (preserving the aspect ratio).
        let window_sz = (window_sz.0 as f64, window_sz.1 as f64);
        let output_sz = (raytracer.output_sz().0 as f64, raytracer.output_sz().1 as f64);
        let display_sz = if window_sz.0 / window_sz.1 > output_sz.0 / output_sz.1 {
            (output_sz.0 * window_sz.1 / output_sz.1, window_sz.1)
        } else {
            (window_sz.0, output_sz.1 * window_sz.0 / output_sz.0)
        };
        let display_pan = (
            (window_sz.0 - display_sz.0) / 2.0,
            (window_sz.1 - display_sz.1) / 2.0,
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
