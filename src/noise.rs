use rayon::prelude::*;
use wide::{bytemuck::cast, f32x4, i32x4, u32x4};

use crate::palette;

const ROWS_PER_CHUNK: usize = 16;

const C1: u32 = 0x0100_0193;
const C2: u32 = 0x8da6_b343;
const C3: u32 = 0xd816_3841;

fn finalize(h: u32x4) -> u32x4 {
    let h = h * u32x4::splat(0x2c1b_3c6d);
    h ^ (h >> 15)
}

fn field_to_unit(field: u32x4) -> f32x4 {
    // 10-bit field in [0, 1023] mapped to [-1, 1).
    let v: i32x4 = cast(field & u32x4::splat(0x3ff));
    f32x4::from_i32x4(v) * f32x4::splat(2.0 / 1023.0) - f32x4::splat(1.0)
}

const SKEW: f32 = 1.0 / 3.0;
const UNSKEW: f32 = 1.0 / 6.0;

fn surflet_x4(h: u32x4, px: f32x4, py: f32x4, pz: f32x4) -> f32x4 {
    let gx = field_to_unit(h);
    let gy = field_to_unit(h >> 10);
    let gz = field_to_unit(h >> 21);

    let t = f32x4::splat(1.0) - (px * px + py * py + pz * pz) * f32x4::splat(2.0);
    let t2 = t * t;
    let t4 = t2 * t2;
    let dot = gx * px + gy * py + gz * pz;
    let contrib = (f32x4::splat(2.0) * t2 + t4) * dot;
    t.simd_gt(f32x4::splat(0.0))
        .blend(contrib, f32x4::splat(0.0))
}

fn simplex_3d_x4(x: f32x4, y: f32x4, z: f32x4, seed: u32) -> f32x4 {
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

    let ix: u32x4 = cast(sx.round_int());
    let iy: u32x4 = cast(sy.round_int());
    let iz: u32x4 = cast(sz.round_int());

    let c1 = u32x4::splat(C1);
    let c2 = u32x4::splat(C2);
    let c3 = u32x4::splat(C3);
    let zc = u32x4::splat(0);

    // Base products shared across all corners (offsets only add C1/C2/C3).
    let base = ix * c1 + iy * c2 + iz * c3 + u32x4::splat(seed);

    // Offset-constant contributions, selected without multiply.
    let m1x: u32x4 = cast(o1x.simd_gt(zero));
    let m1y: u32x4 = cast(o1y.simd_gt(zero));
    let m1z: u32x4 = cast(o1z.simd_gt(zero));
    let m2x: u32x4 = cast(o2x.simd_gt(zero));
    let m2y: u32x4 = cast(o2y.simd_gt(zero));
    let m2z: u32x4 = cast(o2z.simd_gt(zero));

    // Compute each corner to completion to keep live ranges short and reduce
    // register spilling.
    let s0 = surflet_x4(finalize(base), x0, y0, z0);

    let x1 = x0 - o1x + unskew;
    let y1 = y0 - o1y + unskew;
    let z1 = z0 - o1z + unskew;
    let h1 = finalize(base + m1x.blend(c1, zc) + m1y.blend(c2, zc) + m1z.blend(c3, zc));
    let s1 = surflet_x4(h1, x1, y1, z1);

    let x2 = x0 - o2x + f32x4::splat(2.0 * UNSKEW);
    let y2 = y0 - o2y + f32x4::splat(2.0 * UNSKEW);
    let z2 = z0 - o2z + f32x4::splat(2.0 * UNSKEW);
    let h2 = finalize(base + m2x.blend(c1, zc) + m2y.blend(c2, zc) + m2z.blend(c3, zc));
    let s2 = surflet_x4(h2, x2, y2, z2);

    let x3 = x0 - one + f32x4::splat(3.0 * UNSKEW);
    let y3 = y0 - one + f32x4::splat(3.0 * UNSKEW);
    let z3 = z0 - one + f32x4::splat(3.0 * UNSKEW);
    let s3 = surflet_x4(finalize(base + c1 + c2 + c3), x3, y3, z3);

    (s0 + s1) + (s2 + s3)
}

pub struct NoiseField {
    seed: u32,
    scale: f32,
    speed: f32,
}

impl NoiseField {
    pub fn new(seed: u32, scale: f64, speed: f64) -> Self {
        Self {
            seed,
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

                    let compute = |x: usize| -> [u32; 4] {
                        let base = x as f32 * self.scale;
                        let xv = f32x4::new([
                            base,
                            base + self.scale,
                            base + 2.0 * self.scale,
                            base + 3.0 * self.scale,
                        ]);
                        let noise = simplex_3d_x4(xv, yv, zv, self.seed);
                        let wrapped =
                            noise - (noise * f32x4::splat(2.0)).floor() * f32x4::splat(0.5);
                        let hue = wrapped * f32x4::splat(360.0 / 0.5);
                        cast(palette::hsl_to_rgb_x4(hue, 0.75, 0.5))
                    };

                    let mut x = 0usize;
                    let mut chunks = row.chunks_exact_mut(4);
                    for out in &mut chunks {
                        out.copy_from_slice(&compute(x));
                        x += 4;
                    }

                    let tail = chunks.into_remainder();
                    if !tail.is_empty() {
                        let pixels = compute(x);
                        tail.copy_from_slice(&pixels[..tail.len()]);
                    }
                }
            });
    }
}
