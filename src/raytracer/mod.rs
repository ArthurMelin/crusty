mod objects;
mod tile;
mod transform;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use objects::Object;
use tile::Tile;
use transform::Transform;

const OUTPUT_W: u32 = 1920;
const OUTPUT_H: u32 = 1080;
const THREADS: u32 = 8;
const TILE_SIZE: u32 = 16;
const CAMERA_FOV: f64 = 90.;
const CAMERA_NEAR: f64 = 10.;

pub struct Raytracer {
    objects: Vec<Object>,
    output: Vec<AtomicU8>,
    output_sz: (u32, u32),
    progress: AtomicU32,
    stop: AtomicBool,
    tiles: Mutex<VecDeque<Tile>>,
}

#[derive(Debug)]
struct Ray {
    origin: (f64, f64, f64),
    direction: (f64, f64, f64),
}

#[derive(Debug)]
struct Hit {
    distance: f64,
}

#[derive(Debug)]
struct RGBA {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Raytracer {
    pub fn new() -> Arc<Self> {
        // TODO load objects and settings from a scene file
        let mut objects = Vec::<Object>::new();
        objects.push(Object::new().unwrap());

        let output_sz = (OUTPUT_W, OUTPUT_H);

        Arc::new(Self {
            objects,
            output: vec![0u8; (4 * output_sz.0 * output_sz.1) as usize].into_iter().map(AtomicU8::new).collect(),
            output_sz,
            stop: AtomicBool::new(false),
            progress: AtomicU32::new(0),
            tiles: Mutex::new(VecDeque::new()),
        })
    }

    pub fn start(self: Arc<Self>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("Raytracer".to_string())
            .spawn(move || {
                {
                    let mut tiles = self.tiles.lock().unwrap();
                    tiles.clear();
                    for tile in tile::hilbert_tiles(self.output_sz.0, self.output_sz.1, TILE_SIZE) {
                        tiles.push_front(tile);
                    }
                }

                println!("Render starting");
                let start = Instant::now();

                let threads = (0..THREADS)
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
            .name(format!("RT-Worker-{i}").to_string())
            .spawn(move || {
                loop {
                    if self.stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let tile = self.tiles.lock().unwrap().pop_front();
                    match tile {
                        Some(tile) => self.work(tile),
                        None => break,
                    }
                }
            })
            .unwrap()
    }

    #[inline]
    pub fn stop(self: &Arc<Self>) {
        self.stop.store(true, Ordering::Relaxed);
    }

    #[inline]
    pub fn output(self: &Arc<Self>) -> &[u8] {
        unsafe { &*(self.output.as_slice() as *const [AtomicU8] as *const [u8]) }
    }

    #[inline]
    pub fn output_sz(self: &Arc<Self>) -> (u32, u32) {
        self.output_sz
    }

    #[inline]
    pub fn progress(self: &Arc<Self>) -> f64 {
        self.progress.load(Ordering::Relaxed) as f64 / (self.output_sz.0 * self.output_sz.1) as f64
    }

    fn work(self: &Arc<Self>, tile: Tile) {
        for y in tile.top..tile.bottom {
            for x in tile.left..tile.right {
                if self.stop.load(Ordering::Relaxed) {
                    break;
                }
                match self.raytrace(x, y) {
                    Some(hit) => {
                        let color = self.render_color(&hit);

                        let off = 4 * (x + y * self.output_sz.0) as usize;
                        self.output[off].store(color.a, Ordering::Relaxed);
                        self.output[off + 1].store(color.b, Ordering::Relaxed);
                        self.output[off + 2].store(color.g, Ordering::Relaxed);
                        self.output[off + 3].store(color.r, Ordering::Relaxed);
                    }
                    None => {}
                }
                self.progress.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn raytrace(&self, x: u32, y: u32) -> Option<Hit> {
        let camera_tf = Transform::new()
            .translate(0.0, 0.0, 20.0)
            .rotate(180.0, 0.0, 0.0)
            .scale(1.0, 1.0, 1.0);

        let ray = Ray {
            origin: camera_tf.apply((0., 0., 0.)),
            direction: camera_tf.apply_notranslate(vec3norm((
                (2. * (x as f64 + 0.5) / self.output_sz.0 as f64 - 1.) *
                    (CAMERA_FOV.to_radians() / 2.).tan() *
                    (self.output_sz.0 as f64 / self.output_sz.1 as f64),
                (1. - 2. * (y as f64 + 0.5) / self.output_sz.1 as f64) *
                    (CAMERA_FOV.to_radians() / 2.).tan(),
                CAMERA_NEAR,
            ))),
        };

        self.objects.iter().find_map(|obj| { obj.intersect(&ray) })
    }

    fn render_color(&self, hit: &Hit) -> RGBA {
        // TODO
        let c = ((20.5 - hit.distance) * 150.0) as u8;
        RGBA::new(c, c, c, 255)
    }
}

impl RGBA {
    #[inline]
    fn new(r: u8, g: u8, b: u8, a: u8) -> RGBA {
        RGBA { r, g, b, a }
    }
}

#[inline]
pub fn vec3norm(v: (f64, f64, f64)) -> (f64, f64, f64) {
    let mag = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    (v.0 / mag, v.1 / mag, v.2 / mag)
}
