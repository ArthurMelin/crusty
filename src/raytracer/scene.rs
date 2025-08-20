use crate::raytracer::{Camera, Output};
use crate::raytracer::materials::Material;
use crate::raytracer::objects::Object;
use crate::raytracer::transform::Transform;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Scene {
    pub camera: SceneCamera,
    pub output: SceneOutput,
    pub materials: HashMap<String, SceneMaterial>,
    pub objects: Vec<SceneObject>,
}

#[derive(Deserialize)]
pub struct SceneCamera {
    #[serde(default = "default_camera_fov")]
    fov: f64,
    #[serde(default = "default_camera_near")]
    near: f64,
    transform: SceneTransform,
}

#[derive(Deserialize)]
pub struct SceneOutput {
    width: u32,
    height: u32,
    #[serde(default = "default_output_samples")]
    samples: u32,
    #[serde(default = "default_output_tile_size")]
    tile_size: u32,
}

#[derive(Deserialize)]
pub struct SceneMaterial {
    #[serde(rename = "type")]
    type_name: String,
    #[serde(flatten)]
    data: Value,
}

#[derive(Deserialize)]
pub struct SceneObject {
    #[serde(rename = "type")]
    type_name: String,
    transform: SceneTransform,
    #[serde(default)]
    material: String,
    #[serde(flatten)]
    data: Value,
}

#[derive(Deserialize)]
pub struct SceneTransform {
    #[serde(default)]
    translate: [f64; 3],
    #[serde(default)]
    rotate: [f64; 3],
    #[serde(default = "default_transform_scale")]
    scale: [f64; 3],
}

impl From<&SceneCamera> for Camera {
    fn from(scene_camera: &SceneCamera) -> Self {
        Self {
            fov: scene_camera.fov,
            near: scene_camera.near,
            transform: Transform::from(&scene_camera.transform),
        }
    }
}

impl From<&SceneOutput> for Output {
    fn from(scene_output: &SceneOutput) -> Self {
        Self::new(
            scene_output.width,
            scene_output.height,
            scene_output.samples,
            scene_output.tile_size,
        )
    }
}

impl TryFrom<&SceneMaterial> for Material {
    type Error = String;

    fn try_from(scene_material: &SceneMaterial) -> Result<Self, Self::Error> {
        Self::new(
            &scene_material.type_name,
            &scene_material.data,
        )
    }
}

impl TryFrom<&SceneObject> for Object {
    type Error = String;

    fn try_from(scene_object: &SceneObject) -> Result<Self, Self::Error> {
        Self::new(
            &scene_object.type_name,
            &scene_object.data,
            Transform::from(&scene_object.transform),
            &scene_object.material,
        )
    }
}

impl From<&SceneTransform> for Transform {
    fn from(scene_transform: &SceneTransform) -> Self {
        let [tx, ty, tz] = scene_transform.translate;
        let [rx, ry, rz] = scene_transform.rotate;
        let [sx, sy, sz] = scene_transform.scale;
        Self::new()
            .translate(tx, ty, tz)
            .rotate(rx, ry, rz)
            .scale(sx, sy, sz)
    }
}

const fn default_camera_fov() -> f64 { 90.0 }
const fn default_camera_near() -> f64 { 10.0 }
const fn default_output_samples() -> u32 { 1 }
const fn default_output_tile_size() -> u32 { 16 }
const fn default_transform_scale() -> [f64; 3] { [1.0, 1.0, 1.0] }
