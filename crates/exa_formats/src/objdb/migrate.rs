//! Template migration: refreshing an instance's sections from the template
//! its header names, as the original does after loading a level or a save.
//! Layout and evidence: `docs/formats/OTHER.md` (Object database,
//! Instancing, promotion and migration).

use super::packed::{amount_compact, amount_expand, round_f32, round_f64};
use super::record::{copy_bytes, section_range, set_u16, set_u32, u16_at, u32_at};
use super::{
    BODY, CONTAINER, DOOR, ID_MASK, INHERIT, LOCAL, MODEL, NAME, ObjectDb, TAG_MASK, Tier, WORLD,
};

/// The materials of each procedural material set (`Layer::random_set`),
/// from the original's set table: a layer that names a set keeps its
/// material only when the set lists it. Sets 0 and above 15 list none.
pub fn material_set(set: u8) -> &'static [u8] {
    match set {
        1 | 5 => &[1, 2, 3, 4, 7, 8, 9],
        2 => &[5, 6, 10],
        3 | 6 => &[1, 2, 3, 4],
        4 => &[7, 8, 9],
        7 => &[13, 20],
        8 => &[1, 2, 3, 4, 17, 13, 20],
        9 => &[5, 6],
        10 => &[7],
        11 => &[11],
        12 => &[13, 14],
        13 => &[13, 14, 15],
        14 => &[20],
        15 => &[23],
        _ => &[],
    }
}

/// The byte a body without appearance gives each layer's `+7` byte, by its
/// material: `0x88` for materials 1 to 25, 0 for 0 and 26 to 51 (the
/// original's table; higher materials index past it into unrelated data,
/// and give 0 here).
fn plain_layer_byte(material: u8) -> u8 {
    if (1..=25).contains(&material) {
        0x88
    } else {
        0
    }
}

/// The item factory's pass, as the original's migration runs it: a weapon
/// whose model names a factory (`@…`) is rebuilt from that factory and its
/// own seeds and stored choices, the seeds, flags, choices and stat bytes
/// written into the body in place. It is called with the name after the
/// `@` (at most fifteen bytes) and the weapon section body, and does
/// nothing when no factory has that name. The condition scaling that
/// follows is the migration's own.
pub type FactoryPass<'a> = &'a mut (dyn FnMut(&str, &mut [u8]) + 'a);

impl ObjectDb {
    /// Migrates one local or world object (tag exactly bit 31 or bit 30,
    /// non-zero id) from its template. Returns whether the record was
    /// found and visited.
    pub fn migrate(&mut self, id: u32) -> bool {
        self.migrate_with(id, None)
    }

    /// [`Self::migrate`] with the item factory's stat pass to hand: a
    /// migrated factory weapon (its model named `@…`) has its stats rebuilt
    /// from the factory, as the original does inside the migration. Either
    /// way the factory weapon's stat bytes are then scaled by its own
    /// quality and damage (capped at 160) and its model's value word
    /// rescaled, as the original does even when no factory has the name;
    /// without a pass that is all that changes.
    pub fn migrate_with(&mut self, id: u32, factory: Option<FactoryPass<'_>>) -> bool {
        if id & ID_MASK == 0 {
            return false;
        }
        let tier = match id & TAG_MASK {
            WORLD => Tier::World,
            LOCAL => Tier::Local,
            _ => return false,
        };
        let Some(at) = self.store(tier).find_index(id & ID_MASK) else {
            return false;
        };
        self.migrate_at(tier, at, factory)
    }

    /// Migrates every record of a tier that has a view, as the original's
    /// save loader does for a database. Returns how many were visited.
    pub fn migrate_tier(&mut self, tier: Tier) -> usize {
        self.migrate_tier_with(tier, None)
    }

    /// [`Self::migrate_tier`] with the item factory's stat pass to hand.
    pub fn migrate_tier_with(&mut self, tier: Tier, mut factory: Option<FactoryPass<'_>>) -> usize {
        let len = self.store(tier).len();
        let mut visited = 0;
        for at in 0..len {
            let visited_at = match factory.as_mut() {
                Some(pass) => self.migrate_at(tier, at, Some(&mut **pass)),
                None => self.migrate_at(tier, at, None),
            };
            if visited_at {
                visited += 1;
            }
        }
        visited
    }

    /// The original's migration of one record. Nothing happens unless the
    /// header names a template that resolves. The material class (header
    /// word bits 16–23) of the template (read at its payload `0x18`) and
    /// the instance are compared; then each section of the instance with a
    /// tag of `0x1000` or more that the template also has is refreshed by
    /// its kind.
    fn migrate_at(&mut self, tier: Tier, at: usize, mut factory: Option<FactoryPass<'_>>) -> bool {
        let Some(src) = self.store(tier).view(at) else {
            return false;
        };
        let mut payload = src.payload().to_vec();
        let Some(header) = section_range(&payload, NAME) else {
            return true;
        };
        let template_id = u32_at(&payload[header.clone()], 0x14);
        if template_id == 0 {
            return true;
        }
        let Some(template) = self.find(template_id) else {
            return true;
        };
        let template = template.payload().to_vec();
        let class_changed =
            u32_at(&template, 0x18) & 0xFF_0000 != u32_at(&payload[header], 0x10) & 0xFF_0000;
        let mut model: Option<std::ops::Range<usize>> = None;
        let mut pos = 0;
        while let Some(head) = payload.get(pos..pos + 8) {
            let tag = u32_at(head, 0);
            let size = u32_at(head, 4) as usize;
            if size < 8 || pos + size > payload.len() {
                break;
            }
            let body = pos + 8..pos + size;
            pos += size;
            if tag < 0x1000 {
                continue;
            }
            let Some(t) = section_range(&template, tag) else {
                continue;
            };
            let t = &template[t];
            match tag {
                BODY => {
                    let b = &mut payload[body.clone()];
                    if class_changed {
                        migrate_appearance(b, t);
                    }
                    if u32_at(t, 0x1AC) == 0xFFFF_FFFF {
                        set_u32(b, 0x1AC, 0xFFFF_FFFF);
                        copy_bytes(t, b, 0x1B0, 2);
                        for i in 0..8 {
                            if let Some(&material) = b.get(0x1B8 + i * 12) {
                                b[0x1BB + i * 12] = plain_layer_byte(material);
                            }
                        }
                        for i in 0..4 {
                            let at = 0x218 + i * 2;
                            if let (Some(&tb), Some(byte)) = (t.get(at), b.get_mut(at)) {
                                *byte = if tb & 0x80 != 0 { *byte | 0x80 } else { 0 };
                            }
                        }
                    } else if u32_at(b, 0x1AC) == 0xFFFF_FFFF {
                        copy_bytes(t, b, 0, 0x224);
                    }
                    for (at, len) in [
                        (0x00, 0x10),
                        (0x10, 0xA0),
                        (0xB0, 0xA0),
                        (0x150, 0x50),
                        (0x1A6, 3),
                        (0x220, 4),
                    ] {
                        copy_bytes(t, b, at, len);
                    }
                    refresh_condition(&mut payload, body, t, model.clone());
                }
                super::WEAPON => {
                    // The name the record shows, read off the model section
                    // the walk visited before this one. The original reads
                    // through that pointer unconditionally (a record whose
                    // weapon section comes first would fault); here such a
                    // record is simply not a factory weapon.
                    let name = model.clone().and_then(|m| payload.get(m)).map(model_name);
                    let b = &mut payload[body];
                    if u32_at(t, 4) == 0xFFFF_FFFF {
                        set_u32(b, 4, 0xFFFF_FFFF);
                        set_u32(b, 8, 0xFFFF_FFFF);
                        copy_bytes(t, b, 0xC, 4);
                    }
                    set_u32(
                        b,
                        0,
                        u32_at(b, 0) & 0xF000_0000 | u32_at(t, 0) & 0x0FFF_FFFF,
                    );
                    if weapon_follows_template(b) {
                        for (at, len) in [(0x30, 0x10), (0x40, 1), (0x42, 1)] {
                            copy_bytes(t, b, at, len);
                        }
                        // A factory weapon is then rebuilt from its factory
                        // (when the caller has one by that name) and, in
                        // every case, scaled by its own condition.
                        if let Some(factory_name) =
                            name.as_deref().and_then(|n| n.strip_prefix('@'))
                        {
                            if let Some(pass) = factory.as_mut() {
                                pass(factory_name, b);
                            }
                            let (quality, damage) = (b[0xC], b[0xD]);
                            if let Some(stats) = crate::weapon_stats::StatBytes::from_section(b) {
                                let scaled = crate::weapon_stats::condition(stats, quality, damage);
                                b[0x30..0x30 + scaled.0.len()].copy_from_slice(&scaled.0);
                            }
                            if let Some(m) = model.clone() {
                                rescale_value(&mut payload[m], quality, quality, damage);
                            }
                        }
                    }
                }
                CONTAINER => {
                    let b = &mut payload[body];
                    set_u32(b, 0, u32_at(t, 0));
                    set_u16(b, 8, u16_at(t, 8));
                    set_u16(b, 0xA, u16_at(t, 0xA));
                }
                DOOR => {
                    let b = &mut payload[body];
                    set_u32(b, 0, u32_at(b, 0) | u32_at(t, 0) & 0x200);
                }
                MODEL => {
                    let b = &mut payload[body.clone()];
                    if class_changed {
                        copy_bytes(t, b, 0, 0x28);
                        set_u32(b, 0x14, INHERIT);
                        set_u32(b, 0x18, INHERIT);
                    } else {
                        for (at, len) in [(0, 0x10), (0x10, 4), (0x1C, 4)] {
                            copy_bytes(t, b, at, len);
                        }
                    }
                    model = Some(body);
                }
                _ => {}
            }
        }
        self.store_mut(tier).store(at, &payload);
        true
    }
}

impl ObjectDb {
    /// Copies a weapon's stat bytes from its template, as the original's
    /// helper does for a weapon section being built and as the dormant
    /// object editor uses it to start the item it edits: `record`'s header
    /// names the template (none for 0); when that resolves and has a weapon
    /// section, its bytes `0x30`–`0x40` and `0x42` go into `body` (the
    /// hit-sound byte `0x41` and the variant word's high byte are not
    /// copied). The original writes unconditionally; a `body` too short for
    /// a block leaves those bytes as they were. Returns whether a template
    /// weapon section was there to copy.
    pub fn take_template_weapon_stats(&self, record: &super::Record<'_>, body: &mut [u8]) -> bool {
        let template = record.template();
        if template == 0 {
            return false;
        }
        let Some(t) = self.section(template, super::WEAPON) else {
            return false;
        };
        for (at, len) in [(0x30, 0x10), (0x40, 1), (0x42, 1)] {
            copy_bytes(t, body, at, len);
        }
        true
    }
}

/// A model section body's name: its first sixteen bytes up to the first
/// zero, as the original's name buffers hold it.
fn model_name(body: &[u8]) -> String {
    let end = body
        .iter()
        .take(16)
        .position(|&c| c == 0)
        .unwrap_or(body.len().min(16));
    String::from_utf8_lossy(&body[..end]).into_owned()
}

/// The original's weapon test: the words at `+4` and `+8` (the factory
/// seeds) are non-zero and byte `+0xF` is zero. A weapon that passes takes
/// the template's stat bytes during migration.
fn weapon_follows_template(body: &[u8]) -> bool {
    u32_at(body, 4) != 0 && u32_at(body, 8) != 0 && body.get(0xF) == Some(&0)
}

/// Refreshes an instance's appearance layers from its template's, as the
/// original does when the material class changed: mask byte and set; when
/// the template's mask byte is not zero, the set and, if the instance's
/// material is not in that set, the set's first material; byte 3; the
/// template's material when it is not zero; byte `0xB`, and with it the
/// colour when the template's is 5, unless the instance's byte `0xB` is
/// already 5. An instance material under a set that lists none is kept
/// (the original substitutes an uninitialised byte there).
fn migrate_appearance(body: &mut [u8], template: &[u8]) {
    for i in 0..8 {
        let at = 0x1B4 + i * 12;
        let (Some(t), Some(l)) = (template.get(at..at + 12), body.get_mut(at..at + 12)) else {
            break;
        };
        l[0] = t[0];
        l[1] = t[1];
        if t[1] != 0 {
            l[2] = t[2];
            let set = material_set(l[2]);
            if !set.contains(&l[4])
                && let Some(&first) = set.first()
            {
                l[4] = first;
            }
        }
        l[3] = t[3];
        if t[4] != 0 {
            l[4] = t[4];
        }
        if l[0xB] != 5 {
            l[0xB] = t[0xB];
            if t[0xB] == 5 {
                l[8] = t[8];
            }
        }
    }
}

/// The armour variants 1–8 an apparel copy can be rolled with (body byte
/// `0x1AA`), as the original's per-variant handlers apply them after the
/// condition refresh: each scales the padding, slash, crush and pierce
/// bytes and the encumbrance (the bytes wrap; no cap). Variants 7 and 8
/// change nothing.
pub fn variant_scales(variant: u8) -> Option<([f64; 4], f64)> {
    match variant {
        1 => Some(([0.9; 4], 0.8)),
        2 => Some(([1.1; 4], 1.16)),
        3 => Some(([1.2; 4], 1.1)),
        4 => Some(([1.0, 1.35, 1.0, 1.35], 1.06)),
        5 => Some(([1.4, 1.0, 1.2, 1.0], 1.06)),
        6 => Some(([1.3, 1.1, 1.3, 1.1], 1.16)),
        7 | 8 => Some(([1.0; 4], 1.0)),
        _ => None,
    }
}

/// Recomputes an instance body's protection bytes from its template's by
/// its condition, and scales the model's value, as the original's
/// condition refresh does. With quality q and damage d the body's bytes at
/// `0x1B0`/`0x1B1` and the condition factor k = (1 − d·0.4/255)·(q/510 +
/// 0.8) ([`crate::weapon_stats::condition_factor`]):
///
/// - `0x1A0`–`0x1A3` = round(template · k), capped at 160;
/// - `0x1A4` = round((0.3·k + 0.7) · template);
/// - `0x1A5` = round(template / ((1 − d·(0.15/255)) · (q·(0.3/255) + 0.85)));
/// - the armour variant at `0x1AA` then scales them ([`variant_scales`]);
/// - the model's value word `0x1E` (compact amount) becomes
///   value / (2·tq² + 0.5) · (2·q'² + 0.5) · (2·(1 − d')² + 0.5) · 0.5,
///   with tq the template's quality and q', d' the instance's, all /255.
///
/// The precision of each step follows the original (single or double).
fn refresh_condition(
    payload: &mut [u8],
    body: std::ops::Range<usize>,
    template: &[u8],
    model: Option<std::ops::Range<usize>>,
) {
    let b = &mut payload[body];
    if b.len() < 0x1B2 || template.len() < 0x1B1 {
        return;
    }
    let q = b[0x1B0];
    let d = b[0x1B1];
    let k = crate::weapon_stats::condition_factor(q, d);
    b[0x1A4] = round_f64((f64::from(k) * 0.3 + 0.7) * f64::from(template[0x1A4])) as u8;
    for at in 0x1A0..0x1A4 {
        b[at] = round_f32(f32::from(template[at]) * k) as u8;
    }
    for byte in &mut b[0x1A0..0x1A4] {
        *byte = (*byte).min(0xA0);
    }
    let quality_step = f64::from_bits(0x3F53_4679_ACE0_1346); // 0.3 / 255
    let damage_step = f64::from_bits(0x3F43_4679_ACE0_1346); // 0.15 / 255
    let k2 = ((f64::from(q) * quality_step + 0.85) * (1.0 - f64::from(d) * damage_step)) as f32;
    let inverse = 1.0f32 / k2;
    b[0x1A5] = round_f32(f32::from(template[0x1A5]) * inverse) as u8;
    if let Some((scales, encumbrance)) = variant_scales(b[0x1AA]) {
        b[0x1A5] = round_f64(f64::from(b[0x1A5]) * encumbrance) as u8;
        for (at, scale) in (0x1A0..0x1A4).zip(scales) {
            if scale != 1.0 {
                b[at] = round_f64(f64::from(b[at]) * scale) as u8;
            }
        }
    }
    if let Some(model) = model {
        rescale_value(&mut payload[model], template[0x1B0], q, d);
    }
}

/// Scales a model section's value word `0x1E` (compact amount) by
/// condition, as both of the original's condition refreshes do: value /
/// (2·tq² + 0.5) · (2·q² + 0.5) · (2·(1 − d)² + 0.5) · 0.5, with `tq` the
/// base quality, `q` and `d` the item's own, all /255 in double and the
/// curve in float. The weapon refresh passes its own quality as the base.
fn rescale_value(model: &mut [u8], base_quality: u8, quality: u8, damage: u8) {
    if model.len() < 0x20 {
        return;
    }
    let curve = |x: f32| x * x * 2.0 + 0.5;
    let base = curve((f64::from(base_quality) / 255.0) as f32);
    let made = curve((f64::from(quality) / 255.0) as f32);
    let wear = curve((1.0 - f64::from(damage) / 255.0) as f32);
    // The weapon refresh multiplies made · (wear · 0.5), the body refresh
    // (made · wear) · 0.5; halving is exact, so the two agree.
    let scale = made * (wear * 0.5);
    let value = amount_expand(u16_at(model, 0x1E));
    let scaled = round_f32(value as f32 / base * scale);
    set_u16(model, 0x1E, amount_compact(scaled as u32));
}

#[cfg(test)]
mod tests {
    use super::super::record::test_util::*;
    use super::*;
    use crate::objdb::{Record, Store, WEAPON};

    fn store(records: &[(u32, u32, Vec<u8>)]) -> Store {
        Store::from_records(records.iter().map(|(i, f, p)| (*i, *f, &p[..])))
    }

    fn body() -> Vec<u8> {
        vec![0u8; 0x224]
    }

    #[test]
    fn material_sets() {
        assert_eq!(material_set(2), &[5, 6, 10]);
        assert_eq!(material_set(8), &[1, 2, 3, 4, 17, 13, 20]);
        assert_eq!(material_set(15), &[23]);
        assert!(material_set(0).is_empty() && material_set(16).is_empty());
        assert_eq!(plain_layer_byte(25), 0x88);
        assert_eq!(plain_layer_byte(26), 0);
    }

    #[test]
    fn appearance_follows_a_class_change() {
        let mut t = body();
        let mut b = body();
        // Layer 0: the template's mask 2, set 2 (materials 5, 6, 10), no
        // material of its own; the instance has material 7, not in the
        // set, so it becomes 5.
        t[0x1B4..0x1B4 + 12].copy_from_slice(&[9, 2, 2, 3, 0, 0, 0, 0, 0x40, 0, 0, 1]);
        b[0x1B4..0x1B4 + 12].copy_from_slice(&[0, 0, 0, 0, 7, 0, 0, 0, 0x11, 0, 0, 0]);
        // Layer 1: the template's material 13 wins; its byte 0xB is 5, so
        // the colour comes too.
        t[0x1C0..0x1C0 + 12].copy_from_slice(&[0, 1, 7, 0, 13, 0, 0, 0, 0x22, 0, 0, 5]);
        b[0x1C0..0x1C0 + 12].copy_from_slice(&[0, 1, 7, 0, 20, 0, 0, 0, 0x33, 0, 0, 0]);
        // Layer 2: the instance's byte 0xB is 5 already: kept, colour too.
        t[0x1CC..0x1CC + 12].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0x44, 0, 0, 1]);
        b[0x1CC..0x1CC + 12].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0x55, 0, 0, 5]);
        migrate_appearance(&mut b, &t);
        assert_eq!(&b[0x1B4..0x1C0], &[9, 2, 2, 3, 5, 0, 0, 0, 0x11, 0, 0, 1]);
        assert_eq!(&b[0x1C0..0x1CC], &[0, 1, 7, 0, 13, 0, 0, 0, 0x22, 0, 0, 5]);
        assert_eq!(&b[0x1CC..0x1D8], &[0, 0, 0, 0, 0, 0, 0, 0, 0x55, 0, 0, 5]);
    }

    #[test]
    fn condition_scales_protection_and_value() {
        let mut t = body();
        t[0x1A0..0x1A6].copy_from_slice(&[100, 200, 50, 0, 80, 10]);
        t[0x1B0] = 255;
        let mut b = body();
        b[0x1B0] = 255; // best quality
        b[0x1B1] = 0; // undamaged
        let mut m = vec![0u8; 0x28];
        m[0x1E..0x20].copy_from_slice(&1000u16.to_le_bytes());
        let mut payload = b.clone();
        payload.extend(&m);
        refresh_condition(&mut payload, 0..0x224, &t, Some(0x224..0x224 + 0x28));
        // k = 1 · (255/510 + 0.8) = 1.3 (in single precision after the
        // double product): 100 → 130, 200 → 260 → its low byte 4, which
        // the cap leaves alone; 50 → 65.
        let k = crate::weapon_stats::condition_factor(255, 0);
        assert_eq!(payload[0x1A0], (100.0 * k).round() as u8);
        assert_eq!(payload[0x1A0], 130);
        assert_eq!(payload[0x1A1], 4);
        assert_eq!(payload[0x1A2], 65);
        assert_eq!(payload[0x1A3], 0);
        // (0.3·1.3 + 0.7)·80 = 87.2 → 87.
        assert_eq!(payload[0x1A4], 87);
        // 10 / (1 · 1.15) = 8.7 → 9.
        assert_eq!(payload[0x1A5], 9);
        // Value: 1000 / 2.5 · 2.5 · 2.5 · 0.5 = 1250.
        assert_eq!(u16_at(&payload[0x224..], 0x1E), 1250);
        // A damaged, poor copy: q = 0, d = 255.
        let mut payload = body();
        payload[0x1B1] = 255;
        payload.extend(&m);
        refresh_condition(&mut payload, 0..0x224, &t, Some(0x224..0x224 + 0x28));
        // k = (1 − 0.4) · 0.8 = 0.48: 100 → 48, 200 → 96, 50 → 24.
        assert_eq!(&payload[0x1A0..0x1A3], &[48, 96, 24]);
        // Value: 1000 / 2.5 · 0.5 · 0.5 · 0.5 = 50.
        assert_eq!(u16_at(&payload[0x224..], 0x1E), 50);
        // Variant 4 at best condition: slash and pierce ×1.35 after the
        // condition (200 → 260 → 4, then 5.4 → 5), the encumbrance ×1.06
        // (9 → 9.54 → 10).
        let mut payload = body();
        payload[0x1B0] = 255;
        payload[0x1AA] = 4;
        refresh_condition(&mut payload, 0..0x224, &t, None);
        assert_eq!(&payload[0x1A0..0x1A6], &[130, 5, 65, 0, 87, 10]);
    }

    fn template_and_instance(t_header: u32, i_header: u32) -> ObjectDb {
        let mut t_body = body();
        t_body[0..8].copy_from_slice(b"Worn_A\0\0");
        for bone in 0..20 {
            t_body[0x150 + bone * 4] = 50;
        }
        t_body[0x1A6..0x1A9].copy_from_slice(&[1, 2, 3]);
        t_body[0x220..0x224].copy_from_slice(&[7, 7, 7, 7]);
        t_body[0x1B0] = 255;
        let mut t_model = model("Worn", 0x10, 5, 6);
        t_model[0x1C..0x20].copy_from_slice(&[3, 40, 0xE8, 0x03]);
        let mut t_weapon = vec![0u8; 0x60];
        t_weapon[0..4].copy_from_slice(&0x0123_4567u32.to_le_bytes());
        t_weapon[0x30..0x43].copy_from_slice(&[9; 0x13]);
        let mut i_body = body();
        i_body[0x1B0] = 255;
        let mut i_weapon = vec![0u8; 0x60];
        i_weapon[0..4].copy_from_slice(&0xA000_0000u32.to_le_bytes());
        i_weapon[4] = 1;
        i_weapon[8] = 2;
        let mut db = ObjectDb::default();
        db.base = store(&[(
            10,
            NAME | MODEL | BODY | WEAPON,
            payload(&[
                (NAME, header("Template", t_header, 0)),
                (MODEL, t_model),
                (BODY, t_body),
                (WEAPON, t_weapon),
            ]),
        )]);
        db.local = store(&[(
            1,
            NAME | MODEL | BODY | WEAPON,
            payload(&[
                (NAME, header("Instance", i_header, 10)),
                (MODEL, model("Other", 0, 9, 9)),
                (BODY, i_body),
                (WEAPON, i_weapon),
            ]),
        )]);
        db
    }

    #[test]
    fn migration_refreshes_an_instance() {
        let mut db = template_and_instance(0, 0);
        assert!(db.migrate(LOCAL | 1));
        let r: Record<'_> = db.find(LOCAL | 1).unwrap();
        let m = r.model().unwrap();
        // Same class: name, flags and the words at 0x1C copy; the string
        // ids stay; the value is then rescaled by the condition: with the
        // template's and the copy's quality 255 and no damage, 1000 / 2.5
        // · 2.5 · 2.5 · 0.5 = 1250.
        assert_eq!((m.name.as_str(), m.flags), ("Worn", 0x10));
        assert_eq!((m.name_id, m.description_id), (9, 9));
        assert_eq!((m.bulk, m.rarity, m.value), (3, 40, 1250));
        let b = r.section(BODY).unwrap();
        assert_eq!(&b[0..6], b"Worn_A");
        assert_eq!(b[0x150], 50);
        assert_eq!(&b[0x1A6..0x1A9], &[1, 2, 3]);
        assert_eq!(&b[0x220..0x224], &[7, 7, 7, 7]);
        // The weapon has seeds and byte 0xF zero: the stats follow; its
        // top flag nibble stays.
        let w = r.section(WEAPON).unwrap();
        assert_eq!(&w[0x30..0x41], &[9; 0x11]);
        assert_eq!(w[0x41], 0);
        assert_eq!(w[0x42], 9);
        assert_eq!(u32_at(w, 0), 0xA123_4567);
        // A base id, a both-tag id and an id of 0 are not migrated.
        assert!(!db.migrate(10));
        assert!(!db.migrate(LOCAL | WORLD | 1));
        assert!(!db.migrate(LOCAL));
        assert_eq!(db.migrate_tier(Tier::Local), 1);
    }

    #[test]
    fn migration_after_a_class_change() {
        let mut db = template_and_instance(0x12_0000, 0x11_0000);
        assert!(db.migrate(LOCAL | 1));
        let m = db.find(LOCAL | 1).unwrap().model().unwrap();
        assert_eq!((m.name_id, m.description_id), (INHERIT, INHERIT));
        assert_eq!(m.name, "Worn");
    }

    #[test]
    fn a_weapon_built_from_its_template() {
        let db = template_and_instance(0, 0);
        let r = db.find(LOCAL | 1).unwrap();
        let mut body = vec![0u8; 0x60];
        assert!(db.take_template_weapon_stats(&r, &mut body));
        assert_eq!(&body[0x30..0x41], &[9; 0x11]);
        assert_eq!((body[0x41], body[0x42]), (0, 9));
        let t = db.find(10).unwrap();
        assert!(!db.take_template_weapon_stats(&t, &mut body));
        // The object editor starts the item it edits the same way: its own
        // buffer takes the stat block, and the hit-sound byte stays.
        let mut edited = [0xAAu8; 0x60];
        assert!(db.take_template_weapon_stats(&r, &mut edited));
        assert_eq!(&edited[0x30..0x41], &[9; 0x11]);
        assert_eq!((edited[0x41], edited[0x42], edited[0x43]), (0xAA, 9, 0xAA));
        // A buffer too short for a block keeps what it has and does not
        // panic; the original writes unconditionally, so a section was
        // still there to copy.
        let mut short = [0xAAu8; 0x20];
        assert!(db.take_template_weapon_stats(&r, &mut short));
        assert!(short.iter().all(|&b| b == 0xAA));
    }

    #[test]
    fn a_factory_weapon_is_rebuilt_by_the_factory_pass() {
        fn factory_db() -> ObjectDb {
            let mut db = template_and_instance(0, 0);
            // The template's model is a factory weapon (its name starts
            // with `@`), and the migration gives the instance that model
            // before it reaches the weapon section.
            let t = db.find(10).unwrap();
            let mut p = t.payload().to_vec();
            let m = section_range(&p, MODEL).unwrap();
            p[m.start..m.start + 16].fill(0);
            p[m.start..m.start + 11].copy_from_slice(b"@sword_lnga");
            db.base.store(0, &p);
            db
        }
        let weapon = |db: &ObjectDb| {
            db.find(LOCAL | 1)
                .unwrap()
                .section(WEAPON)
                .unwrap()
                .to_vec()
        };
        let value = |db: &ObjectDb| db.find(LOCAL | 1).unwrap().model().unwrap().value;

        // The pass gets the name after the `@` and the weapon body.
        let mut db = factory_db();
        let mut calls: Vec<(String, Vec<u8>)> = Vec::new();
        let mut pass = |name: &str, body: &mut [u8]| {
            calls.push((name.to_string(), body[0x30..0x34].to_vec()));
            body[0x30] = 42;
        };
        assert!(db.migrate_with(LOCAL | 1, Some(&mut pass)));
        assert_eq!(calls, [(String::from("sword_lnga"), vec![9, 9, 9, 9])]);
        let w = weapon(&db);
        // The weight byte the pass wrote is not a scaled stat. The
        // instance's quality and damage are 0, so the condition factor is
        // 0.8: slash and pierce 9·0.8 = 7.2 → 7, crush (0.8·0.7 + 0.3)·9 =
        // 7.74 → 8, thrust 7; impact 9 is only capped.
        assert_eq!(w[0x30], 42);
        assert_eq!((w[0x32], w[0x33]), (9, 7));
        assert_eq!(&w[0x37..0x3A], &[7, 8, 7]);
        assert_eq!(&w[0x3D..0x40], &[7, 8, 7]);
        // The value word is rescaled twice: the body's condition refresh
        // (quality 255 on both sides, no damage) takes 1000 to
        // 1000 / 2.5 · (2.5 · (2.5 · 0.5)) = 1250, then the weapon's, with
        // its own quality 0 as the base, to 1250 / 0.5 · (0.5 · (2.5 · 0.5))
        // = 1562.5, which rounds to even: 1562.
        let factory_value = value(&db);
        assert_eq!(amount_expand(factory_value), 1562);

        // Without a pass (or with no factory of that name) the original
        // still scales the template's stats and the value.
        let mut db = factory_db();
        assert!(db.migrate_with(LOCAL | 1, None));
        let w = weapon(&db);
        assert_eq!((w[0x30], w[0x33]), (9, 7));
        assert_eq!(&w[0x37..0x3A], &[7, 8, 7]);
        assert_eq!(value(&db), factory_value);

        // A model that is not a factory's never reaches the pass, and the
        // migration keeps the template's stats and only the body's value
        // refresh applies.
        let mut db = template_and_instance(0, 0);
        let mut calls = 0;
        let mut pass = |_: &str, _: &mut [u8]| calls += 1;
        assert!(db.migrate_with(LOCAL | 1, Some(&mut pass)));
        assert_eq!(calls, 0);
        let w = weapon(&db);
        assert_eq!((w[0x30], w[0x37]), (9, 9));
        assert_eq!(amount_expand(value(&db)), 1250);
    }

    #[test]
    fn a_weapon_without_seeds_keeps_its_stats() {
        let mut db = template_and_instance(0, 0);
        // Clear the instance's seeds: the stats no longer follow.
        let r = db.find(LOCAL | 1).unwrap();
        let mut p = r.payload().to_vec();
        let w = section_range(&p, WEAPON).unwrap();
        set_u32(&mut p[w], 4, 0);
        db.local.store(0, &p);
        db.migrate(LOCAL | 1);
        let w = db.section(LOCAL | 1, WEAPON).unwrap();
        assert_eq!(&w[0x30..0x43], &[0; 0x13]);
    }

    #[test]
    fn a_template_without_appearance() {
        let mut db = template_and_instance(0, 0);
        let t = db.base.find(10).unwrap();
        let mut p = t.payload().to_vec();
        let b = section_range(&p, BODY).unwrap();
        let tb = &mut p[b];
        set_u32(tb, 0x1AC, 0xFFFF_FFFF);
        tb[0x1B0] = 0x11;
        tb[0x1B1] = 0x22;
        tb[0x218] = 0x80;
        db.base.store(0, &p);
        let r = db.find(LOCAL | 1).unwrap();
        let mut p = r.payload().to_vec();
        let b = section_range(&p, BODY).unwrap();
        let ib = &mut p[b];
        ib[0x1B8] = 12; // layer 0 material: plain byte 0x88
        ib[0x1C4] = 30; // layer 1 material: 0
        ib[0x1C7] = 0x55;
        ib[0x218] = 0x01;
        ib[0x21A] = 0x81;
        db.local.store(0, &p);
        db.migrate(LOCAL | 1);
        let b = db.section(LOCAL | 1, BODY).unwrap();
        assert_eq!(u32_at(b, 0x1AC), 0xFFFF_FFFF);
        assert_eq!(&b[0x1B0..0x1B2], &[0x11, 0x22]);
        assert_eq!((b[0x1BB], b[0x1C7]), (0x88, 0));
        // Power bytes: kept with 0x80 where the template has it, else 0.
        assert_eq!((b[0x218], b[0x21A]), (0x81, 0));
    }
}
