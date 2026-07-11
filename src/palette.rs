use wide::{f32x4, u32x4};

/// Vectorized HSL->RGB for 4 pixels at once, returning packed 0x00RRGGBB.
pub fn hsl_to_rgb_x4(h: f32x4, s: f32, l: f32) -> u32x4 {
    let a = f32x4::splat(s * l.min(1.0 - l));
    let l = f32x4::splat(l);
    let h30 = h * f32x4::splat(1.0 / 30.0);
    let twelve = f32x4::splat(12.0);

    let channel = |n: f32| {
        let k0 = f32x4::splat(n) + h30;
        let k = k0.simd_ge(twelve).blend(k0 - twelve, k0);
        let inner = (k - f32x4::splat(3.0))
            .min(f32x4::splat(9.0) - k)
            .min(f32x4::splat(1.0))
            .max(f32x4::splat(-1.0));
        let v = l - a * inner;
        let scaled = (v * f32x4::splat(255.0))
            .max(f32x4::splat(0.0))
            .min(f32x4::splat(255.0));
        let ints: [i32; 4] = scaled.round_int().to_array();
        u32x4::new([
            ints[0] as u32,
            ints[1] as u32,
            ints[2] as u32,
            ints[3] as u32,
        ])
    };

    let r = channel(0.0);
    let g = channel(8.0);
    let b = channel(4.0);

    (r << 16) | (g << 8) | b
}
