//! The dye palette: 32 colour families of eight shades each, and the two
//! HSV ramps the original builds them with.
//!
//! The family table holds each family's hue, saturation, base value and value
//! step. The linear ramp ([`linear`]) is the one the apparel painter uses; the
//! lifted ramp ([`lifted`]) is the one the factory's material rolls use: it
//! raises the value by 0.1 before squaring it. Both end in the same HSV to RGB
//! conversion.

/// A palette family: hue, saturation, base value and value step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Family {
    pub hue: f32,
    pub saturation: f32,
    pub base: f32,
    pub step: f32,
}

/// The 32 families, by family number (`index >> 3`).
pub const FAMILIES: [Family; 32] = [
    Family {
        hue: 0.0,
        saturation: 0.0,
        base: 0.0,
        step: 1.0,
    },
    Family {
        hue: 0.03137255,
        saturation: 0.27450982,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.98039216,
        saturation: 0.8235294,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.99607843,
        saturation: 0.6784314,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.0,
        saturation: 1.0,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.03529412,
        saturation: 0.6784314,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.058431372,
        saturation: 0.47254902,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.07058824,
        saturation: 0.6313726,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.082431376,
        saturation: 1.0,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.09803922,
        saturation: 0.6666667,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.105882354,
        saturation: 0.8156863,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.11372549,
        saturation: 0.3882353,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.1290196,
        saturation: 1.0,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.13725491,
        saturation: 0.5058824,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.14901961,
        saturation: 1.0,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.17254902,
        saturation: 0.74509805,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.22745098,
        saturation: 0.45882353,
        base: 0.2,
        step: 0.7,
    },
    Family {
        hue: 0.27137256,
        saturation: 0.56764704,
        base: 0.2,
        step: 0.6,
    },
    Family {
        hue: 0.3764706,
        saturation: 0.62352943,
        base: 0.2,
        step: 0.6,
    },
    Family {
        hue: 0.4117647,
        saturation: 0.8,
        base: 0.2,
        step: 0.5,
    },
    Family {
        hue: 0.47058824,
        saturation: 0.7,
        base: 0.2,
        step: 0.6,
    },
    Family {
        hue: 0.5237255,
        saturation: 0.7,
        base: 0.2,
        step: 0.6,
    },
    Family {
        hue: 0.57254905,
        saturation: 1.0,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.5803922,
        saturation: 0.78431374,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.6,
        saturation: 0.50980395,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.62352943,
        saturation: 0.9,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.6417647,
        saturation: 0.8666667,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.6901961,
        saturation: 0.4745098,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.7411765,
        saturation: 0.8,
        base: 0.2,
        step: 0.6,
    },
    Family {
        hue: 0.8039216,
        saturation: 0.4862745,
        base: 0.2,
        step: 0.6,
    },
    Family {
        hue: 0.85882354,
        saturation: 0.3372549,
        base: 0.2,
        step: 0.8,
    },
    Family {
        hue: 0.9372549,
        saturation: 0.7058824,
        base: 0.2,
        step: 0.6,
    },
];

/// HSV to RGB, hue in turns.
pub fn hsv(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let h6 = hue.rem_euclid(1.0) * 6.0;
    let sector = h6.floor();
    let f = h6 - sector;
    let (p, q, t) = (
        value * (1.0 - saturation),
        value * (1.0 - saturation * f),
        value * (1.0 - saturation * (1.0 - f)),
    );
    match sector as i32 {
        0 => [value, t, p],
        1 => [q, value, p],
        2 => [p, value, t],
        3 => [p, q, value],
        4 => [t, p, value],
        _ => [value, p, q],
    }
}

/// The colour's value and saturation before HSV conversion, on the linear
/// ramp (the apparel painter's).
fn linear_ramp(family: &Family, shade: f32) -> (f32, f32) {
    let k = 7.0 - shade;
    let value = (k * family.step * 0.13 + family.base).min(1.0);
    let mut saturation = (1.0 - (0.09 * k).powi(2)) * family.saturation;
    if value < 0.4 {
        saturation *= (value + 0.6).powi(2);
    }
    (value, saturation)
}

/// The colour's value and saturation before HSV conversion, on the lifted
/// ramp (the factory's material rolls): `(value + 0.1)²`, clamped to 1.
fn lifted_ramp(family: &Family, shade: f32) -> (f32, f32) {
    let k = 7.0 - shade;
    let mut value = k * family.step * 0.13 + family.base + 0.1;
    value *= value;
    let value = value.min(1.0);
    let mut saturation = (1.0 - (0.09 * k) * (0.09 * k)) * family.saturation;
    if value < 0.4 {
        saturation *= (value + 0.6) * (value + 0.6);
    }
    (value, saturation)
}

/// Palette colour `index` (family × 8 + shade, shade 0 lightest) on the
/// linear ramp.
pub fn linear(index: u8) -> [f32; 3] {
    let family = FAMILIES[(index >> 3) as usize];
    let (value, saturation) = linear_ramp(&family, (index & 7) as f32);
    hsv(family.hue, saturation, value)
}

/// Palette colour `index` on the lifted ramp (factory area materials).
pub fn lifted(index: u8) -> [f32; 3] {
    let family = FAMILIES[(index >> 3) as usize];
    let (value, saturation) = lifted_ramp(&family, (index & 7) as f32);
    hsv(family.hue, saturation, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 2e-3)
    }

    #[test]
    fn linear_matches_known_entries() {
        assert!(near(linear(0), [0.91, 0.91, 0.91]));
        assert!(near(linear(7), [0.0, 0.0, 0.0]));
        assert!(near(linear(8), [0.928, 0.803_3, 0.774_4]));
        assert!(near(linear(15), [0.2, 0.171_5, 0.164_9]));
    }

    #[test]
    fn lifted_squares_the_raised_value() {
        // Family 1 shade 1: lifted (0.624 + 0.3)² = 0.854 against the linear
        // 0.824; the darkest shade falls to 0.09 against 0.2.
        let lifted_bright = lifted(9);
        let linear_bright = linear(9);
        assert!(lifted_bright.iter().sum::<f32>() > linear_bright.iter().sum::<f32>());
        let lifted_dark = lifted(15);
        let linear_dark = linear(15);
        assert!(lifted_dark.iter().sum::<f32>() < linear_dark.iter().sum::<f32>());
        assert!(lifted_dark.iter().all(|v| (0.0..=1.0).contains(v)));
    }
}
