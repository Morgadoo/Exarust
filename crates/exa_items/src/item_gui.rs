//! The item GUI's own controls and helpers: the crystal sockets of a
//! weapon or a garment, the coin, rating-dot and slider controls of an item
//! card, the drag payloads, the popup's place, the icon renderer's framing
//! and the halo every icon gets.
//!
//! A socket is 16 bits: the crystal kind in bits 0–6 (1, 2, 4 and `0x11`
//! for crystals A, B, C and AR), `0x80` when the socket exists, and the
//! charge in the high byte. Dropping a crystal into a socket gives the one
//! it held back to the player's pack with its charge; dragging one out
//! leaves the socket empty when the drag ends. An AR crystal in any socket
//! fires the game's fourth function. Rating dots are drawn whole up to the
//! value less a quarter, with a half dot for a remainder above a quarter.
//! An icon's halo makes each inner pixel's alpha the clamped sum of the
//! five-pixel cross around it. Evidence: the item GUI notes
//! (`context/re/item-gui-lane.md`).

use crate::possession::crystal;

/// The item drag payload's type, and a scene object's.
pub const ITEM_PAYLOAD: u32 = 0xFB01;
pub const OBJECT_PAYLOAD: u32 = 0xFB02;
/// The return codes of the item GUI's control procedures.
pub const IGNORE: u32 = 0x80_0000;
pub const HANDLED: u32 = 1;
pub const DROP_HANDLED: u32 = 3;
pub const CARD_OPENED: u32 = 0x40;
pub const DRAG_STARTED: u32 = 0x105;
/// The event bits a control procedure is called with.
pub const EVENT_DRAW: u32 = 2;
pub const EVENT_HOVER: u32 = 0x10;
pub const EVENT_MOUSE: u32 = 0x40;
pub const EVENT_DRAG_OVER: u32 = 0x2000;
pub const EVENT_DROP: u32 = 0x3000;
pub const EVENT_DRAG_START: u32 = 0x4000;
/// A socket exists.
pub const SOCKET: u8 = 0x80;
/// The kind bits of a socket.
pub const KIND: u16 = 0x7F;
/// The game function an AR crystal in a socket fires.
pub const AR_FUNCTION: u32 = 4;
/// The size of a rating dot and of the slider's knob, in pixels.
pub const DOT: i32 = 22;
/// The crystals a socket takes, by database id.
pub const SOCKET_CRYSTALS: [u32; 4] = [crystal::A, crystal::B, crystal::C, crystal::AR];
/// The sprite sheets the item GUI loads, how many cells each has and the
/// size and stored places of its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteSheet {
    pub name: &'static str,
    pub cells: u32,
    pub cell: i32,
    /// The stored cell corners, top-left first (the sheets are stored bottom
    /// row first like every texture).
    pub places: &'static [(i32, i32)],
}

const DOTRATING_PLACES: [(i32, i32); 3] = [(0, 0), (64, 0), (128, 0)];
const COIN_PLACES: [(i32, i32); 3] = [(0, 0), (48, 0), (96, 0)];
const GRIP_PLACES: [(i32, i32); 3] = [(0, 96), (96, 96), (192, 96)];
const CRYSTAL_PLACES: [(i32, i32); 6] = [(0, 128), (64, 128), (0, 64), (0, 0), (64, 0), (64, 64)];

pub const SPRITE_SHEETS: [SpriteSheet; 4] = [
    SpriteSheet {
        name: "dotrating.rfi",
        cells: 3,
        cell: 64,
        places: &DOTRATING_PLACES,
    },
    SpriteSheet {
        name: "coinicons.rfi",
        cells: 3,
        cell: 48,
        places: &COIN_PLACES,
    },
    SpriteSheet {
        name: "wpnflpgrpico.rfi",
        cells: 3,
        cell: 96,
        places: &GRIP_PLACES,
    },
    SpriteSheet {
        name: "crystalicons.rfi",
        cells: 6,
        cell: 64,
        places: &CRYSTAL_PLACES,
    },
];
/// The names and cell counts of the sheets the item GUI loads.
pub const SPRITES: [(&str, u32); 4] = [
    ("dotrating.rfi", 3),
    ("coinicons.rfi", 3),
    ("wpnflpgrpico.rfi", 3),
    ("crystalicons.rfi", 6),
];

/// The socket kind of a crystal id.
pub fn kind_of(id: u32) -> Option<u8> {
    match id {
        crystal::A => Some(1),
        crystal::B => Some(2),
        crystal::C => Some(4),
        crystal::AR => Some(0x11),
        _ => None,
    }
}

/// The crystal id of a socket kind.
pub fn crystal_of(kind: u8) -> Option<u32> {
    match kind {
        1 => Some(crystal::A),
        2 => Some(crystal::B),
        4 => Some(crystal::C),
        0x11 => Some(crystal::AR),
        _ => None,
    }
}

/// The four crystal sockets of an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sockets(pub [u16; 4]);

impl Sockets {
    pub fn exists(&self, i: usize) -> bool {
        self.0.get(i).is_some_and(|s| s & u16::from(SOCKET) != 0)
    }

    pub fn kind(&self, i: usize) -> u8 {
        self.0.get(i).map_or(0, |s| (s & KIND) as u8)
    }

    pub fn charge(&self, i: usize) -> u8 {
        self.0.get(i).map_or(0, |s| (s >> 8) as u8)
    }

    /// Drops crystal `id` (its drag flags carry the charge in bits 16–23)
    /// into socket `i`. A crystal already there goes back with its charge.
    /// Refused for anything that is not one of the four socket crystals, and
    /// for a drag from this very socket.
    pub fn drop_crystal(&mut self, i: usize, id: u32, flags: u32, from_here: bool) -> SocketDrop {
        let Some(kind) = kind_of(id) else {
            return SocketDrop::Refused;
        };
        let Some(socket) = self.0.get_mut(i) else {
            return SocketDrop::Refused;
        };
        let old = (*socket & KIND) as u8;
        let mut back = None;
        if old != 0 {
            if from_here {
                return SocketDrop::Refused;
            }
            back = crystal_of(old).map(|c| (c, u32::from((*socket >> 8) as u8) << 16));
        }
        let low = (*socket as u8) & SOCKET | kind;
        let charge = (flags >> 16) as u8;
        *socket = u16::from(low) | u16::from(charge) << 8;
        SocketDrop::Placed { back }
    }

    /// The crystal dragged out of socket `i`, as `(id, flags)`; the socket
    /// keeps it until the drag ends.
    pub fn drag_out(&self, i: usize) -> Option<(u32, u32)> {
        crystal_of(self.kind(i)).map(|c| (c, u32::from(self.charge(i)) << 16))
    }

    /// The drag out of socket `i` ended: the socket's low byte becomes the
    /// bare socket mark; the charge byte above it is left as it was.
    pub fn emptied(&mut self, i: usize) {
        if let Some(s) = self.0.get_mut(i) {
            *s = (*s & 0xFF00) | u16::from(SOCKET);
        }
    }

    /// Whether an AR crystal sits in any socket (the kind's low five bits
    /// are `0x11`).
    pub fn holds_ar(&self) -> bool {
        self.0.iter().any(|s| s & 0x1F == 0x11)
    }

    /// The socket under `x` (pixels from the control's left), 1-based in
    /// bits 16 and up, or the control's "nothing" answer `0x80_0000`.
    pub fn hit(&self, x: i32, unit: i32) -> u32 {
        let i = x / (5 * unit.max(1));
        if (0..4).contains(&i) && self.exists(i as usize) {
            ((i + 1) as u32) << 16
        } else {
            IGNORE
        }
    }

    /// Whether a crystal can be dropped on the row: only the four power
    /// crystals.
    pub fn accepts_id(id: u32) -> bool {
        SOCKET_CRYSTALS.contains(&id)
    }

    /// The crystal shown by a right click on socket `i`: its id and the
    /// flags that carry its charge.
    pub fn card_at(&self, i: usize) -> Option<(u32, u32)> {
        self.drag_out(i)
    }

    /// Starts dragging the crystal in socket `i`; the socket keeps its
    /// crystal until the drag ends (`hold_drag`). Returns the crystal and
    /// the drag flag word.
    pub fn start_drag(&self, i: usize) -> Option<(u32, u32)> {
        self.drag_out(i)
    }

    /// Drops a crystal dragged over the row into socket `i`. A crystal
    /// already there is handed back unless the drag came from this very
    /// socket, in which case nothing changes.
    pub fn drop_from(
        &mut self,
        i: usize,
        id: u32,
        flags: u32,
        from_here: bool,
    ) -> SocketDropResult {
        match self.drop_crystal(i, id, flags, from_here) {
            SocketDrop::Refused => SocketDropResult {
                back: None,
                code: 0,
            },
            SocketDrop::Placed { back } => SocketDropResult {
                back,
                code: DROP_HANDLED,
            },
        }
    }

    /// The drag that took the crystal out of socket `i` ended: the socket
    /// is left empty.
    pub fn drag_ended(&mut self, i: usize) {
        self.emptied(i);
    }

    /// What committing the row into its item does: the four socket words
    /// are written into the item's weapon section when it has one (and the
    /// weapon's effects refresh), else into its apparel section (and the
    /// player's abilities refresh), else nowhere. Either write fires the
    /// game's fourth function when a socket holds the AR crystal.
    pub fn commit(&self, weapon: bool, apparel: bool) -> Option<SocketCommit> {
        let target = if weapon {
            CommitTarget::Weapon
        } else if apparel {
            CommitTarget::Apparel
        } else {
            return None;
        };
        Some(SocketCommit {
            target,
            ar: self.holds_ar(),
        })
    }

    /// What the socket row draws: each existing socket's frame 5 units
    /// apart, then for each socket with a crystal (at its own slot) the
    /// frame variant (2 for kinds with bit `0x10`), the crystal icon (3, 4,
    /// 5, 6 for A, B, C, AR) and its opacity, `255 − charge`.
    pub fn icons(&self, x: i32, unit: i32) -> SocketIcons {
        let mut frames = Vec::new();
        let mut fx = x;
        for i in 0..4 {
            if self.exists(i) {
                frames.push(fx);
                fx += 5 * unit;
            }
        }
        let mut crystals = Vec::new();
        for i in 0..4 {
            let kind = self.kind(i);
            if kind != 0 {
                let frame = if kind & 0x10 == 0 { 1 } else { 2 };
                let icon = match kind {
                    1 => Some(3),
                    2 => Some(4),
                    4 => Some(5),
                    0x11 => Some(6),
                    _ => None,
                };
                crystals.push(CrystalIcon {
                    x: x + 5 * unit * i as i32,
                    frame,
                    icon,
                    alpha: 255 - self.charge(i),
                });
            }
        }
        SocketIcons { frames, crystals }
    }
}

/// What a crystal dropped on a socket did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketDrop {
    Refused,
    /// It sits in the socket; `back` is the crystal that was there, as
    /// `(id, flags)` for the player's pack.
    Placed {
        back: Option<(u32, u32)>,
    },
}

/// What dropping a crystal on the socket row did, with the control's
/// return code (`3` handled).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketDropResult {
    pub back: Option<(u32, u32)>,
    pub code: u32,
}

/// Which section of the item a socket row is written into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitTarget {
    /// The weapon section; the weapon's effects refresh.
    Weapon,
    /// The apparel section; the player's abilities refresh.
    Apparel,
}

/// What committing a socket row into its item does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketCommit {
    pub target: CommitTarget,
    /// An AR crystal fires the game's fourth function.
    pub ar: bool,
}

/// What a card control's event procedure answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAnswer {
    /// The original returns 0.
    Nothing,
    /// The original returns its "ignore" code.
    Ignore,
    /// The original returns 1.
    Hit,
    /// The original runs its press callback and returns 0.
    Press,
}

/// The rating-dot and value controls: a draw draws and answers nothing, a
/// hover always answers "ignore", anything else answers nothing.
pub fn value_control_event(event: u32) -> ControlAnswer {
    if event > 1 {
        if event == EVENT_HOVER {
            return ControlAnswer::Ignore;
        }
        if event == EVENT_DRAW {
            return ControlAnswer::Nothing;
        }
    }
    ControlAnswer::Nothing
}

/// The item-icon cell: a hover answers "hit" or "ignore"; a left press with
/// the drag flag answers `Press` when a handler exists, else nothing; a draw
/// and every other event answer nothing.
pub fn icon_cell_event(event: u32, hit: bool, action: u32, handler: bool) -> ControlAnswer {
    if event <= 1 {
        return ControlAnswer::Nothing;
    }
    match event {
        EVENT_HOVER => {
            if hit {
                ControlAnswer::Hit
            } else {
                ControlAnswer::Ignore
            }
        }
        EVENT_MOUSE => {
            if action & 7 == 1 && action & EVENT_DRAG_OVER != 0 && handler {
                ControlAnswer::Press
            } else {
                ControlAnswer::Nothing
            }
        }
        _ => ControlAnswer::Nothing,
    }
}

/// One drawn crystal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrystalIcon {
    pub x: i32,
    pub frame: u8,
    pub icon: Option<u8>,
    pub alpha: u8,
}

/// The socket row's drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketIcons {
    pub frames: Vec<i32>,
    pub crystals: Vec<CrystalIcon>,
}

/// Where the coin control draws a coin dword's gold (the high 16 bits),
/// silver and copper: each present denomination's icon, then its number 4
/// units further; the next denomination starts 2 units after the number
/// ends. `width` measures a number's text.
pub fn coin_layout<F>(coins: u32, x: f32, unit: f32, mut width: F) -> Vec<(u8, f32, u32)>
where
    F: FnMut(u32) -> f32,
{
    let parts = [
        (1u8, coins >> 16),
        (2, (coins >> 8) & 0xFF),
        (3, coins & 0xFF),
    ];
    let mut out = Vec::new();
    let mut at = x;
    for (icon, n) in parts {
        if n != 0 {
            out.push((icon, at, n));
            let end = at + 4.0 * unit + width(n);
            at = 2.0 * unit + end;
        }
    }
    out
}

/// Where a rating's dots go: whole dots at `x` and every spacing (3 units,
/// or 2 units and half a unit rounded up above a rating of 7.25), then the
/// half dot's place when the remainder is above a quarter. A negative
/// rating draws nothing.
pub fn rating_dots(value: f32, x: i32, unit: i32) -> (Vec<i32>, Option<i32>) {
    if value < 0.0 || value.is_nan() {
        return (Vec::new(), None);
    }
    let step = if value <= 7.25 {
        unit * 3
    } else {
        unit * 2 + ((unit + 2) >> 1)
    };
    let whole = (value - 0.25).round_ties_even() as i32;
    let mut at = x;
    let mut dots = Vec::new();
    for _ in 0..whole.max(0) {
        dots.push(at);
        at += step;
    }
    let half = (value - whole as f32 > 0.25).then_some(at);
    (dots, half)
}

/// The slider's line and knob: the knob centred at `share` of the width
/// from the line's start (half a knob in); the line stops 0.195 knob short
/// of it on each side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slider {
    pub left: Option<(i32, i32)>,
    pub right: Option<(i32, i32)>,
    pub knob: i32,
}

pub fn slider(x: i32, width: i32, share: f32) -> Slider {
    let start = (DOT >> 1) + x;
    let centre = start + (width as f32 * share).round_ties_even() as i32;
    let gap = (f64::from(DOT) * 0.195).round_ties_even() as i32;
    let (a, b) = (centre - gap, centre + gap);
    Slider {
        left: (start < a).then_some((start, a)),
        right: (b < start + width).then_some((b, start + width)),
        knob: centre - (DOT >> 1),
    }
}

/// The side the card's striking rows show: the second when the grip
/// button's mask carries either grip bit.
pub fn card_side(mask: u32) -> usize {
    usize::from(mask & 0x60 != 0)
}

/// A drag in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Payload {
    pub kind: u32,
    /// The icon slot of an item payload.
    pub icon: usize,
    pub flags: u32,
    /// Where the icon is held, from its top-left.
    pub grab: (i32, i32),
}

/// An item payload started by code (a crystal pulled from its socket): the
/// icon is held by its centre.
pub fn item_payload(icon: usize, flags: u32, icon_size: i32) -> Option<Payload> {
    (icon > 0).then_some(Payload {
        kind: ITEM_PAYLOAD,
        icon,
        flags,
        grab: (icon_size >> 1, icon_size >> 1),
    })
}

/// A scene object's payload carries its condition word.
pub fn object_payload(condition: u32) -> Payload {
    Payload {
        kind: OBJECT_PAYLOAD,
        icon: 0,
        flags: condition,
        grab: (0, 0),
    }
}

/// An item payload started from a database id: the id's icon is looked up
/// through `resolve`, and a missing icon starts no drag.
pub fn item_payload_for_id<F>(
    id: u32,
    flags: u32,
    icon_size: i32,
    mut resolve: F,
) -> Option<Payload>
where
    F: FnMut(u32, u32) -> Option<usize>,
{
    let icon = resolve(id, flags & 0x0FFF_FFFF)?;
    item_payload(icon, flags, icon_size)
}

impl Payload {
    /// Cancelling an item drag releases the icon it holds, if any, and
    /// clears the payload's kind (true); any other payload is left alone.
    pub fn cancel<T>(&self, mut release: impl FnMut(usize) -> T) -> bool {
        if self.kind != ITEM_PAYLOAD {
            return false;
        }
        if self.icon != 0 {
            release(self.icon);
        }
        true
    }
}

/// Where a dragged item's icon is drawn: at the cursor less the grab,
/// unless the form under it hides drags (its flag `0x10`).
pub fn drag_icon_at(
    cursor: (i32, i32),
    grab: (i32, i32),
    hovered_flags: Option<u32>,
) -> Option<(i32, i32)> {
    if hovered_flags.is_some_and(|f| f & 0x10 != 0) {
        return None;
    }
    Some((cursor.0 - grab.0, cursor.1 - grab.1))
}

/// Where an item popup goes: centred on the cursor across, kept on the
/// screen, its bottom 8 units below the cursor, and never above one unit.
pub fn popup_place(
    cursor: (i32, i32),
    size: (i32, i32),
    screen_width: i32,
    unit: i32,
) -> (i32, i32) {
    let mut x = cursor.0 - ((size.0 as u32) >> 1) as i32;
    if x < 0 {
        x = 0;
    } else if i64::from(screen_width) < i64::from(size.0) + i64::from(x) {
        x = screen_width - size.0;
    }
    let y = (cursor.1 - (size.1 - 8 * unit)).max(unit);
    (x, y)
}

/// The weapon card's striking-side rows: slash, crush and pierce (0, 1, 2)
/// of side `side` (the grip button's), each shown when its byte is not 0,
/// at that byte × 0.05 in double precision narrowed to single. The shown
/// ones fill the card's rows from the first in that order and the rest of
/// the three rows are hidden; the item stores the second side six bytes
/// after the first.
pub fn side_rows(sides: [[u8; 3]; 2], side: usize) -> Vec<(usize, f32)> {
    let bytes = sides[side.min(1)];
    (0..3)
        .filter(|&r| bytes[r] != 0)
        .map(|r| (r, (f64::from(bytes[r]) * 0.05) as f32))
        .collect()
}

/// The icon manager a drawn control uses, and its cell size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconManager {
    /// Coins: a cell four units wide.
    Coins,
    /// The grip-side icons: eight units.
    Grip,
    /// The crystal sockets: the rating-dot size, 22 pixels.
    Sockets,
}

pub fn manager_cell(manager: IconManager, unit: i32) -> i32 {
    match manager {
        IconManager::Coins => unit * 4,
        IconManager::Grip => unit * 8,
        IconManager::Sockets => DOT,
    }
}

/// A two-stage resolution: the item's object record is looked up first, then
/// its icon. A streamed item whose icon is still pending is retried; a
/// missing record or icon is the "Unable to resolve item" error.
pub fn resolve_item<F, G>(id: u32, flags: u32, mut record: F, mut icon: G) -> Result<usize, String>
where
    F: FnMut(u32, u32) -> bool,
    G: FnMut(u32, u32) -> Option<usize>,
{
    if !record(id, flags) {
        return Err(crate::icon_cache::unresolved(id).unwrap_or_default());
    }
    icon(id, flags)
        .filter(|slot| *slot > 0)
        .ok_or_else(|| crate::icon_cache::unresolved(id).unwrap_or_default())
}

/// The icon renderer's framing for an item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Framing {
    /// The camera's framing distance.
    pub distance: f32,
    /// The sun's height and depth.
    pub sun: (f32, f32),
    /// The model is turned (−0.1, 0.1, 0.5) first.
    pub turned: bool,
    /// The halo is the highlighted one.
    pub highlight: bool,
}

/// The renderer's framing for a variant: 20 when the cache asks for close
/// icons, else 30 for apparel framing and 70 otherwise; the weapon light
/// preset for weapons; turned when both the apparel and weapon bits are set.
pub fn framing(variant: u32, close: bool) -> Framing {
    use crate::icon_cache::variant as v;
    let distance = if close {
        20.0
    } else if variant & v::APPAREL != 0 {
        30.0
    } else {
        70.0
    };
    let sun = if variant & v::WEAPON != 0 {
        (-0.16, 0.6516)
    } else {
        (-0.21, 0.7516)
    };
    Framing {
        distance,
        sun,
        turned: variant & (v::APPAREL | v::WEAPON) == v::APPAREL | v::WEAPON,
        highlight: variant & v::HIGHLIGHT != 0,
    }
}

/// The icon halo, on an RGBA image `width` by `height`: every pixel not on
/// the border takes as alpha the sum of its own and its four neighbours'
/// alpha, at most 255. The highlighted halo also lifts green by what alpha
/// gained, at most 255.
pub fn halo(rgba: &mut [u8], width: usize, height: usize, highlight: bool) {
    if width < 3 || height < 3 || rgba.len() < width * height * 4 {
        return;
    }
    let a = |p: &[u8], x: usize, y: usize| u32::from(p[(y * width + x) * 4 + 3]);
    let mut sums = vec![0u8; width * height];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let s = a(rgba, x + 1, y)
                + a(rgba, x - 1, y)
                + a(rgba, x, y - 1)
                + a(rgba, x, y)
                + a(rgba, x, y + 1);
            sums[y * width + x] = s.min(255) as u8;
        }
    }
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let i = (y * width + x) * 4;
            let s = sums[y * width + x];
            if highlight {
                let g = i32::from(s) - i32::from(rgba[i + 3]) + i32::from(rgba[i + 1]);
                rgba[i + 1] = if g < 256 { g as u8 } else { 255 };
            }
            rgba[i + 3] = s;
        }
    }
}

/// Fills a panel from an explicit id list, as the arsenal's columns are
/// filled: each id that resolves sits at the panel's centre less half an
/// icon, with the given flags, and its bulk counts; the rest are skipped.
pub fn fill_from_ids<F>(
    ids: &[u32],
    flags: u32,
    panel: (u32, u32),
    icon_size: u32,
    mut resolve: F,
) -> (Vec<(u32, u32, i32, i32)>, u32)
where
    F: FnMut(u32) -> Option<u32>,
{
    let at = (
        ((panel.0 >> 1) - (icon_size >> 1)) as i32,
        ((panel.1 >> 1) - (icon_size >> 1)) as i32,
    );
    let mut used = 0;
    let mut out = Vec::new();
    for &id in ids {
        if let Some(bulk) = resolve(id) {
            used += bulk;
            out.push((id, flags, at.0, at.1));
        }
    }
    (out, used)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crystals_go_in_and_come_back() {
        let mut s = Sockets([0x80, 0x80 | 0x02 | 0x4000, 0, 0x80]);
        assert!(s.exists(0) && !s.exists(2));
        assert_eq!(s.drop_crystal(0, 12, 0, false), SocketDrop::Refused);
        assert_eq!(
            s.drop_crystal(0, crystal::A, 0x00C8_0000, false),
            SocketDrop::Placed { back: None }
        );
        assert_eq!(s.0[0], 0x80 | 1 | 0xC800);
        // A crystal B was in socket 1: it goes back with its charge 0x40.
        assert_eq!(
            s.drop_crystal(1, crystal::AR, 0, false),
            SocketDrop::Placed {
                back: Some((crystal::B, 0x0040_0000))
            }
        );
        assert_eq!(s.kind(1), 0x11);
        assert!(s.holds_ar());
        assert_eq!(s.drop_crystal(1, crystal::C, 0, true), SocketDrop::Refused);
        assert_eq!(s.drag_out(0), Some((crystal::A, 0x00C8_0000)));
        s.emptied(0);
        // Only the low byte is rewritten; the charge 0xC8 stays.
        assert_eq!(s.0[0], 0xC880);
        assert_eq!(s.hit(25, 4), 2 << 16);
        assert_eq!(s.hit(45, 4), 0x80_0000);
        assert_eq!(
            (kind_of(crystal::C), crystal_of(4)),
            (Some(4), Some(crystal::C))
        );
        assert_eq!(kind_of(crystal::AL), None);
    }

    #[test]
    fn the_socket_row_draws() {
        let s = Sockets([0x80 | 1 | 0x1000, 0, 0x80, 0x80 | 0x11]);
        let icons = s.icons(100, 4);
        assert_eq!(icons.frames, vec![100, 120, 140]);
        assert_eq!(icons.crystals.len(), 2);
        assert_eq!(
            icons.crystals[0],
            CrystalIcon {
                x: 100,
                frame: 1,
                icon: Some(3),
                alpha: 239
            }
        );
        assert_eq!(
            icons.crystals[1],
            CrystalIcon {
                x: 160,
                frame: 2,
                icon: Some(6),
                alpha: 255
            }
        );
    }

    #[test]
    fn coins_dots_and_the_slider() {
        let layout = coin_layout(0x0003_0500, 10.0, 4.0, |n| n as f32);
        assert_eq!(layout, vec![(1, 10.0, 3), (2, 10.0 + 16.0 + 3.0 + 8.0, 5)]);
        let (dots, half) = rating_dots(3.5, 0, 4);
        assert_eq!(dots, vec![0, 12, 24]);
        assert_eq!(half, Some(36));
        let (dots, half) = rating_dots(8.0, 0, 4);
        assert_eq!(dots.len(), 8);
        assert_eq!(dots[1], 11);
        assert_eq!(half, None);
        assert_eq!(rating_dots(-1.0, 0, 4), (vec![], None));
        let s = slider(0, 200, 0.5);
        assert_eq!(s.knob, 100);
        assert_eq!((s.left, s.right), (Some((11, 107)), Some((115, 211))));
        assert_eq!(slider(0, 200, 0.0).left, None);
    }

    #[test]
    fn the_card_controls_answer_their_events() {
        assert_eq!(value_control_event(EVENT_DRAW), ControlAnswer::Nothing);
        assert_eq!(value_control_event(EVENT_HOVER), ControlAnswer::Ignore);
        assert_eq!(value_control_event(EVENT_MOUSE), ControlAnswer::Nothing);
        assert_eq!(value_control_event(1), ControlAnswer::Nothing);
        assert_eq!(
            icon_cell_event(EVENT_HOVER, true, 0, false),
            ControlAnswer::Hit
        );
        assert_eq!(
            icon_cell_event(EVENT_HOVER, false, 0, false),
            ControlAnswer::Ignore
        );
        assert_eq!(
            icon_cell_event(EVENT_MOUSE, true, 0x2001, true),
            ControlAnswer::Press
        );
        assert_eq!(
            icon_cell_event(EVENT_MOUSE, true, 0x2001, false),
            ControlAnswer::Nothing
        );
        assert_eq!(
            icon_cell_event(EVENT_DRAW, true, 0, true),
            ControlAnswer::Nothing
        );
    }

    #[test]
    fn an_item_resolution_has_two_stages() {
        assert_eq!(
            resolve_item(
                7,
                0x10,
                |id, flags| id == 7 && flags == 0x10,
                |_, _| Some(4)
            ),
            Ok(4)
        );
        assert_eq!(
            resolve_item(7, 0, |_, _| false, |_, _| Some(4)),
            Err("Unable to resolve item: 00000007".to_string())
        );
        assert_eq!(
            resolve_item(7, 0, |_, _| true, |_, _| None),
            Err("Unable to resolve item: 00000007".to_string())
        );
    }

    #[test]
    fn the_socket_row_takes_only_crystals_and_hands_old_ones_back() {
        assert!(Sockets::accepts_id(crystal::A));
        assert!(Sockets::accepts_id(crystal::AR));
        assert!(!Sockets::accepts_id(crystal::AL));
        assert!(!Sockets::accepts_id(7));
        let mut s = Sockets([0x80 | 2 | 0x4000, 0x80, 0, 0]);
        let r = s.drop_from(0, crystal::C, 0x0030_0000, false);
        assert_eq!(r.back, Some((crystal::B, 0x0040_0000)));
        assert_eq!(r.code, DROP_HANDLED);
        assert_eq!(s.kind(0), 4);
        assert_eq!(s.charge(0), 0x30);
        assert_eq!(s.drop_from(0, crystal::A, 0, true).code, 0);
        assert_eq!(s.card_at(0), Some((crystal::C, 0x0030_0000)));
        assert_eq!(s.start_drag(0), Some((crystal::C, 0x0030_0000)));
        s.drag_ended(0);
        // The kind goes; the charge byte stays behind the bare mark.
        assert_eq!(s.0[0], 0x3080);
        assert_eq!(s.card_at(0), None);
        let s = Sockets([0x80 | 0x11, 0, 0, 0]);
        assert_eq!(
            s.commit(true, true),
            Some(SocketCommit {
                target: CommitTarget::Weapon,
                ar: true
            })
        );
        assert_eq!(
            s.commit(false, true).map(|c| c.target),
            Some(CommitTarget::Apparel)
        );
        assert_eq!(s.commit(false, false), None);
        assert!(!Sockets::default().commit(false, true).unwrap().ar);
    }

    #[test]
    fn a_weapon_card_switches_its_striking_side() {
        // The shown rows fill from the first: slash, then pierce.
        assert_eq!(
            side_rows([[10, 0, 40], [0, 20, 0]], 0),
            vec![(0, 0.5), (2, 2.0)]
        );
        // 9 × 0.05 in double precision narrows to a different single than
        // the single-precision product.
        let nine = side_rows([[9, 0, 0], [0; 3]], 0)[0].1;
        assert_eq!(nine, (9.0f64 * 0.05) as f32);
        assert_ne!(nine, 9.0f32 * 0.05);
        assert_eq!(card_side(0x40), 1);
        assert_eq!(card_side(0x10), 0);
        assert_eq!(card_side(0x60), 1);
        assert_eq!(manager_cell(IconManager::Coins, 4), 16);
        assert_eq!(manager_cell(IconManager::Grip, 3), 24);
        assert_eq!(manager_cell(IconManager::Sockets, 4), 22);
        assert_eq!(SPRITE_SHEETS[0].cell, 64);
        assert_eq!(SPRITE_SHEETS[3].cells, 6);
    }

    #[test]
    fn payloads_and_placement() {
        let p = item_payload(3, 0x10, 160).unwrap();
        assert_eq!((p.kind, p.grab), (ITEM_PAYLOAD, (80, 80)));
        assert_eq!(item_payload(0, 0, 160), None);
        let q = item_payload_for_id(431, 0x1234_0000, 160, |id, flags| {
            (id == 431 && flags == 0x234_0000).then_some(9)
        })
        .unwrap();
        assert_eq!((q.icon, q.flags, q.grab), (9, 0x1234_0000, (80, 80)));
        assert!(item_payload_for_id(431, 0, 160, |_, _| None).is_none());
        let mut released = Vec::new();
        assert!(p.cancel(|icon| released.push(icon)));
        assert_eq!(released, vec![3]);
        assert!(!object_payload(0).cancel(|_| {}));
        assert_eq!(object_payload(0x0012_0000).kind, OBJECT_PAYLOAD);
        assert_eq!(drag_icon_at((100, 90), (80, 80), None), Some((20, 10)));
        assert_eq!(drag_icon_at((100, 90), (80, 80), Some(0x10)), None);
        assert_eq!(popup_place((50, 300), (268, 200), 1920, 4), (0, 132));
        assert_eq!(popup_place((1900, 100), (268, 200), 1920, 4), (1652, 4));
        assert_eq!(
            side_rows([[10, 0, 40], [0, 20, 0]], 0),
            vec![(0, 0.5), (2, 2.0)]
        );
        assert_eq!(side_rows([[10, 0, 40], [0, 20, 0]], 1), vec![(1, 1.0)]);
    }

    #[test]
    fn the_framing_of_each_variant() {
        use crate::icon_cache::variant as v;
        assert_eq!(framing(0, false).distance, 70.0);
        assert_eq!(framing(v::APPAREL, false).distance, 30.0);
        assert_eq!(framing(v::APPAREL, true).distance, 20.0);
        assert_eq!(framing(v::WEAPON, false).sun, (-0.16, 0.6516));
        assert!(framing(v::WEAPON | v::APPAREL, false).turned);
        assert!(!framing(v::WEAPON, false).turned);
        assert!(framing(v::HIGHLIGHT, false).highlight);
    }

    #[test]
    fn the_halo_spreads_alpha_by_one_pixel() {
        let (w, h) = (4, 4);
        let mut img = vec![0u8; w * h * 4];
        img[(w + 1) * 4 + 3] = 100;
        img[(w + 1) * 4 + 1] = 10;
        img[(w + 2) * 4 + 3] = 200;
        let mut lit = img.clone();
        halo(&mut img, w, h, false);
        let alpha = |p: &[u8], x: usize, y: usize| p[(y * w + x) * 4 + 3];
        assert_eq!(alpha(&img, 1, 1), 255);
        assert_eq!(alpha(&img, 2, 1), 255);
        assert_eq!(alpha(&img, 1, 2), 100);
        assert_eq!(alpha(&img, 2, 2), 200);
        // The border keeps its alpha.
        assert_eq!(alpha(&img, 0, 1), 0);
        halo(&mut lit, w, h, true);
        // Green rises by what alpha gained: 10 + (255 − 100).
        assert_eq!(lit[(w + 1) * 4 + 1], 165);
        assert_eq!(lit[(2 * w + 1) * 4 + 1], 100);
    }

    #[test]
    fn a_panel_filled_from_ids() {
        let (entries, used) = fill_from_ids(&[1, 2, 3], 0x400, (236, 628), 160, |id| {
            (id != 2).then_some(id * 10)
        });
        assert_eq!(entries, vec![(1, 0x400, 38, 234), (3, 0x400, 38, 234)]);
        assert_eq!(used, 40);
        assert_eq!(SPRITES[3], ("crystalicons.rfi", 6));
    }
}
