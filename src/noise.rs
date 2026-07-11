use rayon::prelude::*;

use crate::palette::{self, Rgb};

const TABLE_SIZE: usize = 256;
const ROWS_PER_CHUNK: usize = 16;

struct PermutationTable {
    values: [u8; TABLE_SIZE],
}

impl PermutationTable {
    fn new(seed: u32) -> Self {
        let mut values = [0u8; TABLE_SIZE];
        for (i, v) in values.iter_mut().enumerate() {
            *v = i as u8;
        }

        let mut state = seed as u64 ^ 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        for i in (1..TABLE_SIZE).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            values.swap(i, j);
        }

        Self { values }
    }

    fn hash(&self, x: isize, y: isize, z: isize) -> usize {
        let a = (x & 0xff) as usize;
        let b = self.values[a] as usize ^ (y & 0xff) as usize;
        let c = self.values[b] as usize ^ (z & 0xff) as usize;
        self.values[c] as usize
    }
}

const DIAG: f32 = std::f32::consts::FRAC_1_SQRT_2;
const DIAG2: f32 = 0.577_350_3;

fn grad3(index: usize) -> [f32; 3] {
    match index % 32 {
        0 | 12 => [DIAG, DIAG, 0.0],
        1 | 13 => [-DIAG, DIAG, 0.0],
        2 | 14 => [DIAG, -DIAG, 0.0],
        3 | 15 => [-DIAG, -DIAG, 0.0],
        4 | 16 => [DIAG, 0.0, DIAG],
        5 | 17 => [-DIAG, 0.0, DIAG],
        6 | 18 => [DIAG, 0.0, -DIAG],
        7 | 19 => [-DIAG, 0.0, -DIAG],
        8 | 20 => [0.0, DIAG, DIAG],
        9 | 21 => [0.0, -DIAG, DIAG],
        10 | 22 => [0.0, DIAG, -DIAG],
        11 | 23 => [0.0, -DIAG, -DIAG],
        24 => [DIAG2, DIAG2, DIAG2],
        25 => [-DIAG2, DIAG2, DIAG2],
        26 => [DIAG2, -DIAG2, DIAG2],
        27 => [-DIAG2, -DIAG2, DIAG2],
        28 => [DIAG2, DIAG2, -DIAG2],
        29 => [-DIAG2, DIAG2, -DIAG2],
        30 => [DIAG2, -DIAG2, -DIAG2],
        _ => [-DIAG2, -DIAG2, -DIAG2],
    }
}

const SKEW: f32 = 1.0 / 3.0;
const UNSKEW: f32 = 1.0 / 6.0;

fn surflet(grad: [f32; 3], px: f32, py: f32, pz: f32) -> f32 {
    let t = 1.0 - (px * px + py * py + pz * pz) * 2.0;
    if t > 0.0 {
        let t2 = t * t;
        let t4 = t2 * t2;
        let dot = grad[0] * px + grad[1] * py + grad[2] * pz;
        (2.0 * t2 + t4) * dot
    } else {
        0.0
    }
}

fn simplex_3d(x: f32, y: f32, z: f32, hasher: &PermutationTable) -> f32 {
    let skew = (x + y + z) * SKEW;
    let sx = (x + skew).floor();
    let sy = (y + skew).floor();
    let sz = (z + skew).floor();
    let ix = sx as isize;
    let iy = sy as isize;
    let iz = sz as isize;

    let unskew = (sx + sy + sz) * UNSKEW;
    let x0 = x - (sx - unskew);
    let y0 = y - (sy - unskew);
    let z0 = z - (sz - unskew);

    let (o1x, o1y, o1z, o2x, o2y, o2z) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else if y0 < z0 {
        (0, 0, 1, 0, 1, 1)
    } else if x0 < z0 {
        (0, 1, 0, 0, 1, 1)
    } else {
        (0, 1, 0, 1, 1, 0)
    };

    let x1 = x0 - o1x as f32 + UNSKEW;
    let y1 = y0 - o1y as f32 + UNSKEW;
    let z1 = z0 - o1z as f32 + UNSKEW;
    let x2 = x0 - o2x as f32 + 2.0 * UNSKEW;
    let y2 = y0 - o2y as f32 + 2.0 * UNSKEW;
    let z2 = z0 - o2z as f32 + 2.0 * UNSKEW;
    let x3 = x0 - 1.0 + 3.0 * UNSKEW;
    let y3 = y0 - 1.0 + 3.0 * UNSKEW;
    let z3 = z0 - 1.0 + 3.0 * UNSKEW;

    let gi0 = hasher.hash(ix, iy, iz);
    let gi1 = hasher.hash(ix + o1x, iy + o1y, iz + o1z);
    let gi2 = hasher.hash(ix + o2x, iy + o2y, iz + o2z);
    let gi3 = hasher.hash(ix + 1, iy + 1, iz + 1);

    surflet(grad3(gi0), x0, y0, z0)
        + surflet(grad3(gi1), x1, y1, z1)
        + surflet(grad3(gi2), x2, y2, z2)
        + surflet(grad3(gi3), x3, y3, z3)
}

pub struct NoiseField {
    hasher: PermutationTable,
    scale: f32,
    speed: f32,
}

impl NoiseField {
    pub fn new(seed: u32, scale: f64, speed: f64) -> Self {
        Self {
            hasher: PermutationTable::new(seed),
            scale: scale as f32,
            speed: speed as f32,
        }
    }

    pub fn fill(&self, buffer: &mut [u32], width: u32, height: u32, t: f64) {
        let z = (t as f32) * self.speed * 0.3;
        assert_eq!(buffer.len(), (width * height) as usize);

        buffer
            .par_chunks_mut(width as usize * ROWS_PER_CHUNK)
            .enumerate()
            .for_each(|(chunk, block)| {
                let y0 = chunk * ROWS_PER_CHUNK;
                for (dy, row) in block.chunks_mut(width as usize).enumerate() {
                    let sy = (y0 + dy) as f32 * self.scale;
                    for x in 0..width {
                        let noise = simplex_3d(x as f32 * self.scale, sy, z, &self.hasher);

                        let wrapped = noise - (noise * 2.0).floor() * 0.5;
                        let hue = wrapped / 0.5 * 360.0;
                        let lightness = 0.5;
                        let saturation = 0.75;

                        let Rgb { r, g, b } = palette::hsl_to_rgb(hue, saturation, lightness);
                        row[x as usize] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
                    }
                }
            });
    }
}
