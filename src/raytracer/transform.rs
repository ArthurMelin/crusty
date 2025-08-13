use crate::raytracer::utils::{matmul414, matmul444};

pub struct Transform {
    matrix: [[f64; 4]; 4],
    invmatrix: [[f64; 4]; 4],
}

impl Transform {
    pub const fn new() -> Transform {
        Self::identity()
    }

    pub const fn identity() -> Transform {
        Transform {
            matrix: [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
            invmatrix: [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        }
    }

    pub const fn inverse(&self) -> Transform {
        Transform {
            matrix: self.invmatrix,
            invmatrix: self.matrix,
        }
    }

    pub const fn translate(&self, x: f64, y: f64, z: f64) -> Transform {
        let translate = [
            [1., 0., 0., x],
            [0., 1., 0., y],
            [0., 0., 1., z],
            [0., 0., 0., 1.],
        ];
        let invtranslate = [
            [1., 0., 0., -x],
            [0., 1., 0., -y],
            [0., 0., 1., -z],
            [0., 0., 0., 1.],
        ];
        Transform {
            matrix: matmul444(&self.matrix, &translate),
            invmatrix: matmul444(&invtranslate, &self.invmatrix),
        }
    }

    pub fn rotate(&self, x: f64, y: f64, z: f64) -> Transform {
        // FIXME: can't make this fn const because sin_cos() is non-const
        let (sx, cx) = x.to_radians().sin_cos();
        let (sy, cy) = y.to_radians().sin_cos();
        let (sz, cz) = z.to_radians().sin_cos();

        let rotate = [
            [cy*cz,             -cy*sz,             sy,         0.],
            [sx*sy*cz+cx*sz,    -sx*sy*sz+cx*cz,    -sx*cy,     0.],
            [-cx*sy*cz+sx*sz,   cx*sy*sz+sx*cz,     cx*cy,      0.],
            [0.,                0.,                 0.,         1.],
        ];
        let invrotate = [
            [cy*cz,     sx*sy*cz+cx*sz,     -cx*sy*cz+sx*sz,    0.],
            [-cy*sz,    -sx*sy*sz+cx*cz,    cx*sy*sz+sx*cz,     0.],
            [sy,        -sx*cy,             cx*cy,              0.],
            [0.,        0.,                 0.,                 1.],
        ];

        Transform {
            matrix: matmul444(&self.matrix, &rotate),
            invmatrix: matmul444(&invrotate, &self.invmatrix),
        }
    }

    pub const fn scale(&self, x: f64, y: f64, z: f64) -> Transform {
        let scale = [
            [x,  0., 0., 0.],
            [0., y,  0., 0.],
            [0., 0., z,  0.],
            [0., 0., 0., 1.],
        ];
        let invscale = [
            [1./x,  0.,     0.,     0.],
            [0.,    1./y,   0.,     0.],
            [0.,    0.,     1./z,   0.],
            [0.,    0.,     0.,     1.],
        ];
        Transform {
            matrix: matmul444(&self.matrix, &scale),
            invmatrix: matmul444(&invscale, &self.invmatrix),
        }
    }

    #[inline]
    pub const fn apply(&self, to: (f64, f64, f64)) -> (f64, f64, f64) {
        let [x, y, z, _] = matmul414(&self.matrix, &[to.0, to.1, to.2, 1.0]);

        (x, y, z)
    }

    #[inline]
    pub const fn apply_notranslate(&self, to: (f64, f64, f64)) -> (f64, f64, f64) {
        let [x, y, z, _] = matmul414(&self.matrix, &[to.0, to.1, to.2, 0.0]);

        (x, y, z)
    }
}
