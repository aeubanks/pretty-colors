mod noise;
mod palette;

use clap::Parser;
use minifb::{Key, Window, WindowOptions};
use noise::NoiseField;

#[derive(Parser)]
#[command(name = "pretty-colors", about = "Animated 3D Perlin noise visualization")]
struct Args {
    #[arg(long, default_value_t = 800)]
    width: u32,

    #[arg(long, default_value_t = 600)]
    height: u32,

    #[arg(long, default_value_t = 0.008)]
    scale: f64,

    #[arg(long, default_value_t = 1.0)]
    speed: f64,

    #[arg(long)]
    seed: Option<u32>,

    #[arg(long, default_value_t = 30)]
    fps: u32,
}

fn main() {
    let args = Args::parse();

    let seed = args.seed.unwrap_or_else(|| rand::random::<u32>() % 1_000_000);
    let noise = NoiseField::new(seed, args.scale, args.speed);

    eprintln!(
        "Window: {}×{}, scale: {}, speed: {}, seed: {}, fps: {}",
        args.width, args.height, args.scale, args.speed, seed, args.fps
    );

    let mut window = Window::new(
        "pretty-colors",
        args.width as usize,
        args.height as usize,
        WindowOptions::default(),
    )
    .unwrap();

    window.set_target_fps(args.fps as usize);

    let mut buffer: Vec<u32> = vec![0; (args.width * args.height) as usize];
    let mut t = 0.0_f64;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        noise.fill(&mut buffer, args.width, args.height, t);
        window
            .update_with_buffer(&buffer, args.width as usize, args.height as usize)
            .unwrap();
        t += 0.01;
    }
}
