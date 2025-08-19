mod objects;
mod scene;
mod tile;
mod transform;
mod utils;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use objects::{Object, ObjectHit};
use scene::Scene;
use tile::Tile;
use transform::Transform;
use utils::vec3norm;

pub struct Raytracer {
    camera: Camera,
    output: Output,
    objects: Vec<Object>,
    progress: AtomicU32,
    stop: AtomicBool,
    tiles: Mutex<VecDeque<Tile>>,
}

struct Camera {
    fov: f64,
    near: f64,
    transform: Transform,
}

pub struct Output {
    pub width: u32,
    pub height: u32,
    tile_size: u32,
    buffer: Vec<AtomicU32>,
}

#[derive(Clone, Copy)]
pub struct Ray {
    pub origin: (f64, f64, f64),
    pub direction: (f64, f64, f64),
}

struct RGBA {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Raytracer {
    pub fn new<R>(reader: R) -> Result<Arc<Self>, String>
    where
        R: std::io::Read
    {
        let scene: Scene = serde_json::from_reader(reader)
            .map_err(|err| format!("Failed to parse scene: {}", err))?;

        Ok(Arc::new(Self {
            camera: Camera::from(&scene.camera),
            output: Output::from(&scene.output),
            objects: scene.objects.iter()
                .map(|scene_object| { Object::try_from(scene_object) })
                .collect::<Result<Vec<Object>, String>>()?,
            stop: AtomicBool::new(false),
            progress: AtomicU32::new(0),
            tiles: Mutex::new(VecDeque::new()),
        }))
    }

    pub fn start(self: &Arc<Self>, threads: u32) -> JoinHandle<()> {
        let clone = self.clone();
        thread::Builder::new()
            .name("Raytracer".to_string())
            .spawn(move || {
                {
                    let output = &clone.output;
                    let mut tiles = clone.tiles.lock().unwrap();
                    tiles.clear();
                    for tile in tile::hilbert_tiles(output.width, output.height, output.tile_size) {
                        tiles.push_front(tile);
                    }
                }

                println!("Render starting");
                let start = Instant::now();

                let threads = (0..threads)
                    .map(|i| clone.start_worker(i + 1))
                    .collect::<Vec<JoinHandle<()>>>();
                threads.into_iter().for_each(|t| t.join().unwrap());

                if clone.stop.load(Ordering::Relaxed) {
                    println!("Render cancelled");
                } else {
                    let end = Instant::now();
                    let d = end - start;
                    println!("Render completed in {}.{:03}s", d.as_secs(), d.subsec_millis());
                }
            })
            .unwrap()
    }

    fn start_worker(self: &Arc<Self>, i: u32) -> JoinHandle<()> {
        let clone = self.clone();
        thread::Builder::new()
            .name(format!("RT-Worker-{i}").to_string())
            .spawn(move || {
                loop {
                    if clone.stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let tile = clone.tiles.lock().unwrap().pop_front();
                    match tile {
                        Some(tile) => clone.work(tile),
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
    pub fn output(self: &Arc<Self>) -> &Output {
        &self.output
    }

    #[inline]
    pub fn progress(self: &Arc<Self>) -> f64 {
        self.progress.load(Ordering::Relaxed) as f64 / (self.output.width * self.output.height) as f64
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

                        self.output.put(x, y, color);
                    }
                    None => {}
                }
                self.progress.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn raytrace(&self, x: u32, y: u32) -> Option<ObjectHit> {
        let ray = Ray {
            origin: self.camera.transform.apply((0.0, 0.0, 0.0)),
            direction: self.camera.transform.apply_notranslate(vec3norm((
                (2.0 * (x as f64 + 0.5) / self.output.width as f64 - 1.0) *
                    (self.camera.fov.to_radians() / 2.0).tan() *
                    (self.output.width as f64 / self.output.height as f64),
                self.camera.near,
                (1.0 - 2.0 * (y as f64 + 0.5) / self.output.height as f64) *
                    (self.camera.fov.to_radians() / 2.0).tan(),
            ))),
        };

        self.objects.iter()
            .filter_map(|obj| { obj.intersect(&ray) })
            .min_by(|a, b| a.hit.distance.total_cmp(&b.hit.distance))
    }

    fn render_color(&self, oh: &ObjectHit) -> RGBA {
        // TODO
        if ((oh.hit.uv.0 * 20.0) as u8 % 2) ^ ((oh.hit.uv.1 * 20.0) as u8 % 2) == 0 {
            RGBA::new((oh.hit.uv.0 * 256.0) as u8, 0, (oh.hit.uv.1 * 256.0) as u8, 255)
        } else {
            RGBA::new(0, 0, 0, 255)
        }
    }
}

impl Output {
    fn new(width: u32, height: u32, tile_size: u32) -> Output {
        Output {
            width,
            height,
            tile_size,
            buffer: vec![0u32; (width * height) as usize].into_iter().map(AtomicU32::new).collect(),
        }
    }
    pub fn get(&self) -> &[u8] {
        unsafe { &*(self.buffer.as_slice() as *const [AtomicU32] as *const [u8]) }
    }
    fn put(&self, x: u32, y: u32, color: RGBA) {
        self.buffer[(x + y * self.width) as usize].store(color.into(), Ordering::Relaxed)
    }
}

impl RGBA {
    #[inline]
    fn new(r: u8, g: u8, b: u8, a: u8) -> RGBA {
        RGBA { r, g, b, a }
    }
}

impl Into<u32> for RGBA {
    fn into(self) -> u32 {
        (self.r as u32) << 24 | (self.g as u32) << 16 | (self.b as u32) << 8 | self.a as u32
    }
}
