use std::collections::HashMap;
use std::sync::LazyLock;

use crate::raytracer::{Hit, Ray, Transform};

pub struct Object {
    transform: Transform,
    inner: Box<dyn ObjectType + Send + Sync>
}

impl Object {
    pub fn new() -> Result<Object, &'static str> {
        // TODO
        match OBJECT_TYPES.get("sphere") {
            Some(new_fn) => {
                Ok(
                    Object {
                        transform: Transform::identity(),
                        inner: new_fn(),
                    }
                )
            }
            _ => Err("object type not found"),
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let tmp = Ray {
            origin: self.transform.inverse().apply(ray.origin),
            direction: self.transform.inverse().apply_notranslate(ray.direction),
        };
        self.inner.intersect(&tmp)
    }
}

trait ObjectType {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

static OBJECT_TYPES: LazyLock<HashMap<String, fn() -> Box<dyn ObjectType + Sync + Send>>> = LazyLock::new(|| {
    HashMap::from([
        ("plane".to_string(), (|| { Box::new(Plane) }) as fn() -> Box<dyn ObjectType + Sync + Send>),
        ("sphere".to_string(), || { Box::new(Sphere) }),
    ])
});

struct Plane;
struct Sphere;

impl ObjectType for Plane {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        if ray.direction.2.abs() < f64::EPSILON {
            return None;
        }
        let distance = -ray.origin.2 / ray.direction.2;
        if  (ray.origin.0 + ray.direction.0 * distance) > 0.5 ||
            (ray.origin.1 + ray.direction.1 * distance) > 0.5 {
            return None;
        }
        Some(
            Hit {
                distance,
            }
        )
    }
}

impl ObjectType for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let distance = solve_linear(
            ray.direction.0 * ray.direction.0 +
                ray.direction.1 * ray.direction.1 +
                ray.direction.2 * ray.direction.2,
            2.0 * (
                ray.direction.0 * ray.origin.0 +
                ray.direction.1 * ray.origin.1 +
                ray.direction.2 * ray.origin.2),
            ray.origin.0 * ray.origin.0 +
                ray.origin.1 * ray.origin.1 +
                ray.origin.2 * ray.origin.2 -
                1.0,
        );
        if distance.is_nan() || distance <= 0.0 {
            return None;
        }
        Some(
            Hit {
                distance,
            }
        )
    }
}

#[inline]
fn solve_linear(a: f64, b: f64, c: f64) -> f64 {
    let delta = b * b - 4.0 * a * c;
    match delta {
        f64::EPSILON.. => f64::min(
            (-b + delta.sqrt()) / (2.0 * a),
            (-b - delta.sqrt()) / (2.0 * a),
        ),
        0.0.. => -b / (2.0 * a),
        _ => { f64::NAN },
    }
}
