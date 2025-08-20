use crate::raytracer::{Ray, RGBA};
use crate::raytracer::objects::ObjectHit;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub type MaterialNewFn = fn(&Value) -> Result<Box<dyn MaterialType + Sync + Send>, String>;

static MATERIAL_TYPES: LazyLock<Mutex<HashMap<String, MaterialNewFn>>> =
    LazyLock::new(|| Mutex::new(HashMap::from([
    ])));

pub static FALLBACK: LazyLock<Material> = LazyLock::new(|| Material { inner: Box::new(Fallback) } );

pub struct Material {
    inner: Box<dyn MaterialType + Send + Sync>,
}

pub trait MaterialType {
    fn shade<'a>(&self, oh: &'a ObjectHit, raytrace: Box<dyn Fn(Ray) -> RGBA + 'a>) -> RGBA;
}

struct Fallback;

impl Material {
    pub fn register_type(name: String, new_fn: MaterialNewFn) {
        let mut types = MATERIAL_TYPES.lock().unwrap();
        types.insert(name, new_fn);
    }

    pub fn new(type_name: &String, data: &Value) -> Result<Self, String> {
        let types = MATERIAL_TYPES.lock().unwrap();
        let inner = match types.get(type_name) {
            Some(mat_new_fn) => mat_new_fn(data),
            None => Err(format!("Could not find material type {}", type_name)),
        }?;

        Ok(Material {
            inner,
        })
    }

    pub fn shade<'a>(&self, oh: &'a ObjectHit, raytrace: Box<dyn Fn(Ray) -> RGBA + 'a>) -> RGBA {
        self.inner.shade(oh, raytrace)
    }
}

impl MaterialType for Fallback {
    fn shade<'a>(&self, oh: &'a ObjectHit, _: Box<dyn Fn(Ray) -> RGBA +'a>) -> RGBA {
        if ((oh.hit.uv.0 * 20.0) as u8 % 2) ^ ((oh.hit.uv.1 * 20.0) as u8 % 2) == 0 {
            RGBA::new(oh.hit.uv.0, 0.0, oh.hit.uv.1, 1.0)
        } else {
            RGBA::black()
        }
    }
}
