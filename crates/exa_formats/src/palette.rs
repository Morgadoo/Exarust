//! The dye palette: 32 colour families of eight shades each, and the two
//! HSV ramps the original builds them with.
//!
//! The family table holds each family's hue, saturation, base value and value
//! step. The linear ramp ([`linear`]) is the one the apparel painter uses; the
//! lifted ramp ([`lifted`]) is the one the factory's material rolls use: it
//! raises the value by 0.1 before squaring it. Both end in the same HSV to RGB
//! conversion. The precision of each step (float or double) is the
//! original's.

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

/// HSV to RGB, hue in turns: the sector is `⌊6h⌋` (the engine's floor),
/// a grey when the saturation is 0, and any sector outside 1–5 takes the
/// first one's order.
pub fn hsv(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    if saturation == 0.0 {
        return [value; 3];
    }
    let h6 = hue * 6.0;
    let sector = h6.floor();
    let f = h6 - sector;
    let p = (1.0 - saturation) * value;
    let q = (1.0 - saturation * f) * value;
    let t = (1.0 - (1.0 - f) * saturation) * value;
    match sector as i32 {
        1 => [q, value, p],
        2 => [p, value, t],
        3 => [p, q, value],
        4 => [t, p, value],
        5 => [value, p, q],
        _ => [value, t, p],
    }
}

/// RGB to hue (in turns), saturation and value, as the engine's colour
/// helper converts them: the value is the largest channel (green wins over
/// red only when strictly larger, blue over that only when strictly
/// larger), a value under 0.5/255 is black (hue and saturation 0), a
/// saturation `1 − min/max` under 0.5/255 is a grey (hue 0). The hue is a
/// sixth of the two other channels' spread, each taken above the least and
/// over the spread in single precision; the red sextant wraps a negative
/// hue by adding it to 1, and the green and blue sextants add their third
/// in double. A NaN falls through the compares as the instructions do.
pub fn rgb_to_hsv(c: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = c;
    let mut imax = usize::from(g > r);
    if c[imax] < b {
        imax = 2;
    }
    let mut imin = usize::from(g < r);
    if c[imin] > b {
        imin = 2;
    }
    let value = c[imax];
    let floor = f32::from_bits(0x3b00_8081);
    if value < floor {
        return [0.0, 0.0, value];
    }
    let saturation = 1.0 - c[imin] / value;
    if saturation < floor {
        return [0.0, saturation, value];
    }
    let inv = 1.0 / (c[imax] - c[imin]);
    let t = c.map(|x| (x - c[imin]) * inv);
    let hue = match imax {
        0 => {
            let h = (t[1] - t[2]) / 6.0;
            if h < 0.0 { 1.0 + h } else { h }
        }
        1 => (f64::from((t[2] - t[0]) / 6.0) + 1.0 / 3.0) as f32,
        _ => (f64::from((t[0] - t[1]) / 6.0) + 2.0 / 3.0) as f32,
    };
    [hue, saturation, value]
}

/// The saturation both ramps share: `(1 − (0.09k)²)·s`, then, below a value
/// of 0.4, `(v + 0.6)²` times that, each in double precision and stored as
/// a float.
fn ramp_saturation(family: &Family, k: f32, value: f32) -> f32 {
    let d = f64::from(k) * 0.09;
    let mut saturation = ((1.0 - d * d) * f64::from(family.saturation)) as f32;
    if f64::from(value) < 0.4 {
        let lift = f64::from(value) + 0.6;
        saturation = (lift * lift * f64::from(saturation)) as f32;
    }
    saturation
}

/// The step a shade darkens by: the family step times 0.13 in double,
/// stored as a float.
fn ramp_step(family: &Family) -> f32 {
    (f64::from(family.step) * 0.13) as f32
}

/// The colour's value and saturation before HSV conversion, on the linear
/// ramp (the apparel painter's): `k·step + base` in float, at most 1.
fn linear_ramp(family: &Family, shade: f32) -> (f32, f32) {
    let k = 7.0 - shade;
    let value = (k * ramp_step(family) + family.base).min(1.0);
    (value, ramp_saturation(family, k, value))
}

/// The colour's value and saturation before HSV conversion, on the lifted
/// ramp (the factory's area colours): `(k·step + (base + 0.1))²` in double,
/// at most 1.
fn lifted_ramp(family: &Family, shade: f32) -> (f32, f32) {
    let k = 7.0 - shade;
    let raised = f64::from(k * ramp_step(family)) + (f64::from(family.base) + 0.1);
    let value = ((raised * raised) as f32).min(1.0);
    (value, ramp_saturation(family, k, value))
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

/// [`lifted`] as the factory stores an area's colour: each channel times
/// 255, rounded to even and saturated to a byte.
pub fn lifted_bytes(index: u8) -> [u8; 3] {
    lifted(index).map(|c| crate::pixel::round_f32(c * 255.0).clamp(0, 255) as u8)
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

    #[test]
    fn rgb_to_hsv_reads_the_sextants_and_its_thresholds() {
        assert_eq!(rgb_to_hsv([1.0, 0.0, 0.0]), [0.0, 1.0, 1.0]);
        assert_eq!(rgb_to_hsv([0.0, 1.0, 0.0]), [(1.0f64 / 3.0) as f32, 1.0, 1.0]);
        assert_eq!(rgb_to_hsv([0.0, 0.0, 1.0]), [(2.0f64 / 3.0) as f32, 1.0, 1.0]);
        // A red with more blue than green wraps: 1 + (0 − 0.5)/6.
        assert_eq!(rgb_to_hsv([1.0, 0.0, 0.5]), [1.0 - 0.5 / 6.0, 1.0, 1.0]);
        assert_eq!(rgb_to_hsv([0.5, 0.5, 0.5]), [0.0, 0.0, 0.5]);
        // Under half a byte the value is black and the rest zero.
        assert_eq!(rgb_to_hsv([0.001, 0.0, 0.0]), [0.0, 0.0, 0.001]);
        // A tied green does not displace red, so the hue is red's.
        assert_eq!(rgb_to_hsv([1.0, 1.0, 0.0])[0], 1.0 / 6.0);
        // Round trip through the engine's HSV to RGB.
        let back = {
            let [h, s, v] = rgb_to_hsv([0.2, 0.4, 0.6]);
            hsv(h, s, v)
        };
        for (a, b) in back.iter().zip([0.2, 0.4, 0.6]) {
            assert!((a - b).abs() < 1e-6, "{back:?}");
        }
    }

    #[test]
    fn lifted_bytes_round_each_channel() {
        // Family 0 shade 0: v = (7 + 0.1)² clamped to 1, a grey of 255; the
        // darkest shade: v = 0.1² = 0.01, 2.55 rounds to 3.
        assert_eq!(lifted_bytes(0), [255, 255, 255]);
        assert_eq!(lifted_bytes(7), [3, 3, 3]);
    }
}
