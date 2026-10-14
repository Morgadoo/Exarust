//! A level's environment record (chunk `0xEC00DC00`): ambient colours, the
//! level's vertical extent and sun settings. Layout and evidence:
//! `docs/formats/RFC.md` (Environment).

use crate::{Error, Result};

pub const ENVIRONMENT: u32 = 0xEC00_DC00;
const RECORD_LEN: usize = 0xF8;
const CENTIMETRES_PER_METRE: f32 = 100.0;

/// How a level gets sunlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunMode {
    /// A sun only when the level has no lights of its own.
    Auto,
    Off,
    On,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    pub flags: u32,
    pub sun_mode: SunMode,
    /// 0 points straight down, 0.5 at the horizon, 1 at the zenith.
    pub sun_altitude: f32,
    /// 0 towards +Z, growing towards −X; ±1 towards −Z.
    pub sun_azimuth: f32,
    /// sRGB.
    pub sun_colour: [f32; 3],
    /// Secondary ambient, sRGB RGBA.
    pub fill: [f32; 4],
    /// Main ambient colour, sRGB.
    pub ambient: [f32; 3],
    /// Vertical extent of the level, metres.
    pub floor: f32,
    pub ceiling: f32,
    /// Added to the base reverb decay time, seconds.
    pub reverb_decay: f32,
}

impl Default for Environment {
    /// The values a level without the record gets.
    fn default() -> Self {
        Self {
            flags: 0,
            sun_mode: SunMode::Auto,
            sun_altitude: 0.7516,
            sun_azimuth: -0.21,
            sun_colour: [0.75; 3],
            fill: [0.04, 0.04, 0.04, 1.0],
            ambient: [0.314; 3],
            floor: 0.0,
            ceiling: 3.4,
            reverb_decay: 0.0,
        }
    }
}

impl Environment {
    pub fn parse(b: &[u8]) -> Result<Self> {
        if b.len() < RECORD_LEN {
            return Err(Error::Truncated {
                offset: 0,
                needed: RECORD_LEN,
                len: b.len(),
            });
        }
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        let f32_at = |o: usize| f32::from_bits(u32_at(o));
        let i32_at = |o: usize| u32_at(o) as i32;
        let floor = i32_at(0x94) as f32 / CENTIMETRES_PER_METRE;
        let ceiling = match i32_at(0x98) {
            0 => i32_at(0x90),
            c => c,
        } as f32
            / CENTIMETRES_PER_METRE;
        Ok(Self {
            flags: u32_at(0),
            sun_mode: match u32_at(0x04) {
                1 => SunMode::Off,
                2 => SunMode::On,
                _ => SunMode::Auto,
            },
            sun_altitude: f32_at(0x08),
            sun_azimuth: f32_at(0x0C),
            sun_colour: [f32_at(0x10), f32_at(0x14), f32_at(0x18)],
            fill: [f32_at(0x30), f32_at(0x34), f32_at(0x38), f32_at(0x3C)],
            ambient: [f32_at(0x40), f32_at(0x44), f32_at(0x48)],
            floor,
            ceiling,
            reverb_decay: f32_at(0xD0),
        })
    }

    /// Unit vector pointing towards the sun (Y up).
    pub fn sun_direction(&self) -> [f32; 3] {
        let theta = (1.0 - self.sun_altitude) * std::f32::consts::PI;
        let phi = -self.sun_azimuth * std::f32::consts::PI;
        [
            theta.sin() * phi.sin(),
            theta.cos(),
            theta.sin() * phi.cos(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> Vec<u8> {
        let mut b = vec![0u8; RECORD_LEN];
        let mut put = |o: usize, v: [u8; 4]| b[o..o + 4].copy_from_slice(&v);
        put(0, 0x0001_0020u32.to_le_bytes());
        put(4, 2u32.to_le_bytes());
        put(8, 0.75f32.to_le_bytes());
        put(0x40, 0.5f32.to_le_bytes());
        put(0x44, 0.4f32.to_le_bytes());
        put(0x48, 0.3f32.to_le_bytes());
        put(0x90, 400i32.to_le_bytes());
        put(0x94, (-500i32).to_le_bytes());
        put(0x98, 0i32.to_le_bytes());
        b
    }

    #[test]
    fn reads_ambient_extent_and_sun() {
        let e = Environment::parse(&record()).unwrap();
        assert_eq!(e.sun_mode, SunMode::On);
        assert_eq!(e.ambient, [0.5, 0.4, 0.3]);
        assert_eq!(e.floor, -5.0);
        // A zero ceiling falls back to the grid top.
        assert_eq!(e.ceiling, 4.0);
        assert!(Environment::parse(&[0; 10]).is_err());
    }

    #[test]
    fn sun_direction_from_altitude_and_azimuth() {
        let mut e = Environment {
            sun_altitude: 1.0,
            sun_azimuth: 0.0,
            ..Default::default()
        };
        let d = e.sun_direction();
        assert!((d[1] - 1.0).abs() < 1e-6);
        e.sun_altitude = 0.5;
        let d = e.sun_direction();
        assert!(d[1].abs() < 1e-6 && (d[2] - 1.0).abs() < 1e-6);
        e.sun_azimuth = 0.5;
        let d = e.sun_direction();
        assert!((d[0] + 1.0).abs() < 1e-6);
    }
}
