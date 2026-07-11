use rayon::prelude::*;
use wide::f32x4;

use crate::palette;

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

fn surflet_x4(gx: f32x4, gy: f32x4, gz: f32x4, px: f32x4, py: f32x4, pz: f32x4) -> f32x4 {
    let t = f32x4::splat(1.0) - (px * px + py * py + pz * pz) * f32x4::splat(2.0);
    let t2 = t * t;
    let t4 = t2 * t2;
    let dot = gx * px + gy * py + gz * pz;
    let contrib = (f32x4::splat(2.0) * t2 + t4) * dot;
    t.simd_gt(f32x4::splat(0.0))
        .blend(contrib, f32x4::splat(0.0))
}

fn simplex_3d_x4(x: f32x4, y: f32x4, z: f32x4, hasher: &PermutationTable) -> f32x4 {
    let one = f32x4::splat(1.0);
    let zero = f32x4::splat(0.0);
    let unskew = f32x4::splat(UNSKEW);

    let skew = (x + y + z) * f32x4::splat(SKEW);
    let sx = (x + skew).floor();
    let sy = (y + skew).floor();
    let sz = (z + skew).floor();

    let unskew_cell = (sx + sy + sz) * unskew;
    let x0 = x - (sx - unskew_cell);
    let y0 = y - (sy - unskew_cell);
    let z0 = z - (sz - unskew_cell);

    // Branchless corner ordering, mirroring the scalar decision tree:
    //   x0>=y0 && y0>=z0            -> o1=(1,0,0) o2=(1,1,0)
    //   x0>=y0 && x0>=z0            -> o1=(1,0,0) o2=(1,0,1)
    //   x0>=y0                      -> o1=(0,0,1) o2=(1,0,1)
    //   x0<y0  && y0<z0            -> o1=(0,0,1) o2=(0,1,1)
    //   x0<y0  && x0<z0            -> o1=(0,1,0) o2=(0,1,1)
    //   else                       -> o1=(0,1,0) o2=(1,1,0)
    let ge_xy = x0.simd_ge(y0);
    let ge_yz = y0.simd_ge(z0);
    let ge_xz = x0.simd_ge(z0);
    let lt_yz = y0.simd_lt(z0);
    let lt_xz = x0.simd_lt(z0);

    let o1x = ge_xy.blend(ge_yz.blend(one, ge_xz.blend(one, zero)), zero);
    let o1y = ge_xy.blend(zero, lt_yz.blend(zero, one));
    let o1z = ge_xy.blend(
        ge_yz.blend(zero, ge_xz.blend(zero, one)),
        lt_yz.blend(one, zero),
    );
    let o2x = ge_xy.blend(one, lt_yz.blend(zero, lt_xz.blend(zero, one)));
    let o2y = ge_xy.blend(ge_yz.blend(one, zero), one);
    let o2z = ge_xy.blend(
        ge_yz.blend(zero, one),
        lt_yz.blend(one, lt_xz.blend(one, zero)),
    );

    let x1 = x0 - o1x + unskew;
    let y1 = y0 - o1y + unskew;
    let z1 = z0 - o1z + unskew;
    let x2 = x0 - o2x + f32x4::splat(2.0 * UNSKEW);
    let y2 = y0 - o2y + f32x4::splat(2.0 * UNSKEW);
    let z2 = z0 - o2z + f32x4::splat(2.0 * UNSKEW);
    let x3 = x0 - one + f32x4::splat(3.0 * UNSKEW);
    let y3 = y0 - one + f32x4::splat(3.0 * UNSKEW);
    let z3 = z0 - one + f32x4::splat(3.0 * UNSKEW);

    let sxa = sx.to_array();
    let sya = sy.to_array();
    let sza = sz.to_array();
    let o1xa = o1x.to_array();
    let o1ya = o1y.to_array();
    let o1za = o1z.to_array();
    let o2xa = o2x.to_array();
    let o2ya = o2y.to_array();
    let o2za = o2z.to_array();

    let mut g0 = [[0.0f32; 4]; 3];
    let mut g1 = [[0.0f32; 4]; 3];
    let mut g2 = [[0.0f32; 4]; 3];
    let mut g3 = [[0.0f32; 4]; 3];

    for l in 0..4 {
        let ix = sxa[l] as isize;
        let iy = sya[l] as isize;
        let iz = sza[l] as isize;

        let grad0 = grad3(hasher.hash(ix, iy, iz));
        let grad1 = grad3(hasher.hash(
            ix + o1xa[l] as isize,
            iy + o1ya[l] as isize,
            iz + o1za[l] as isize,
        ));
        let grad2 = grad3(hasher.hash(
            ix + o2xa[l] as isize,
            iy + o2ya[l] as isize,
            iz + o2za[l] as isize,
        ));
        let grad3v = grad3(hasher.hash(ix + 1, iy + 1, iz + 1));

        for c in 0..3 {
            g0[c][l] = grad0[c];
            g1[c][l] = grad1[c];
            g2[c][l] = grad2[c];
            g3[c][l] = grad3v[c];
        }
    }

    let s0 = surflet_x4(
        f32x4::new(g0[0]),
        f32x4::new(g0[1]),
        f32x4::new(g0[2]),
        x0,
        y0,
        z0,
    );
    let s1 = surflet_x4(
        f32x4::new(g1[0]),
        f32x4::new(g1[1]),
        f32x4::new(g1[2]),
        x1,
        y1,
        z1,
    );
    let s2 = surflet_x4(
        f32x4::new(g2[0]),
        f32x4::new(g2[1]),
        f32x4::new(g2[2]),
        x2,
        y2,
        z2,
    );
    let s3 = surflet_x4(
        f32x4::new(g3[0]),
        f32x4::new(g3[1]),
        f32x4::new(g3[2]),
        x3,
        y3,
        z3,
    );

    (s0 + s1) + (s2 + s3)
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

        let zv = f32x4::splat(z);
        let width = width as usize;

        buffer
            .par_chunks_mut(width * ROWS_PER_CHUNK)
            .enumerate()
            .for_each(|(chunk, block)| {
                let y0 = chunk * ROWS_PER_CHUNK;
                for (dy, row) in block.chunks_mut(width).enumerate() {
                    let yv = f32x4::splat((y0 + dy) as f32 * self.scale);
                    let mut x = 0usize;
                    while x < width {
                        let base = x as f32 * self.scale;
                        let xv = f32x4::new([
                            base,
                            base + self.scale,
                            base + 2.0 * self.scale,
                            base + 3.0 * self.scale,
                        ]);
                        let noise = simplex_3d_x4(xv, yv, zv, &self.hasher);

                        let wrapped =
                            noise - (noise * f32x4::splat(2.0)).floor() * f32x4::splat(0.5);
                        let hue = wrapped * f32x4::splat(360.0 / 0.5);
                        let pixels = palette::hsl_to_rgb_x4(hue, 0.75, 0.5).to_array();

                        let lanes = (width - x).min(4);
                        row[x..x + lanes].copy_from_slice(&pixels[..lanes]);
                        x += 4;
                    }
                }
            });
    }
}
