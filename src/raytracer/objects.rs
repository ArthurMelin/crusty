use crate::raytracer::{Ray, Transform};
use crate::raytracer::utils::{vec3add, vec3norm, vec3scale};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::LazyLock;

const HALF_EPSILON: f64 = 0.49999999;

static OBJECT_TYPES: LazyLock<HashMap<String, fn() -> Box<dyn ObjectType + Sync + Send>>> =
    LazyLock::new(|| {
        HashMap::from([
            ("cone".to_string(), (|| { Box::new(Cone) }) as fn() -> Box<dyn ObjectType + Sync + Send>),
            ("cube".to_string(), || { Box::new(Cube) }),
            ("cylinder".to_string(), || { Box::new(Cylinder) }),
            ("plane".to_string(), || { Box::new(Plane) }),
            ("sphere".to_string(), || { Box::new(Sphere) }),
        ])
    });

pub struct Object {
    transform: Transform,
    inner: Box<dyn ObjectType + Send + Sync>,
}

pub trait ObjectType {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

struct Cone;
struct Cube;
struct Cylinder;
struct Plane;
struct Sphere;

#[derive(Clone, Copy)]
pub struct ObjectHit<'a> {
    pub ray: Ray,
    pub object: &'a Object,
    pub hit: Hit,
}

#[derive(Clone, Copy)]
pub struct Hit {
    pub distance: f64,
    pub intersection: (f64, f64, f64),
    pub normal: (f64, f64, f64),
    pub uv: (f64, f64),
}

impl Object {
    pub fn new(type_name: &String, transform: Transform) -> Result<Object, String> {
        match OBJECT_TYPES.get(type_name) {
            Some(object_new_fn) => Ok(Object {
                transform,
                inner: object_new_fn(),
            }),
            _ => Err(format!("Object type {} not found", type_name)),
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<ObjectHit> {
        let tmp = Ray {
            origin: self.transform.inverse().apply(ray.origin),
            direction: self.transform.inverse().apply_notranslate(ray.direction),
        };
        match self.inner.intersect(&tmp) {
            Some(hit) => {
                let mut hit = hit;
                hit.intersection = self.transform.apply(hit.intersection);
                hit.normal = vec3norm(self.transform.apply_notranslate(hit.normal));

                Some(ObjectHit {
                    ray: *ray,
                    object: self,
                    hit,
                })
            }
            _ => None,
        }
    }
}

impl ObjectType for Cone {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let mut dists = [
            solve_linear(
                ray.direction.0 * ray.direction.0 +
                    ray.direction.1 * ray.direction.1 -
                    ray.direction.2 * ray.direction.2 / 4.0,
                2.0 * (
                    ray.direction.0 * ray.origin.0 +
                    ray.direction.1 * ray.origin.1 +
                    ray.direction.2 * (0.5 - ray.origin.2) / 4.0),
                ray.origin.0 * ray.origin.0 +
                    ray.origin.1 * ray.origin.1 -
                    (0.5 - ray.origin.2) * (0.5 - ray.origin.2) / 4.0,
            ),
            -(0.5 + ray.origin.2) / ray.direction.2,
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
        let intersection = intersection(ray, distance);
        let normal = if intersection.2 >= HALF_EPSILON {
            (0.0, 0.0, 1.0)
        } else {
            vec3norm((intersection.0, intersection.1, intersection.2))
        };
        let uv = (
            0.5 - f64::atan2(intersection.0, intersection.1) / (2.0 * PI),
            intersection.2 + 0.5,
        );

        Some(Hit {
            distance,
            intersection,
            normal,
            uv,
        })
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

        let distance = tmin;
        let intersection = intersection(ray, distance);
        let (normal, uv) = match intersection {
            (x, y, z) if x <= -HALF_EPSILON => ((-1.0, 0.0, 0.0), (0.5 - y, z + 0.5)),
            (x, y, z) if x >= HALF_EPSILON => ((1.0, 0.0, 0.0), (y + 0.5, z + 0.5)),
            (x, y, z) if y <= -HALF_EPSILON => ((0.0, -1.0, 0.0), (x + 0.5, z + 0.5)),
            (x, y, z) if y >= HALF_EPSILON => ((0.0, 1.0, 0.0), (0.5 - x, z + 0.5)),
            (x, y, z) if z <= -HALF_EPSILON => ((0.0, 0.0, -1.0), (x + 0.5, 0.5 - y)),
            (x, y, z) if z >= HALF_EPSILON => ((0.0, 0.0, 1.0), (x + 0.5, y + 0.5)),
            _ => unreachable!(),
        };

        Some(Hit {
            distance,
            intersection,
            normal,
            uv,
        })
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
                    0.25,
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
                (dists[i] * ray.direction.1 + ray.origin.1).powf(2.0) > 0.25 {
                dists[i] = f64::NAN;
            }
        }

        let distance = dists.iter().filter(|d| !(d.is_nan() || **d < 0.0)).min_by(|a, b| a.total_cmp(b));
        if distance.is_none() {
            return None;
        }

        let distance = *distance.unwrap();
        let intersection = intersection(ray, distance);
        let normal = if intersection.2 <= -HALF_EPSILON {
            (0.0, 0.0, -1.0)
        } else if intersection.2 >= HALF_EPSILON {
            (0.0, 0.0, 1.0)
        } else {
            vec3norm((intersection.0, intersection.1, 0.0))
        };
        let uv = (
            0.5 - f64::atan2(intersection.0, intersection.1) / (2.0 * PI),
            intersection.2 + 0.5,
        );

        Some(Hit {
            distance,
            intersection,
            normal,
            uv,
        })
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
        let intersection = intersection(ray, distance);
        let normal = (0.0, 0.0, if intersection.2 < 0.0 { 1.0 } else { -1.0 });
        let uv = (intersection.0 + 0.5, intersection.1 + 0.5);

        Some(Hit {
            distance,
            intersection,
            normal,
            uv,
        })
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
                0.25,
        );
        if distance.is_nan() || distance <= 0.0 {
            return None;
        }
        let intersection = intersection(ray, distance);
        let normal = vec3norm(intersection);
        let uv = (
            0.5 - f64::atan2(normal.0, normal.1) / (2.0 * PI),
            normal.2 * 0.5 + 0.5,
        );

        Some(Hit {
            distance,
            intersection,
            normal,
            uv,
        })
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
        _ => f64::NAN,
    }
}

fn intersection(ray: &Ray, distance: f64) -> (f64, f64, f64) {
    vec3add(vec3scale(ray.direction, distance), ray.origin)
}
