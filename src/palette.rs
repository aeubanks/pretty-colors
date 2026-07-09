pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (rp, gp, bp) = match (h / 60.0).floor() as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Rgb {
        r: ((rp + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        g: ((gp + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        b: ((bp + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    }
}
