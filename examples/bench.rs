#[path = "../src/noise.rs"]
mod noise;
#[path = "../src/palette.rs"]
mod palette;

use noise::NoiseField;
use std::time::Instant;

fn main() {
    let (w, h) = (800u32, 600u32);
    let field = NoiseField::new(12345, 0.0015, 0.15);
    let mut buf = vec![0u32; (w * h) as usize];

    for i in 0..20 {
        field.fill(&mut buf, w, h, i as f64 * 0.01);
    }

    let frames = 300;
    for _ in 0..3 {
        let start = Instant::now();
        for i in 0..frames {
            field.fill(&mut buf, w, h, i as f64 * 0.01);
        }
        let per = start.elapsed().as_secs_f64() / frames as f64;
        println!("{:6.3} ms/frame, {:5.1} fps", per * 1000.0, 1.0 / per);
    }
}
