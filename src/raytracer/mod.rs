mod materials;
mod objects;
mod scene;
mod tile;
mod transform;
mod utils;

use rand;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::ptr;
use std::thread;
use std::time::Instant;

use materials::Material;
use objects::Object;
use scene::Scene;
use tile::Tile;
use transform::Transform;
use utils::vec3norm;
use crate::raytracer::materials::FALLBACK;

pub struct Raytracer {
    camera: Camera,
    output: Output,
    materials: HashMap<String, Material>,
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
    samples: u32,
    tile_size: u32,
    buffer: Vec<AtomicU32>,
}

#[derive(Clone, Copy)]
pub struct Ray {
    pub ray_type: RayType,
    pub origin: (f64, f64, f64),
    pub direction: (f64, f64, f64),
}

struct RGBA {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

#[derive(Clone, Copy)]
pub enum RayType {
    Camera,
}

impl Raytracer {
    pub fn new<R>(reader: R) -> Result<Arc<Self>, String>
    where
        R: std::io::Read
    {
        let scene: Scene = serde_json::from_reader(reader)
            .map_err(|err| format!("Failed to parse scene: {}", err))?;

        let materials = scene.materials.iter()
            .map(|(id, scene_material)| Material::try_from(scene_material).map(|mat| (id.clone(), mat)))
            .collect::<Result<HashMap<String, Material>, String>>()?;

        let objects = scene.objects.iter()
            .map(|scene_object| Object::try_from(scene_object))
            .collect::<Result<Vec<Object>, String>>()?;

        Ok(Arc::new(Self {
            camera: Camera::from(&scene.camera),
            output: Output::from(&scene.output),
            materials,
            objects,
            stop: AtomicBool::new(false),
            progress: AtomicU32::new(0),
            tiles: Mutex::new(VecDeque::new()),
        }))
    }

    pub fn start(self: &Arc<Self>, threads: u32) -> thread::JoinHandle<()> {
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
                    .collect::<Vec<_>>();
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

    fn start_worker(self: &Arc<Self>, i: u32) -> thread::JoinHandle<()> {
        let clone = self.clone();
        thread::Builder::new()
            .name(format!("RT-Worker-{i}"))
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

                let samples: Vec<RGBA> = (0..self.output.samples)
                    .map(|_| {
                        let offset: (f64, f64) = rand::random();
                        let ray = Ray {
                            ray_type: RayType::Camera,
                            origin: self.camera.transform.apply((0.0, 0.0, 0.0)),
                            direction: self.camera.transform.apply_notranslate(vec3norm((
                                (2.0 * (x as f64 + offset.0) / self.output.width as f64 - 1.0) *
                                    (self.camera.fov.to_radians() / 2.0).tan() *
                                    (self.output.width as f64 / self.output.height as f64),
                                self.camera.near,
                                (1.0 - 2.0 * (y as f64 + offset.1) / self.output.height as f64) *
                                    (self.camera.fov.to_radians() / 2.0).tan(),
                            ))),
                        };

                        self.raytrace(ray, None)
                    })
                    .collect();

                let color = RGBA::average(&samples);
                self.output.put(x, y, color);
                self.progress.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn raytrace(&self, ray: Ray, ignore: Option<&Object>) -> RGBA {
        let hit = self.objects.iter()
            .filter(|object| !ignore.is_some_and(|ignore| ptr::eq(*object, ignore)))
            .filter_map(|obj| obj.intersect(&ray))
            .min_by(|a, b| a.hit.distance.total_cmp(&b.hit.distance));

        match hit {
            Some(hit) => {
                let material = self.materials.get(&hit.object.material).or(Some(&FALLBACK)).unwrap();
                material.shade(&hit, Box::new(|ray| self.raytrace(ray, Some(hit.object))))
            },
            None => RGBA::transparent(),
        }
    }
}

impl Output {
    fn new(width: u32, height: u32, samples: u32, tile_size: u32) -> Output {
        Output {
            width,
            height,
            samples,
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
    fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    fn transparent() -> Self { Self::new(0.0, 0.0, 0.0, 0.0) }
    fn black() -> Self { Self::new(0.0, 0.0, 0.0, 1.0) }
    fn white() -> Self { Self::new(1.0, 1.0, 1.0, 1.0) }

    fn average(samples: &Vec<RGBA>) -> RGBA {
        let ssum = samples
            .iter()
            .fold((0.0, 0.0, 0.0, 0.0), |acc, s| (
                acc.0 + s.r * s.r * s.a,
                acc.1 + s.g * s.g * s.a,
                acc.2 + s.b * s.b * s.a,
                acc.3 + s.a,
            ));

        RGBA::new(
            (ssum.0 / ssum.3).sqrt(),
            (ssum.1 / ssum.3).sqrt(),
            (ssum.2 / ssum.3).sqrt(),
            ssum.3 / samples.len() as f64,
        )
    }
}

impl Into<u32> for RGBA {
    fn into(self) -> u32 {
        ((self.r * 255.0) as u32) << 24 | ((self.g * 255.0) as u32) << 16 | ((self.b * 255.0) as u32) << 8 | (self.a * 255.0) as u32
    }
}
