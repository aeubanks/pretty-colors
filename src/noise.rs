use noise::{NoiseFn, Perlin};

use crate::palette::{self, Rgb};

pub struct NoiseField {
    perlin: Perlin,
    scale: f64,
    speed: f64,
}

impl NoiseField {
    pub fn new(seed: u32, scale: f64, speed: f64) -> Self {
        Self {
            perlin: Perlin::new(seed),
            scale,
            speed,
        }
    }

    pub fn fill(&self, buffer: &mut [u32], width: u32, height: u32, t: f64) {
        let z = t * self.speed * 0.3;
        assert_eq!(buffer.len(), (width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let v = self.perlin.get([
                    x as f64 * self.scale,
                    y as f64 * self.scale,
                    z,
                ]);

                let hue = ((v + 1.0) / 2.0 * 360.0) % 360.0;
                let lightness = (0.45 + v * 0.15) * 0.1;
                let saturation = 0.75 + v.abs() * 0.2;

                let Rgb { r, g, b } = palette::hsl_to_rgb(hue, saturation, lightness);
                let idx = (y * width + x) as usize;
                buffer[idx] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
            }
        }
    }
}
