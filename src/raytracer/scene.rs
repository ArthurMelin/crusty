use crate::raytracer::Camera;
use crate::raytracer::objects::Object;
use crate::raytracer::transform::Transform;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Scene {
    pub camera: SceneCamera,
    pub output: SceneOutput,
    pub objects: Vec<SceneObject>,
}

#[derive(Deserialize)]
pub struct SceneCamera {
    #[serde(default = "default_camera_fov")]
    pub fov: f64,
    #[serde(default = "default_camera_near")]
    pub near: f64,
    pub transform: SceneTransform,
}

#[derive(Deserialize)]
pub struct SceneOutput {
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_output_tile_size")]
    pub tile_size: u32,
}

#[derive(Deserialize)]
pub struct SceneObject {
    #[serde(rename = "type")]
    pub type_name: String,
    pub transform: SceneTransform,
}

#[derive(Deserialize)]
pub struct SceneTransform {
    #[serde(default)]
    pub translate: [f64; 3],
    #[serde(default)]
    pub rotate: [f64; 3],
    #[serde(default = "default_transform_scale")]
    pub scale: [f64; 3],
}

impl From<&SceneCamera> for Camera {
    fn from(scene_camera: &SceneCamera) -> Camera {
        Camera {
            fov: scene_camera.fov,
            near: scene_camera.near,
            transform: Transform::from(&scene_camera.transform),
        }
    }
}

impl TryFrom<&SceneObject> for Object {
    type Error = String;

    fn try_from(scene_object: &SceneObject) -> Result<Object, Self::Error> {
        Object::new(&scene_object.type_name, Transform::from(&scene_object.transform))
    }
}

impl From<&SceneTransform> for Transform {
    fn from(scene_transform: &SceneTransform) -> Transform {
        let [tx, ty, tz] = scene_transform.translate;
        let [rx, ry, rz] = scene_transform.rotate;
        let [sx, sy, sz] = scene_transform.scale;
        Transform::new()
            .translate(tx, ty, tz)
            .rotate(rx, ry, rz)
            .scale(sx, sy, sz)
    }
}

const fn default_camera_fov() -> f64 {
    90.0
}

const fn default_camera_near() -> f64 {
    10.0
}

const fn default_output_tile_size() -> u32 {
    16
}

const fn default_transform_scale() -> [f64; 3] {
    [1.0, 1.0, 1.0]
}
