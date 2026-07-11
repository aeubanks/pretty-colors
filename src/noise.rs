use noise::{NoiseFn, Simplex};
use rayon::prelude::*;

use crate::palette::{self, Rgb};

pub struct NoiseField {
    simplex: Simplex,
    scale: f64,
    speed: f64,
}

impl NoiseField {
    pub fn new(seed: u32, scale: f64, speed: f64) -> Self {
        Self {
            simplex: Simplex::new(seed),
            scale,
            speed,
        }
    }

    pub fn fill(&self, buffer: &mut [u32], width: u32, height: u32, t: f64) {
        let z = t * self.speed * 0.3;
        assert_eq!(buffer.len(), (width * height) as usize);

        buffer
            .par_chunks_mut(width as usize)
            .enumerate()
            .for_each(|(y, row)| {
                let sy = y as f64 * self.scale;
                for x in 0..width {
                    let noise = self.simplex.get([x as f64 * self.scale, sy, z]);

                    let wrapped = noise - (noise * 2.0).floor() * 0.5;
                    let hue = wrapped / 0.5 * 360.0;
                    let lightness = 0.5;
                    let saturation = 0.75;

                    let Rgb { r, g, b } = palette::hsl_to_rgb(hue, saturation, lightness);
                    row[x as usize] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
                }
            });
    }
}
