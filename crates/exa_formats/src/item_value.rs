//! What a made item is weighed by: a weapon's and a shield's desirability
//! and apparel's protection value, the figures the AI's search, the doll
//! and the character view compare items by. One implementation for the
//! item crate and the AI. The float and double steps follow the
//! assembly: the protection value's bone factor is `w·(√(c·0.01)·0.005)` in
//! double, narrowed once. Evidence: the item-instance notes
//! (`context/re/item-gui-lane.md`, Item copies).

/// The protection weight of each of bones 1..20.
pub const BONE_WEIGHTS: [f32; 20] = [
    1.0, 1.0, 1.0, 0.5, 1.0, 1.0, 0.5, 2.0, 2.5, 4.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 1.0,
    1.0,
];
/// Desirability of a weapon with no skill points.
pub const UNSKILLED_WEAPON: f32 = -10.0;
/// What the AI and the doll weigh a weapon by: its thrust over 140 plus its
/// skill points times `(0.5q² + 0.5)(1 − 0.3d)`; a one-handed weapon keeps
/// 0.8 of that. Without skill points it is [`UNSKILLED_WEAPON`].
pub fn weapon_desirability(flags: u32, quality: u8, damage: u8, thrust: u8, skill: u8) -> f32 {
    let points = u32::from(skill & 0x1F);
    if points == 0 {
        return UNSKILLED_WEAPON;
    }
    let q = f32::from(quality) / 255.0;
    let d = f32::from(damage) / 255.0;
    let v = f32::from(thrust) / 140.0 + points as f32 * (q * q * 0.5 + 0.5) * (1.0 - d * 0.3);
    if flags & 0x30 == 0x10 { v * 0.8 } else { v }
}

/// What a shield is weighed by: its sixth body byte × 0.05.
pub fn shield_desirability(byte: u8) -> f64 {
    f64::from(byte) * 0.05
}

/// Apparel's protection value, the AI's and the character sheet's figure:
/// over the twenty bones, each nonzero coverage word `c` pairs with the
/// bone's weight `w` (same index) and gives `w·(√(c·0.01)·0.005)` in double
/// precision, rounded to single; the single-precision sum of the four
/// resistance bytes multiplies it and the total adds it, bone by bone.
pub fn protection_value(coverage: &[u32; 20], resist: [u8; 4]) -> f32 {
    let sum = resist.iter().map(|&r| u64::from(r)).sum::<u64>() as f32;
    let mut total = 0.0f32;
    for (bone, &c) in coverage.iter().enumerate() {
        if c != 0 {
            let w = f64::from(BONE_WEIGHTS[bone]) * ((f64::from(c) * 0.01).sqrt() * 0.005);
            total += sum * w as f32;
        }
    }
    total
}

/// The weighted protection and the largest single bone's, without the
/// square root: each bone gives `coverage·0.01·0.005` times the four bytes.
pub fn protection_parts(coverage: &[u32; 20], resist: [u8; 4]) -> (f32, f32) {
    let sum = resist.iter().map(|&r| u64::from(r)).sum::<u64>() as f32;
    let (mut weighted, mut most) = (0.0f32, 0.0f32);
    for (bone, &c) in coverage.iter().enumerate() {
        if c != 0 {
            let part = (f64::from(c) * 0.01 * 0.005) as f32 * sum;
            weighted += part * BONE_WEIGHTS[bone];
            if most < part {
                most = part;
            }
        }
    }
    (weighted, most)
}
