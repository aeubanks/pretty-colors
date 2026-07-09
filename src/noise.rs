use noise::{NoiseFn, Simplex};

use crate::palette::{self, Rgb};

pub struct NoiseField {
    perlin_hue: Simplex,
    perlin_sat: Simplex,
    perlin_light: Simplex,
    scale: f64,
    speed: f64,
}

impl NoiseField {
    pub fn new(seed: u32, scale: f64, speed: f64) -> Self {
        Self {
            perlin_hue: Simplex::new(seed),
            perlin_sat: Simplex::new(seed.wrapping_add(1)),
            perlin_light: Simplex::new(seed.wrapping_add(2)),
            scale,
            speed,
        }
    }

    pub fn fill(&self, buffer: &mut [u32], width: u32, height: u32, t: f64) {
        let z = t * self.speed * 0.3;
        assert_eq!(buffer.len(), (width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let v_hue = self
                    .perlin_hue
                    .get([x as f64 * self.scale, y as f64 * self.scale, z]);
                let v_sat = self
                    .perlin_sat
                    .get([x as f64 * self.scale, y as f64 * self.scale, z]);
                let v_light =
                    self.perlin_light
                        .get([x as f64 * self.scale, y as f64 * self.scale, z]);

                let hue = v_hue.rem_euclid(0.5) / 0.5 * 360.0;
                let lightness = 0.25 + (v_light + 1.0) / 2.0 * 0.5;
                let saturation = 0.5 + (v_sat + 1.0) / 2.0 * 0.5;

                let Rgb { r, g, b } = palette::hsl_to_rgb(hue, saturation, lightness);
                let idx = (y * width + x) as usize;
                buffer[idx] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
            }
        }
    }
}
