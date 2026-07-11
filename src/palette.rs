pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let a = s * l.min(1.0 - l);
    let f = |n: f64| {
        let k0 = n + h / 30.0;
        let k = if k0 >= 12.0 { k0 - 12.0 } else { k0 };
        l - a * (k - 3.0).min(9.0 - k).min(1.0).max(-1.0)
    };
    let to_u8 = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;

    Rgb {
        r: to_u8(f(0.0)),
        g: to_u8(f(8.0)),
        b: to_u8(f(4.0)),
    }
}
