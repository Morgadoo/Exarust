//! The teleporter's destination screen: the reachable levels laid on a
//! ring around a hub, its reveal and close fades, the pointer's hover and
//! click, and the journey a click starts (the bubble above the teleporter,
//! the teleporter's glow, the crystal's charge and what the journey does
//! when the bubble reaches its end).
//!
//! Evidence: the destination screen's routines in the retail image, read
//! from the instructions, with the teleporter's own opener, the glow
//! envelope and the bubble it creates. Times are the game clock's
//! milliseconds, positions the screen's pixels (the bubble's offset is
//! the world's centimetres).
//!
//! The ring: node `i > 0` sits at the angle `(1 − turn) + (2.5 − i)·2π /
//! (count − 1)`, at `unit · sin · 40` across and `unit · cos · 40` down
//! (each rounded to an integer, half to even), scaled by the reveal's
//! `turn`; node 0 is the centre. The reveal runs 500 ms: `x = 1 − left ·
//! 0.002` eases to `e = x(2 − x)`, the alpha is `e` and the ring's turn
//! `e(2 − e)`. The close fades the alpha over 256 ms.

use bevy::prelude::Resource;

use crate::engine_math::{round_i64, sin_cos};

/// The word the form's class returns (the small no-op accessor).
pub const FORM_RETURN: i32 = 4;
/// The symbol shader's source, copied from the retail image.
pub const SYMBOL_SHADER: &str = "in vec2 vrTex;\r\nout vec4 FragColor;\r\nuniform vec4 Colour;\r\nuniform vec4 TexPos;\r\nuniform vec2 TexOfs;\r\nuniform sampler2D Tex;\r\nvoid main() {\r\nvec2 a = vrTex - vec2(0.5);\r\nfloat b = min(dot(a,a) * 4.0, 1.0);\r\nb = b * b;b = 1.0 - b * b;b = b * b;float c = dot(a,a);c = abs(c*128-25);c = c * c;c = max(1.0 - c, 0.0);float d = min(dot(a,a) * 4.0, 1.0);\r\nd = clamp((0.6-d) * 16, 0.0, 1.0);a = vrTex * TexPos.xy * 1.3 + TexPos.zw - TexPos.xy * 0.15;a = a + TexOfs * 0.1;d = d * (1.0 - dot(TexOfs, TexOfs) * 30);c = c + texture(Tex, a).a * 0.6 * d;FragColor.rgb = Colour.rgb * c;\r\nFragColor.a = b * Colour.a;\r\n}";
/// The symbol shader's uniform names.
pub const SYMBOL_UNIFORMS: [&str; 3] = ["Colour", "TexPos", "TexOfs"];
/// The symbol atlas the screen draws (a 2 × 2 sheet).
pub const SCREEN_ATLAS: &str = "prtlsmbla.rfi";
/// The reveal takes this long (ms).
pub const REVEAL_MS: u32 = 500;
/// The close tail takes this long (ms).
pub const CLOSE_MS: u32 = 256;
/// The reveal's per-millisecond share (a double).
pub const REVEAL_PER_MS: f64 = 0.002;
/// The leave key waits until the reveal has this long left (ms).
pub const LEAVE_GRACE_MS: i64 = 400;
/// The ring's radius in units.
pub const RING_RADIUS_UNITS: f32 = 40.0;
/// A node is hit within this many units.
pub const HIT_RADIUS_UNITS: i64 = 14;
/// The orbit nodes the hit test tries, whatever the row count.
pub const HIT_NODES: i32 = 3;
/// A symbol is this many units square.
pub const SYMBOL_UNITS: i32 = 32;
/// The share of the screen's alpha a symbol is drawn with (a double).
pub const SYMBOL_ALPHA: f64 = 0.6;
/// The wobble's band: a value at or past it is pushed back (doubles).
pub const WOBBLE_BAND: f64 = 0.9;
/// A node's wobble keeps within this magnitude.
pub const WOBBLE_LIMIT: f32 = 1.0;
/// A hover flash lasts this long (ms).
pub const HOVER_FLASH_MS: u32 = 200;
/// The wobble's per-millisecond strength (a double).
pub const FLASH_PER_MS: f64 = 0.02;
/// A flash fades at this share a millisecond (a double).
pub const FLASH_FADE_PER_MS: f64 = 0.005;
/// The texture offset is the wobble times this (a double).
pub const TEX_OFFSET_SCALE: f64 = 0.1;

/// The screen's state byte: bit 0 while the reveal runs, bit 1 while the
/// close tail runs, and 4 once closed.
pub mod state {
    pub const REVEALING: u8 = 1;
    pub const CLOSING: u8 = 2;
    pub const CLOSED: u8 = 4;
}

/// The screen's form metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenLayout {
    /// The parent's width and height.
    pub width: i32,
    pub height: i32,
    pub flags: u32,
    pub style: u32,
    /// The ring's radius in units.
    pub orbit: f32,
}

/// The form's layout: it fills its parent (the interface root when none is
/// given), with flags 0x1010 under a parent and 0x10 under the root, style
/// 0x800 and the 40-unit ring.
pub fn layout(width: i32, height: i32, has_parent: bool) -> ScreenLayout {
    ScreenLayout {
        width,
        height,
        flags: if has_parent { 0x1010 } else { 0x10 },
        style: 0x800,
        orbit: RING_RADIUS_UNITS,
    }
}

/// A destination marker word: the high bit says it names an object record
/// (the low 30 bits are its index). The journey's end marks that record's
/// operative flags with bit 4, the single-row bit.
pub fn mark_destination(word: u32) -> Option<u32> {
    (word & 0x8000_0000 != 0).then_some(word & 0x3fff_ffff)
}

/// The operative flag a journey's end sets on the marked record.
pub const MARK_FLAG: u16 = 4;

/// One destination row: its flash deadline, its wobble and drift on two
/// axes, the record entry it shows, and the marker word a click passes to
/// the journey.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DestinationSlot {
    /// The time this row's hover flash ends.
    pub flash_until: u32,
    /// The wobble (the symbol's texture offset over ten) on each axis.
    pub wobble: [f32; 2],
    /// The wobble's drift on each axis.
    pub drift: [f32; 2],
    /// The record entry the row shows: the index of its level id, which
    /// also picks the symbol's atlas cell.
    pub entry: usize,
    /// The destination's marker word, handed to the journey (the high bit
    /// names an object record; see [`mark_destination`]).
    pub marker: u32,
}

impl DestinationSlot {
    /// A row showing `entry`, with no flash, no wobble and no marker.
    pub fn new(entry: usize) -> Self {
        Self {
            flash_until: 0,
            wobble: [0.0; 2],
            drift: [0.0; 2],
            entry,
            marker: 0,
        }
    }
}

/// The rows the screen shows, filled in the original's order: entry 0 goes
/// to row 1, later entries to rows 2..., and an empty entry or an entry met
/// once the other rows are taken goes back to row 0 (the last such wins).
/// Every row nothing reaches keeps entry 0.
pub fn fill_rows(destinations: &[u32], rows: usize) -> Vec<usize> {
    if rows == 0 {
        return Vec::new();
    }
    let mut entries = vec![0usize; rows];
    let mut next = 1usize;
    for (i, destination) in destinations.iter().copied().enumerate().take(rows) {
        if destination == 0 || next >= rows {
            entries[0] = i;
        } else {
            entries[next] = i;
            next += 1;
        }
    }
    entries
}

/// The state the screen keeps while it is open.
#[derive(Debug, Clone, PartialEq, Resource)]
pub struct DestinationScreen {
    /// The rows, in screen order.
    pub slots: Vec<DestinationSlot>,
    /// Bit 0 while the reveal runs, bit 1 while the close tail runs, 4
    /// once closed.
    pub state: u8,
    /// The reveal's deadline.
    pub reveal_until: u32,
    /// The close tail's deadline.
    pub close_until: u32,
    /// The hovered row, 1-based; 0 is none.
    pub selected: i32,
    /// The ring's turn, 0..1.
    pub turn: f32,
    /// The screen's alpha, 0..1.
    pub alpha: f32,
    /// The clock the last paint measured the wobble from.
    pub flash_clock: u32,
    /// The record's level ids, by entry.
    pub level_ids: Vec<u32>,
    /// The record's destination words, by entry (the word a click hands to
    /// the journey).
    pub markers: Vec<u32>,
    /// The crystal is the large-crystal adapter: the orbit nodes wobble.
    pub adapter: bool,
}

/// One frame's reveal values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub turn: f32,
    pub alpha: f32,
    /// The close tail ended this frame: the screen is gone and nothing is
    /// drawn.
    pub closed: bool,
}

impl DestinationScreen {
    /// The screen of a teleporter record: `rows` rows over `level_ids`,
    /// opening at `now`.
    pub fn open(level_ids: Vec<u32>, rows: usize, adapter: bool, now: u32) -> Self {
        Self::open_with_markers(level_ids, Vec::new(), rows, adapter, now)
    }

    /// The screen with the record's destination words: row `r` hands the
    /// journey the word of the entry it shows.
    pub fn open_with_markers(
        level_ids: Vec<u32>,
        markers: Vec<u32>,
        rows: usize,
        adapter: bool,
        now: u32,
    ) -> Self {
        let entries = fill_rows(&level_ids, rows);
        Self {
            slots: entries
                .into_iter()
                .map(|entry| DestinationSlot {
                    marker: markers.get(entry).copied().unwrap_or(0),
                    ..DestinationSlot::new(entry)
                })
                .collect(),
            state: state::REVEALING,
            reveal_until: now.wrapping_add(REVEAL_MS),
            close_until: 0,
            selected: 0,
            turn: 0.0,
            alpha: 0.0,
            flash_clock: now,
            level_ids,
            markers,
            adapter,
        }
    }

    /// The number of rows.
    pub fn rows(&self) -> usize {
        self.slots.len()
    }

    /// Starts the close: bit 1 is set and the tail runs for 256 ms (the
    /// original also pops its controller action layer).
    pub fn close(&mut self, now: u32) {
        self.state |= state::CLOSING;
        self.close_until = now.wrapping_add(CLOSE_MS);
    }

    /// The leave key: once the reveal is within 400 ms of its end (or
    /// over) and no close runs, the screen closes. Returns whether it did.
    pub fn leave(&mut self, now: u32) -> bool {
        let revealed = self.state & state::REVEALING == 0
            || i64::from(self.reveal_until) - LEAVE_GRACE_MS <= i64::from(now);
        if revealed && self.state & state::CLOSING == 0 {
            self.close(now);
            true
        } else {
            false
        }
    }

    /// Hovers row `row` (1-based; 0 or less is none). Returns 3, or 0 when
    /// neither the new nor the old row is a row (the base handles it).
    /// Both rows' flashes restart.
    pub fn hover(&mut self, row: i32, now: u32) -> i32 {
        if row < 1 && self.selected < 1 {
            return 0;
        }
        if row > 0
            && let Some(slot) = self.slots.get_mut(row as usize - 1)
        {
            bump_flash(slot, now);
        }
        if self.selected > 0
            && let Some(slot) = self.slots.get_mut(self.selected as usize - 1)
        {
            bump_flash(slot, now);
        }
        self.selected = row;
        3
    }

    /// The reveal and the close at `now`, as the paint steps them. A closed
    /// screen draws at zero; the close tail's end marks the screen closed
    /// and leaves the stored turn and alpha as they were.
    pub fn frame(&mut self, now: u32) -> Frame {
        let mut turn = 1.0f32;
        let mut alpha = 1.0f32;
        if self.state == state::CLOSED {
            turn = 0.0;
            alpha = 0.0;
        } else if self.state & state::REVEALING != 0 {
            if self.reveal_until > now {
                let left = f64::from(self.reveal_until - now);
                let x = (1.0 - left * REVEAL_PER_MS) as f32;
                let eased = x * (2.0 - x);
                alpha = eased * 1.0;
                if alpha > 1.0 {
                    alpha = 1.0;
                }
                turn = eased * (2.0 - eased);
            } else {
                self.state &= !state::REVEALING;
            }
        }
        if self.state & state::CLOSING != 0 {
            if self.close_until <= now {
                self.state = state::CLOSED;
                return Frame {
                    turn: self.turn,
                    alpha: self.alpha,
                    closed: true,
                };
            }
            alpha = (self.close_until - now) as f32 * alpha / CLOSE_MS as f32;
        }
        self.turn = turn;
        self.alpha = alpha;
        Frame {
            turn,
            alpha,
            closed: false,
        }
    }

    /// Places row `index` on the ring (see [`place_node`]).
    pub fn node(&self, index: i32, width: i32, height: i32, unit: i32) -> (f32, f32) {
        place_node(index, self.rows(), width, height, unit, self.turn)
    }

    /// The hit test of a point in the screen's own pixels, after the base
    /// found the screen itself under it: 0 while the close runs, else the
    /// last of the orbit nodes 1…3 within 14 units (an unordered compare,
    /// so a NaN distance hits), as row `node + 1`, or −1.
    pub fn hit(&self, point: (i32, i32), width: i32, height: i32, unit: i32) -> i32 {
        if self.state & state::CLOSING != 0 {
            return 0;
        }
        let limit = i64::from(unit) * HIT_RADIUS_UNITS;
        let limit = (limit * limit) as f32;
        let mut hit = -1;
        for node in 1..=HIT_NODES {
            let (x, y) = self.node(node, width, height, unit);
            let dx = point.0 as f32 - x;
            let dy = point.1 as f32 - y;
            let distance = dy * dy + dx * dx;
            // Only an ordered "greater" misses.
            if distance <= limit || distance.is_nan() {
                hit = node + 1;
            }
        }
        hit
    }

    /// The click rule for button mask `mask` (bit 0 the left button, bit 1
    /// the right). The click handler closes the screen on both a dismissal
    /// and a journey.
    pub fn click(&self, mask: u16) -> Click {
        if self.selected < -1 {
            return Click::Base;
        }
        if mask & 2 != 0 {
            return Click::Dismiss;
        }
        if self.state == 0 && mask & 1 != 0 && self.selected > 0 {
            let (entry, marker) = self
                .slots
                .get(self.selected as usize - 1)
                .map_or((0, 0), |slot| (slot.entry, slot.marker));
            return Click::Travel {
                level: self.level_ids.get(entry).copied().unwrap_or(0),
                marker,
            };
        }
        Click::Ignored
    }

    /// One paint at `now`: the reveal ([`Self::frame`]), the wobble of the
    /// orbit nodes (every node 1… with the adapter, the hub when it is the
    /// only row), then every row's symbol. `random` is the engine's unit
    /// draw. Returns `None` when the close ended this frame.
    pub fn paint(
        &mut self,
        now: u32,
        width: i32,
        height: i32,
        unit: i32,
        mut random: impl FnMut() -> f32,
    ) -> Option<Vec<SelectionDraw>> {
        let frame = self.frame(now);
        if frame.closed {
            return None;
        }
        let flash = flash_amount(now, self.flash_clock);
        self.flash_clock = now;
        let mut draw = vec![SelectionDraw::Backdrop];
        let rows = self.rows();
        for row in 0..rows {
            if (self.adapter && row > 0) || rows == 1 {
                wobble_step(&mut self.slots[row], flash, &mut random);
            }
            let slot = self.slots[row];
            let mix = node_mix(flash_fade(&slot, now), row, self.selected);
            let alpha = (f64::from(frame.alpha) * SYMBOL_ALPHA) as f32;
            let centre = self.node(row as i32, width, height, unit);
            draw.push(SelectionDraw::Symbol {
                row,
                quad: symbol_quad(centre, unit, self.turn, &slot),
                mix,
                alpha,
            });
        }
        Some(draw)
    }
}

/// Restarts a row's flash: from a deadline still ahead the deadline is
/// reflected through now plus 200 ms, else it is now + 200.
pub fn bump_flash(slot: &mut DestinationSlot, now: u32) {
    slot.flash_until = if slot.flash_until > now {
        now.wrapping_mul(2)
            .wrapping_add(HOVER_FLASH_MS)
            .wrapping_sub(slot.flash_until)
    } else {
        now.wrapping_add(HOVER_FLASH_MS)
    };
}

/// Places node `index` of `count` on a `width` × `height` screen. Node 0
/// is the centre; the others are on the ring at the angle `(1 − turn) +
/// (2.5 − index)·2π / (count − 1)`, their offsets rounded to integers
/// before `turn` scales them. A one-row screen divides by zero, as the
/// original does.
pub fn place_node(
    index: i32,
    count: usize,
    width: i32,
    height: i32,
    unit: i32,
    turn: f32,
) -> (f32, f32) {
    let centre_x = width / 2;
    let centre_y = height / 2;
    if index == 0 {
        return (centre_x as f32, centre_y as f32);
    }
    let spread = ((2.5 - index as f32) * std::f32::consts::TAU) / (count as i64 - 1) as f32;
    let angle = (1.0 - turn) + spread;
    let (sin, cos) = sin_cos(angle);
    let unit = unit as f32;
    let x = round_i64(unit * sin * RING_RADIUS_UNITS * 1.0) as f32;
    let y = round_i64(unit * cos * RING_RADIUS_UNITS) as f32;
    (x * turn + centre_x as f32, y * turn + centre_y as f32)
}

/// A symbol's quad: its corners and the shader's values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SymbolQuad {
    /// Left, top, right and bottom. The texture's v runs from 1 at the top
    /// to 0 at the bottom.
    pub corners: [f32; 4],
    /// `TexPos`: the atlas cell's size, then its origin.
    pub tex_pos: [f32; 4],
    /// `TexOfs`: the row's wobble times 0.1.
    pub tex_ofs: [f32; 2],
}

/// One fragment of the symbol shader at texture coordinates `(u, v)`: the
/// colour the shader writes. `colour` is the `Colour` uniform, the atlas is
/// its `Tex` (a coverage texture whose alpha is sampled), and `tex_pos` and
/// `tex_ofs` are the quad's uniforms. The body is the shader's own, term by
/// term, in single precision.
pub fn symbol_sample(
    atlas: &crate::gui_draw::GuiTexture,
    u: f32,
    v: f32,
    tex_pos: [f32; 4],
    tex_ofs: [f32; 2],
    colour: [f32; 4],
) -> [f32; 4] {
    let a = [u - 0.5, v - 0.5];
    let dot = a[0] * a[0] + a[1] * a[1];
    let mut b = (dot * 4.0).min(1.0);
    b *= b;
    b = 1.0 - b * b;
    b *= b;
    let mut c = (dot * 128.0 - 25.0).abs();
    c *= c;
    c = (1.0 - c).max(0.0);
    let mut d = (dot * 4.0).min(1.0);
    d = ((0.6 - d) * 16.0).clamp(0.0, 1.0);
    let mut t = [
        u * tex_pos[0] * 1.3 + tex_pos[2] - tex_pos[0] * 0.15,
        v * tex_pos[1] * 1.3 + tex_pos[3] - tex_pos[1] * 0.15,
    ];
    t[0] += tex_ofs[0] * 0.1;
    t[1] += tex_ofs[1] * 0.1;
    d *= 1.0 - (tex_ofs[0] * tex_ofs[0] + tex_ofs[1] * tex_ofs[1]) * 30.0;
    let texel = atlas.sample(t[0], t[1]);
    c += f32::from(texel[3]) / 255.0 * 0.6 * d;
    [colour[0] * c, colour[1] * c, colour[2] * c, b * colour[3]]
}

/// A row's symbol drawn through [`symbol_sample`] at one texel per pixel.
/// The image is `size` square, the quad's size rounded to an integer, and
/// its rows run top-down as the draw path stores them. `colour`'s alpha is
/// the symbol's own.
pub fn symbol_image(
    atlas: &crate::gui_draw::GuiTexture,
    size: i32,
    tex_pos: [f32; 4],
    tex_ofs: [f32; 2],
    colour: [f32; 4],
) -> Option<crate::gui_draw::GuiTexture> {
    let size = size.max(1) as u32;
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        // A stored-top-down image is sampled with v running up it.
        let v = 1.0 - (y as f32 + 0.5) / size as f32;
        for x in 0..size {
            let u = (x as f32 + 0.5) / size as f32;
            let [r, g, b, a] = symbol_sample(atlas, u, v, tex_pos, tex_ofs, colour);
            let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
            pixels.push([byte(r), byte(g), byte(b), byte(a)]);
        }
    }
    Some(crate::gui_draw::GuiTexture {
        width: size,
        height: size,
        pixels,
    })
}

/// The quad of a row's symbol centred at `centre`: 32 units times the
/// ring's turn, the atlas cell `(entry mod 2, entry / 2)` of the 2 × 2
/// sheet counted from the top, and the wobble as the texture offset.
pub fn symbol_quad(centre: (f32, f32), unit: i32, turn: f32, slot: &DestinationSlot) -> SymbolQuad {
    let size = ((unit << 5) as f32) * turn;
    let left = centre.0 - size * 0.5;
    let top = centre.1 - size * 0.5;
    let entry = slot.entry as i64;
    SymbolQuad {
        corners: [left, top, left + size, top + size],
        tex_pos: [
            0.5,
            0.5,
            (entry % 2) as f32 * 0.5,
            1.0 - (entry / 2 + 1) as f32 * 0.5,
        ],
        tex_ofs: [
            (f64::from(slot.wobble[0]) * TEX_OFFSET_SCALE) as f32,
            (f64::from(slot.wobble[1]) * TEX_OFFSET_SCALE) as f32,
        ],
    }
}

/// One step of the screen's paint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionDraw {
    /// The full-screen backdrop skin.
    Backdrop,
    /// A row's symbol: its quad, the share it mixes between the palette's
    /// two symbol colours and its alpha (the screen's times 0.6).
    Symbol {
        row: usize,
        quad: SymbolQuad,
        mix: f32,
        alpha: f32,
    },
}

/// Steps a row's wobble by one paint's `flash`. `random` returns [0, 1);
/// it is drawn twice per axis. A wobble at or above 0.9 drifts back by
/// `random · flash`, one at or below −0.9 forward, any other (a NaN too)
/// by `2·flash·(random − 0.5)`; then the wobble moves by `2·flash · drift
/// · random` and is held to ±1 (a NaN stays).
pub fn wobble_step(slot: &mut DestinationSlot, flash: f32, mut random: impl FnMut() -> f32) {
    for axis in 0..2 {
        let value = f64::from(slot.wobble[axis]);
        if value >= WOBBLE_BAND {
            slot.drift[axis] -= random() * flash;
        } else if value <= -WOBBLE_BAND {
            slot.drift[axis] += random() * flash;
        } else {
            let r = random() - 0.5;
            slot.drift[axis] += 2.0 * flash * r;
        }
        let r = random();
        slot.wobble[axis] += flash * 2.0 * slot.drift[axis] * r;
        // A NaN passes both compares and stays, as `clamp` keeps it.
        slot.wobble[axis] = slot.wobble[axis].clamp(-WOBBLE_LIMIT, WOBBLE_LIMIT);
    }
}

/// The wobble strength of a paint: the time since the last paint times
/// 0.02 (a double).
pub fn flash_amount(now: u32, last: u32) -> f32 {
    (f64::from(now.wrapping_sub(last)) * FLASH_PER_MS) as f32
}

/// A row's flash fade: 1 once its deadline has passed, else `1 − left ·
/// 0.005` (a double).
pub fn flash_fade(slot: &DestinationSlot, now: u32) -> f32 {
    if slot.flash_until < now {
        1.0
    } else {
        (1.0 - f64::from(slot.flash_until - now) * FLASH_FADE_PER_MS) as f32
    }
}

/// The colour mix a row draws with: the fade for the hovered row, one less
/// the fade for the others.
pub fn node_mix(fade: f32, row: usize, selected: i32) -> f32 {
    if i64::from(selected) == row as i64 + 1 {
        fade
    } else {
        1.0 - fade
    }
}

/// What a click does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// The hovered word is a control's: the base form handles it.
    Base,
    /// Nothing (the form returns 0x10).
    Ignored,
    /// The right button closes the screen.
    Dismiss,
    /// Start the journey to this level id and marker word and close the
    /// screen.
    Travel { level: u32, marker: u32 },
}

/// Power Crystal A, the teleporter's small crystal: the journey spends it.
pub const POWER_CRYSTAL: u32 = 0x1af;
/// The large crystal's adapter: its bubble holds.
pub const CRYSTAL_ADAPTER: u32 = 0x58f;
/// The bubble sits this far above the teleporter's origin (cm).
pub const BUBBLE_LIFT_CM: f32 = 98.0;
/// The level change's arrival word.
pub const ARRIVAL_WORD: u32 = 0xeeee_00da;
/// The bubble's state word when a resting bubble is told to shrink, and
/// when it is freed.
pub const BUBBLE_SHRINKING: i32 = 4;
pub const BUBBLE_FREED: i32 = 6;
/// The player's red health above which the journey's end saves (a double).
pub const SAVE_RED: f64 = 0.01;

/// The teleporter's glow a journey starts: rising over `ramp_ms`, held for
/// `hold_ms`, falling over `ramp_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JourneyGlow {
    pub ramp_ms: u32,
    pub hold_ms: u32,
}

/// The journey a click starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Journey {
    /// The bubble holds (the adapter) instead of resting and shrinking.
    pub hold: bool,
    /// The teleporter's glow, when a crystal pays.
    pub glow: Option<JourneyGlow>,
    /// The crystal's charge when the journey started (a small crystal's).
    pub stored: u8,
    /// The journey's start.
    pub started: u32,
    /// The row's level id.
    pub level: u32,
    /// The teleporter's marker word.
    pub marker: u32,
}

/// The journey from a teleporter whose marker word is `marker`, paid by
/// the crystal `crystal` (its record id and charge): the adapter holds the
/// bubble and glows 1.5 s up, 7 s held; any other crystal stores its
/// charge and glows 1 s up, 1.4 s held. No crystal, no glow.
pub fn journey(crystal: Option<(u32, u8)>, level: u32, marker: u32, now: u32) -> Journey {
    let mut journey = Journey {
        hold: false,
        glow: None,
        stored: 0,
        started: now,
        level,
        marker,
    };
    match crystal {
        Some((CRYSTAL_ADAPTER, _)) => {
            journey.hold = true;
            journey.glow = Some(JourneyGlow {
                ramp_ms: 1_500,
                hold_ms: 7_000,
            });
        }
        Some((_, charge)) => {
            journey.stored = charge;
            journey.glow = Some(JourneyGlow {
                ramp_ms: 1_000,
                hold_ms: 1_400,
            });
        }
        None => {}
    }
    journey
}

/// A small crystal's charge at `now`: the stored charge plus the elapsed
/// time over 12 (a signed 64-bit difference, truncated), held to 255
/// above (a negative sum is kept as it is).
pub fn crystal_charge(stored: u8, started: u32, now: u32) -> i32 {
    let elapsed = (i64::from(now) - i64::from(started)) / 12;
    let charge = (i64::from(stored) + elapsed) as i32;
    if charge > 255 { 255 } else { charge }
}

/// What a journey tick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TravelStep {
    /// The crystal's new charge byte.
    pub charge: Option<u8>,
    /// Change level: the row's level id with [`ARRIVAL_WORD`].
    pub change_level: Option<u32>,
    /// The player's controls are handed back after the menu.
    pub restore_controls: bool,
    /// The player's controls stop (the level change, or the journey's
    /// end).
    pub stop_controls: bool,
    /// The marker word whose record gets the single-row bit.
    pub mark: Option<u32>,
    /// The game saves.
    pub save: bool,
    /// The journey is over.
    pub done: bool,
}

/// One journey tick after the bubble's own step. `crystal` is the paying
/// crystal's record id and charge, if it is still there; `bubble` the
/// bubble's state word; `in_game` whether a game mode runs (the menu has
/// none); `player_red` the player's red health, if there is a player.
///
/// A small crystal (Power Crystal A) charges by [`crystal_charge`] every
/// tick. When the resting bubble is told to shrink, a game changes level
/// to the row's level (and stops the player's controls), the menu hands
/// the controls back. When the bubble is freed the crystal is spent (255),
/// and a held bubble marks the teleporter, stops the player's controls and
/// saves if the player's red health is above 0.01.
pub fn travel_step(
    journey: &Journey,
    crystal: Option<(u32, u8)>,
    bubble: i32,
    now: u32,
    in_game: bool,
    player_red: Option<f32>,
) -> TravelStep {
    let mut step = TravelStep::default();
    if let Some((POWER_CRYSTAL, current)) = crystal {
        let charge = crystal_charge(journey.stored, journey.started, now);
        if charge != i32::from(current) {
            step.charge = Some(charge as u8);
        }
    }
    if bubble == BUBBLE_SHRINKING {
        if in_game {
            step.change_level = Some(journey.level);
            step.stop_controls = true;
        } else {
            step.restore_controls = true;
        }
    } else if bubble == BUBBLE_FREED {
        if crystal.is_some() {
            step.charge = Some(255);
        }
        if journey.hold {
            step.mark = mark_destination(journey.marker);
            if let Some(red) = player_red {
                step.stop_controls = true;
                step.save = f64::from(red) > SAVE_RED;
            }
        }
        step.done = true;
    }
    step
}

/// The colours the destination screen paints with. The original takes its
/// two symbol colours and its backdrop from the skinned form; the draw
/// path's caller supplies them here (the line skin's text and highlight
/// colours are Exarust's stand-in [guess]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionPalette {
    /// The colour a row starts from (its colour at mix 0).
    pub symbol_base: [f32; 4],
    /// The colour added on top, scaled by the row's mix: the paint's
    /// `base + add · mix`, not a blend between the two.
    pub symbol_add: [f32; 4],
    /// The full-screen backdrop's ink.
    pub backdrop: [u8; 4],
}

/// The layer a symbol image draws as: a textured quad over its corners.
fn symbol_layer(
    quad: &SymbolQuad,
    image: crate::gui_draw::GuiTexture,
) -> crate::gui_draw::GuiLayer {
    let mut batch = crate::skin::SkinBatch::new();
    batch.select(2, 0, true);
    batch.blend = crate::gui_draw::BLEND_ALPHA;
    batch.tex_col = Some(1.0);
    batch.set_ink([255, 255, 255, 255]);
    batch.begin();
    let [left, top, right, bottom] = quad.corners;
    batch.set_uv(0.0, 1.0);
    batch.vertex(left, top);
    batch.set_uv(0.0, 0.0);
    batch.vertex(left, bottom);
    batch.set_uv(1.0, 0.0);
    batch.vertex(right, bottom);
    batch.set_uv(1.0, 1.0);
    batch.vertex(right, top);
    batch.end_quads();
    crate::gui_draw::GuiLayer::new(batch, crate::gui_draw::LayerTexture::Owned(image))
}

/// A frame's backdrop layer.
fn backdrop_layer(width: i32, height: i32, ink: [u8; 4]) -> crate::gui_draw::GuiLayer {
    let mut batch = crate::skin::SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink(ink);
    batch.begin();
    batch.vertex(0.0, 0.0);
    batch.vertex(0.0, height as f32);
    batch.vertex(width as f32, height as f32);
    batch.vertex(width as f32, 0.0);
    batch.end_quads();
    crate::gui_draw::GuiLayer::new(batch, crate::gui_draw::LayerTexture::None)
}

/// Turns one paint's steps into the draw path's frame: the backdrop, then
/// every symbol through [`symbol_image`], in the paint's order.
pub fn frame(
    draws: &[SelectionDraw],
    atlas: &crate::gui_draw::GuiTexture,
    palette: &SelectionPalette,
    width: i32,
    height: i32,
) -> crate::gui_draw::GuiFrame {
    let mut frame = crate::gui_draw::GuiFrame::new();
    for draw in draws {
        match *draw {
            SelectionDraw::Backdrop => {
                frame.push(backdrop_layer(width, height, palette.backdrop));
            }
            SelectionDraw::Symbol {
                quad, mix, alpha, ..
            } => {
                // The paint adds the second colour scaled by the mix to the
                // first, then overwrites the alpha.
                let colour = [
                    palette.symbol_base[0] + palette.symbol_add[0] * mix,
                    palette.symbol_base[1] + palette.symbol_add[1] * mix,
                    palette.symbol_base[2] + palette.symbol_add[2] * mix,
                    alpha,
                ];
                let size = crate::engine_math::round_i64(quad.corners[2] - quad.corners[0]);
                let Some(image) =
                    symbol_image(atlas, size as i32, quad.tex_pos, quad.tex_ofs, colour)
                else {
                    continue;
                };
                frame.push(symbol_layer(&quad, image));
            }
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_fill_in_the_originals_order() {
        assert_eq!(fill_rows(&[7, 10, 12, 13], 4), [3, 0, 1, 2]);
        assert_eq!(fill_rows(&[0, 7, 12, 13], 4), [0, 1, 2, 3]);
        assert_eq!(fill_rows(&[7, 0, 12, 13], 4), [1, 0, 2, 3]);
        assert_eq!(fill_rows(&[7, 10, 0, 13], 4), [2, 0, 1, 3]);
        assert_eq!(fill_rows(&[0, 0, 0, 7], 4), [2, 3, 0, 0]);
        assert_eq!(fill_rows(&[7, 10, 12, 13], 1), [0]);
        assert_eq!(fill_rows(&[7, 10, 12, 13], 0), Vec::<usize>::new());
        assert_eq!(fill_rows(&[7, 10, 12, 13, 14], 4), [3, 0, 1, 2]);
    }

    #[test]
    fn the_form_metrics_and_the_marker_word_are_pinned() {
        let metrics = layout(800, 600, true);
        assert_eq!((metrics.width, metrics.height), (800, 600));
        assert_eq!(metrics.flags, 0x1010);
        assert_eq!(metrics.style, 0x800);
        assert_eq!(metrics.orbit, 40.0);
        assert_eq!(layout(800, 600, false).flags, 0x10);
        assert_eq!(mark_destination(0x8000_0007), Some(7));
        assert_eq!(mark_destination(0xc000_0007), Some(7));
        assert_eq!(mark_destination(7), None);
        assert_eq!(FORM_RETURN, 4);
        assert!(SYMBOL_SHADER.contains("uniform sampler2D Tex;"));
    }

    #[test]
    fn the_reveal_eases_twice_for_the_ring_and_once_for_the_alpha() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12, 13], 4, false, 1_000);
        assert_eq!(screen.reveal_until, 1_500);
        assert_eq!(screen.state, state::REVEALING);
        assert_eq!(screen.slots[0].entry, 3);
        let frame = screen.frame(1_000);
        assert_eq!((frame.turn, frame.alpha), (0.0, 0.0));
        // 250 ms left: x = 0.5, e = 0.75; the ring's turn 0.75 · 1.25.
        let frame = screen.frame(1_250);
        assert_eq!(frame.alpha, 0.75);
        assert_eq!(frame.turn, 0.9375);
        // At the deadline the reveal bit clears and the screen is full.
        let frame = screen.frame(1_500);
        assert_eq!(screen.state, 0);
        assert_eq!((frame.turn, frame.alpha), (1.0, 1.0));
    }

    #[test]
    fn the_close_fades_over_its_tail_and_then_draws_nothing() {
        let mut screen = DestinationScreen::open(vec![7], 1, false, 0);
        screen.frame(600);
        screen.close(2_000);
        assert_eq!(screen.close_until, 2_256);
        assert_eq!(screen.frame(2_000).alpha, 1.0);
        let frame = screen.frame(2_156);
        assert_eq!(frame.alpha, 100.0 / 256.0);
        let frame = screen.frame(2_256);
        assert!(frame.closed);
        // The stored values are the last drawn.
        assert_eq!(frame.alpha, 100.0 / 256.0);
        assert_eq!(screen.state, state::CLOSED);
        // A closed screen draws at zero.
        let frame = screen.frame(2_300);
        assert_eq!((frame.turn, frame.alpha, frame.closed), (0.0, 0.0, false));
        assert_eq!(screen.alpha, 0.0);
    }

    #[test]
    fn the_leave_key_waits_for_the_last_four_hundred_ms_of_the_reveal() {
        let mut screen = DestinationScreen::open(vec![1], 1, false, 1_000);
        assert!(!screen.leave(1_099));
        assert!(screen.leave(1_100));
        assert_eq!(screen.close_until, 1_356);
        // A running close is not restarted.
        assert!(!screen.leave(1_200));
        assert_eq!(screen.close_until, 1_356);
        // A deadline that wrapped under 400 compares signed, so it is past.
        let mut early = DestinationScreen::open(vec![1], 1, false, u32::MAX - 200);
        assert_eq!(early.reveal_until, 299);
        assert!(early.leave(u32::MAX - 200));
    }

    #[test]
    fn nodes_use_the_engine_sine_across_and_the_cosine_down() {
        // Four rows on 200 × 100 at unit 4: radius 160 px.
        assert_eq!(place_node(0, 4, 200, 100, 4, 1.0), (100.0, 50.0));
        // Node 1: angle 1.5 · 2π / 3 = π: sin 0 (rounded), cos −1.
        assert_eq!(place_node(1, 4, 200, 100, 4, 1.0), (100.0, -110.0));
        // Node 2: 0.5 · 2π / 3 = π / 3: 160 · 0.866 = 138.56 → 139; 80.
        assert_eq!(place_node(2, 4, 200, 100, 4, 1.0), (239.0, 130.0));
        // Node 3: −π / 3.
        assert_eq!(place_node(3, 4, 200, 100, 4, 1.0), (-39.0, 130.0));
        // Half turned: the angle gains 0.5 rad and the offsets halve.
        let (s, c) = sin_cos(0.5 + std::f32::consts::PI);
        let x = round_i64(4.0 * s * 40.0) as f32 * 0.5 + 100.0;
        let y = round_i64(4.0 * c * 40.0) as f32 * 0.5 + 50.0;
        assert_eq!(place_node(1, 4, 200, 100, 4, 0.5), (x, y));
        assert_eq!((x, y), (61.5, -20.0));
    }

    #[test]
    fn the_hit_test_tries_three_orbit_nodes_and_none_while_closing() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12, 13], 4, false, 0);
        screen.frame(500);
        assert_eq!(screen.hit((100, -110), 200, 100, 4), 2);
        assert_eq!(screen.hit((239, 130), 200, 100, 4), 3);
        assert_eq!(screen.hit((-39, 130), 200, 100, 4), 4);
        // The hub is not a target.
        assert_eq!(screen.hit((100, 50), 200, 100, 4), -1);
        // 56 px is inside, a pixel more is not.
        assert_eq!(screen.hit((100, -54), 200, 100, 4), 2);
        assert_eq!(screen.hit((100, -53), 200, 100, 4), -1);
        screen.close(600);
        assert_eq!(screen.hit((100, -110), 200, 100, 4), 0);
        // A one-row screen's orbit nodes land nowhere a pointer can be.
        let mut single = DestinationScreen::open(vec![7], 1, false, 0);
        single.frame(500);
        assert_eq!(single.hit((100, 50), 200, 100, 4), -1);
    }

    #[test]
    fn a_symbol_is_thirty_two_units_with_its_cell_and_wobble() {
        let mut slot = DestinationSlot::new(3);
        slot.wobble = [0.5, -1.0];
        let quad = symbol_quad((100.0, 50.0), 4, 1.0, &slot);
        assert_eq!(quad.corners, [36.0, -14.0, 164.0, 114.0]);
        assert_eq!(quad.tex_pos, [0.5, 0.5, 0.5, 0.0]);
        assert_eq!(quad.tex_ofs, [0.05, -0.1]);
        let quad = symbol_quad((100.0, 50.0), 4, 0.5, &DestinationSlot::new(1));
        assert_eq!(quad.corners, [68.0, 18.0, 132.0, 82.0]);
        assert_eq!(quad.tex_pos, [0.5, 0.5, 0.5, 0.5]);
        let quad = symbol_quad((0.0, 0.0), 4, 1.0, &DestinationSlot::new(0));
        assert_eq!(quad.tex_pos, [0.5, 0.5, 0.0, 0.5]);
    }

    #[test]
    fn the_flash_reflects_a_deadline_ahead_or_restarts_it() {
        let mut slot = DestinationSlot::new(0);
        bump_flash(&mut slot, 1_000);
        assert_eq!(slot.flash_until, 1_200);
        slot.flash_until = 1_100;
        bump_flash(&mut slot, 1_000);
        assert_eq!(slot.flash_until, 1_100);
        slot.flash_until = 1_500;
        bump_flash(&mut slot, 1_000);
        assert_eq!(slot.flash_until, 700);
        // A deadline equal to now restarts.
        slot.flash_until = 1_000;
        bump_flash(&mut slot, 1_000);
        assert_eq!(slot.flash_until, 1_200);
    }

    #[test]
    fn hover_restarts_both_rows_and_returns_three() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12], 3, false, 1_000);
        assert_eq!(screen.hover(1, 1_000), 3);
        assert_eq!(screen.selected, 1);
        assert_eq!(screen.slots[0].flash_until, 1_200);
        assert_eq!(screen.hover(2, 1_000), 3);
        assert_eq!(screen.selected, 2);
        assert_eq!(screen.slots[0].flash_until, 1_000);
        assert_eq!(screen.slots[1].flash_until, 1_200);
        assert_eq!(screen.hover(0, 1_000), 3);
        assert_eq!(screen.selected, 0);
        assert_eq!(screen.hover(-1, 1_000), 0);
        assert_eq!(screen.selected, 0);
    }

    #[test]
    fn a_rows_fade_rises_to_its_deadline() {
        let mut slot = DestinationSlot::new(0);
        slot.flash_until = 1_100;
        assert_eq!(flash_fade(&slot, 1_000), 0.5);
        assert_eq!(flash_fade(&slot, 1_050), 0.75);
        assert_eq!(flash_fade(&slot, 1_100), 1.0);
        assert_eq!(flash_fade(&slot, 1_101), 1.0);
        assert_eq!(node_mix(0.25, 1, 2), 0.25);
        assert_eq!(node_mix(0.25, 1, 1), 0.75);
        assert_eq!(node_mix(0.25, 0, 0), 0.75);
    }

    #[test]
    fn the_wobble_compares_in_double_and_keeps_a_nan() {
        let mut slot = DestinationSlot::new(0);
        let mut draws = [0.75f32, 0.5, 0.0, 0.0].into_iter();
        wobble_step(&mut slot, 0.2, || draws.next().unwrap());
        assert!((slot.drift[0] - 0.1).abs() < 1e-6);
        assert!((slot.wobble[0] - 0.4 * 0.1 * 0.5).abs() < 1e-6);
        // 0.9 as a float is under the double 0.9: still the two-sided nudge.
        let mut slot = DestinationSlot::new(0);
        slot.wobble[0] = 0.9;
        let mut draws = [1.0f32, 0.0, 0.0, 0.0].into_iter();
        wobble_step(&mut slot, 0.2, || draws.next().unwrap());
        assert!((slot.drift[0] - 0.2).abs() < 1e-6);
        // Above it drifts back; at −0.9 or below forward.
        let mut slot = DestinationSlot::new(0);
        slot.wobble = [0.95, -0.95];
        let mut draws = [0.5f32, 0.0, 0.5, 0.0].into_iter();
        wobble_step(&mut slot, 0.2, || draws.next().unwrap());
        assert!((slot.drift[0] + 0.1).abs() < 1e-6);
        assert!((slot.drift[1] - 0.1).abs() < 1e-6);
        // The wobble is held to ±1; a NaN stays a NaN.
        let mut slot = DestinationSlot::new(0);
        slot.drift[0] = 100.0;
        slot.wobble[1] = f32::NAN;
        let mut draws = [0.5f32, 1.0, 0.5, 1.0].into_iter();
        wobble_step(&mut slot, 10.0, || draws.next().unwrap());
        assert_eq!(slot.wobble[0], 1.0);
        assert!(slot.wobble[1].is_nan());
        assert_eq!(flash_amount(1_050, 1_000), 1.0);
    }

    #[test]
    fn the_paint_wobbles_the_adapters_orbit_and_draws_every_row() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12, 13], 4, true, 0);
        screen.selected = 2;
        let mut draws = 0;
        let draw = screen
            .paint(600, 200, 100, 4, || {
                draws += 1;
                0.5
            })
            .unwrap();
        // Three orbit nodes, two axes, two draws each.
        assert_eq!(draws, 12);
        assert_eq!(screen.flash_clock, 600);
        assert_eq!(draw[0], SelectionDraw::Backdrop);
        assert_eq!(draw.len(), 5);
        let SelectionDraw::Symbol {
            row,
            quad,
            mix,
            alpha,
        } = draw[2]
        else {
            panic!();
        };
        assert_eq!(row, 1);
        assert_eq!(quad.corners, [36.0, -174.0, 164.0, -46.0]);
        // The hovered row with no flash running mixes fully.
        assert_eq!(mix, 1.0);
        assert_eq!(alpha, 0.6);
        assert!(matches!(draw[1], SelectionDraw::Symbol { mix: 0.0, .. }));
        // Without the adapter nothing wobbles but a lone hub.
        let mut plain = DestinationScreen::open(vec![7, 10, 12, 13], 4, false, 0);
        let mut draws = 0;
        plain.paint(600, 200, 100, 4, || {
            draws += 1;
            0.5
        });
        assert_eq!(draws, 0);
        let mut lone = DestinationScreen::open(vec![7], 1, false, 0);
        let mut draws = 0;
        lone.paint(600, 200, 100, 4, || {
            draws += 1;
            0.5
        });
        assert_eq!(draws, 4);
    }

    #[test]
    fn a_rows_marker_word_goes_to_the_journey() {
        let screen = DestinationScreen::open_with_markers(
            vec![7, 10],
            vec![0x8000_0007, 0x8000_000a],
            2,
            true,
            0,
        );
        assert_eq!(screen.slots[0].marker, 0x8000_000a);
        assert_eq!(screen.slots[1].marker, 0x8000_0007);
        let mut screen = screen;
        screen.frame(600);
        screen.selected = 2;
        assert_eq!(
            screen.click(1),
            Click::Travel {
                level: 7,
                marker: 0x8000_0007
            }
        );
    }

    #[test]
    fn the_symbol_shader_body_is_pinned() {
        let atlas = crate::gui_draw::GuiTexture::solid([0, 0, 0, 255]);
        let tex_pos = [0.5, 0.5, 0.5, 0.0];
        let colour = [0.25, 0.5, 0.75, 1.0];
        // The centre: c = 1 · 0.6 · coverage, alpha = colour's.
        let centre = symbol_sample(&atlas, 0.5, 0.5, tex_pos, [0.0, 0.0], colour);
        assert!((centre[0] - 0.15).abs() < 1e-6);
        assert!((centre[1] - 0.3).abs() < 1e-6);
        assert!((centre[2] - 0.45).abs() < 1e-6);
        assert!((centre[3] - 1.0).abs() < 1e-6);
        // A corner: the ring term is 0 and the coverage weight 0 too.
        let corner = symbol_sample(&atlas, 0.0, 0.0, tex_pos, [0.0, 0.0], colour);
        assert_eq!(corner, [0.0, 0.0, 0.0, 0.0]);
        // The wobble dims the sampled coverage by 1 − |offset|² · 30.
        let dim = symbol_sample(&atlas, 0.5, 0.5, tex_pos, [0.1, 0.0], colour);
        assert!((dim[0] - 0.15 * 0.7).abs() < 1e-6);
    }

    #[test]
    fn a_symbol_renders_through_the_shader_and_draws_in_order() {
        let atlas = crate::gui_draw::GuiTexture::solid([0, 0, 0, 255]);
        let image = symbol_image(&atlas, 4, [0.5, 0.5, 0.5, 0.0], [0.0, 0.0], [1.0; 4]).unwrap();
        assert_eq!((image.width, image.height), (4, 4));
        assert!(image.pixels.iter().any(|pixel| pixel[3] != 0));
        let draws = [
            SelectionDraw::Backdrop,
            SelectionDraw::Symbol {
                row: 0,
                quad: symbol_quad((100.0, 50.0), 4, 1.0, &DestinationSlot::new(1)),
                mix: 1.0,
                alpha: 0.6,
            },
        ];
        let palette = SelectionPalette {
            symbol_base: [0.25, 0.25, 0.25, 1.0],
            symbol_add: [0.5, 0.25, 0.0, 1.0],
            backdrop: [0, 0, 0, 128],
        };
        let frame = frame(&draws, &atlas, &palette, 200, 100);
        assert_eq!(frame.layers.len(), 2);
        assert!(frame.layers[1].batch.primitives.len() == 1);
        // At mix 1 the colour is base + add: (0.75, 0.5, 0.25). The atlas
        // coverage is 1, so the symbol's centre is that times
        // `c = ring + 0.6 · d`, where the centre's ring term is 0 and d is 1.
        let crate::gui_draw::LayerTexture::Owned(image) = &frame.layers[1].texture else {
            panic!("the symbol carries its image");
        };
        let centre = image.pixels[(image.height / 2 * image.width + image.width / 2) as usize];
        let expected = symbol_sample(
            &atlas,
            ((image.width / 2) as f32 + 0.5) / image.width as f32,
            1.0 - ((image.height / 2) as f32 + 0.5) / image.height as f32,
            symbol_quad((100.0, 50.0), 4, 1.0, &DestinationSlot::new(1)).tex_pos,
            [0.0, 0.0],
            [0.75, 0.5, 0.25, 0.6],
        );
        assert_eq!(
            centre[0],
            (expected[0].clamp(0.0, 1.0) * 255.0).round_ties_even() as u8
        );
        assert!(centre[0] > centre[1] && centre[1] > centre[2]);
    }

    #[test]
    fn clicks_travel_dismiss_or_go_to_the_base() {
        let mut screen = DestinationScreen::open(vec![7, 10, 12], 3, false, 1_000);
        // Still revealing: the left button does nothing.
        screen.selected = 2;
        assert_eq!(screen.click(1), Click::Ignored);
        screen.state = 0;
        // Row 2 shows entry 0 (7), row 1 entry 2 (12).
        assert_eq!(
            screen.click(1),
            Click::Travel {
                level: 7,
                marker: 0
            }
        );
        screen.selected = 1;
        assert_eq!(
            screen.click(1),
            Click::Travel {
                level: 12,
                marker: 0
            }
        );
        assert_eq!(screen.click(3), Click::Dismiss);
        assert_eq!(screen.click(0), Click::Ignored);
        screen.selected = 0;
        assert_eq!(screen.click(1), Click::Ignored);
        screen.selected = -2;
        assert_eq!(screen.click(2), Click::Base);
    }

    #[test]
    fn a_journey_takes_its_bubble_and_glow_from_the_crystal() {
        let adapter = journey(Some((CRYSTAL_ADAPTER, 9)), 7, 0x8000_0003, 100);
        assert!(adapter.hold);
        assert_eq!(
            adapter.glow,
            Some(JourneyGlow {
                ramp_ms: 1_500,
                hold_ms: 7_000
            })
        );
        assert_eq!(adapter.stored, 0);
        let small = journey(Some((POWER_CRYSTAL, 9)), 7, 0, 100);
        assert!(!small.hold);
        assert_eq!(small.stored, 9);
        assert_eq!(
            small.glow,
            Some(JourneyGlow {
                ramp_ms: 1_000,
                hold_ms: 1_400
            })
        );
        let none = journey(None, 7, 0, 100);
        assert_eq!((none.hold, none.glow), (false, None));
        // 12 ms a point, held to 255 above.
        assert_eq!(crystal_charge(0, 1_000, 1_011), 0);
        assert_eq!(crystal_charge(0, 1_000, 1_012), 1);
        assert_eq!(crystal_charge(250, 0, 120), 255);
        // A clock behind the start truncates toward zero, then goes below.
        assert_eq!(crystal_charge(5, 100, 89), 5);
        assert_eq!(crystal_charge(5, 100, 0), -3);
    }

    #[test]
    fn a_small_crystal_journey_charges_and_changes_level() {
        let small = journey(Some((POWER_CRYSTAL, 0)), 7, 0, 1_000);
        let step = travel_step(&small, Some((POWER_CRYSTAL, 0)), 1, 1_120, true, None);
        assert_eq!(step.charge, Some(10));
        assert!(!step.done);
        // The same charge writes nothing; another crystal does not charge.
        let step = travel_step(&small, Some((POWER_CRYSTAL, 10)), 1, 1_120, true, None);
        assert_eq!(step.charge, None);
        let step = travel_step(&small, Some((0x1f4, 0)), 1, 1_120, true, None);
        assert_eq!(step.charge, None);
        // The bubble told to shrink changes level in a game.
        let step = travel_step(&small, None, BUBBLE_SHRINKING, 3_300, true, Some(1.0));
        assert_eq!(step.change_level, Some(7));
        assert!(step.stop_controls);
        let step = travel_step(&small, None, BUBBLE_SHRINKING, 3_300, false, Some(1.0));
        assert_eq!(step.change_level, None);
        assert!(step.restore_controls);
        // Freed: the crystal is spent, nothing is marked or saved.
        let step = travel_step(
            &small,
            Some((POWER_CRYSTAL, 9)),
            BUBBLE_FREED,
            4_000,
            true,
            Some(1.0),
        );
        assert_eq!(step.charge, Some(255));
        assert_eq!((step.mark, step.save, step.done), (None, false, true));
    }

    #[test]
    fn an_adapter_journey_marks_and_saves_at_its_end() {
        let held = journey(Some((CRYSTAL_ADAPTER, 0)), 7, 0x8000_0003, 0);
        let step = travel_step(&held, Some((CRYSTAL_ADAPTER, 0)), 3, 5_000, true, Some(0.5));
        assert_eq!(step, TravelStep::default());
        let step = travel_step(
            &held,
            Some((CRYSTAL_ADAPTER, 0)),
            BUBBLE_FREED,
            8_800,
            true,
            Some(0.5),
        );
        assert_eq!(step.charge, Some(255));
        assert_eq!(step.mark, Some(3));
        assert!(step.stop_controls && step.save && step.done);
        // The red health compares in double: 0.01 as a float is under it.
        let step = travel_step(&held, None, BUBBLE_FREED, 8_800, true, Some(0.01));
        assert!(!step.save);
        let step = travel_step(&held, None, BUBBLE_FREED, 8_800, true, Some(f32::NAN));
        assert!(!step.save);
        // No player: nothing saves.
        let step = travel_step(&held, None, BUBBLE_FREED, 8_800, true, None);
        assert!(!step.save && !step.stop_controls);
    }
}
