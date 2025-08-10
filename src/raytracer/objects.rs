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
        match OBJECT_TYPES.get("cone") {
            Some(object_new_fn) => {
                Ok(
                    Object {
                        transform: Transform::identity()
                            .rotate(-60.0, 0.0, 0.0),
                        inner: object_new_fn(),
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

pub trait ObjectType {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

static OBJECT_TYPES: LazyLock<HashMap<String, fn() -> Box<dyn ObjectType + Sync + Send>>> = LazyLock::new(|| {
    HashMap::from([
        ("cone".to_string(), (|| { Box::new(Cone) }) as fn() -> Box<dyn ObjectType + Sync + Send>),
        ("cube".to_string(), || { Box::new(Cube) }),
        ("cylinder".to_string(), || { Box::new(Cylinder) }),
        ("plane".to_string(), || { Box::new(Plane) }),
        ("sphere".to_string(), || { Box::new(Sphere) }),
    ])
});

struct Cone;
struct Cube;
struct Cylinder;
struct Plane;
struct Sphere;

impl ObjectType for Cone {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let mut dists = [
            solve_linear(
                ray.direction.0 * ray.direction.0 +
                    ray.direction.1 * ray.direction.1 -
                    ray.direction.2 * ray.direction.2 / 4.0,
                2.0 * (
                    ray.direction.0 * ray.origin.0 +
                    ray.direction.1 * ray.origin.1 -
                    ray.direction.2 * (ray.origin.2 + 0.5) / 4.0),
                ray.origin.0 * ray.origin.0 +
                    ray.origin.1 * ray.origin.1 -
                    (ray.origin.2 + 0.5).powf(2.0) / 4.0,
            ),
            -(ray.origin.2 - 0.5) / ray.direction.2,
        ];

        if (ray.direction.2 * dists[0] + ray.origin.2).abs() > 0.5 {
            dists[0] = f64::NAN
        }
        if ray.direction.2.abs() < f64::EPSILON ||
            (dists[1] * ray.direction.0 + ray.origin.0).powf(2.0) +
            (dists[1] * ray.direction.1 + ray.origin.1).powf(2.0) > 0.25 {
            dists[1] = f64::NAN;
        }

        let distance = dists.iter().filter(|d| !(d.is_nan() || **d < 0.0)).min_by(|a, b| a.total_cmp(b));
        if distance.is_none() {
            return None;
        }

        let distance = *distance.unwrap();
        Some(Hit { distance })
    }
}

impl ObjectType for Cube {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let inv_dir = (1.0 / ray.direction.0, 1.0 / ray.direction.1, 1.0 / ray.direction.2);

        let t1 = ((-0.5 - ray.origin.0) * inv_dir.0, (-0.5 - ray.origin.1) * inv_dir.1, (-0.5 - ray.origin.2) * inv_dir.2);
        let t2 = ((0.5 - ray.origin.0) * inv_dir.0, (0.5 - ray.origin.1) * inv_dir.1, (0.5 - ray.origin.2) * inv_dir.2);
        let tmin = *[f64::min(t1.0, t2.0), f64::min(t1.1, t2.1), f64::min(t1.2, t2.2)].iter().max_by(|a, b| a.total_cmp(b)).unwrap();
        let tmax = *[f64::max(t1.0, t2.0), f64::max(t1.1, t2.1), f64::max(t1.2, t2.2)].iter().min_by(|a, b| a.total_cmp(b)).unwrap();

        if tmax < 1.0 || tmin > tmax {
            return None;
        }

        Some(Hit { distance: tmin })
    }
}

impl ObjectType for Cylinder {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let mut dists = [
            solve_linear(
                ray.direction.0 * ray.direction.0 +
                    ray.direction.1 * ray.direction.1,
                2.0 * (
                    ray.direction.0 * ray.origin.0 +
                    ray.direction.1 * ray.origin.1),
                ray.origin.0 * ray.origin.0 +
                    ray.origin.1 * ray.origin.1 -
                    1.0,
            ),
            -(ray.origin.2 - 0.5) / ray.direction.2,
            -(ray.origin.2 + 0.5) / ray.direction.2,
        ];

        if (ray.direction.2 * dists[0] + ray.origin.2).abs() > 0.5 {
            dists[0] = f64::NAN
        }
        for i in 1..=2 {
            if ray.direction.2.abs() < f64::EPSILON ||
                (dists[i] * ray.direction.0 + ray.origin.0).powf(2.0) +
                (dists[i] * ray.direction.1 + ray.origin.1).powf(2.0) > 1.0 {
                dists[i] = f64::NAN;
            }
        }

        let distance = dists.iter().filter(|d| !(d.is_nan() || **d < 0.0)).min_by(|a, b| a.total_cmp(b));
        if distance.is_none() {
            return None;
        }

        let distance = *distance.unwrap();
        Some(Hit { distance })
    }
}

impl ObjectType for Plane {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        if ray.direction.2.abs() < f64::EPSILON {
            return None;
        }
        let distance = -ray.origin.2 / ray.direction.2;
        if  (ray.origin.0 + ray.direction.0 * distance).abs() > 0.5 ||
            (ray.origin.1 + ray.direction.1 * distance).abs() > 0.5 {
            return None;
        }
        Some(Hit { distance })
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
        Some(Hit { distance })
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
