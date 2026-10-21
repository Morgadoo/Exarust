//! The search: a companion's idle activity (activity 6). It gathers the
//! containers, loose items and corpses of the nearby rooms, walks to each,
//! looks it over, kneels, and takes what beats its gear, or tells its
//! leader about it; then it re-equips and leaves behind what it no longer
//! needs.
//!
//! The tick, by state:
//!
//! 0. **Stand** (the wait runs). Once the anchor (the leader's place, or its
//!    own) is 100 cm from where it last gathered, it gathers again → 1.
//! 1. **Pick** (the wait runs). The next candidate within 1100 cm of the
//!    anchor and reachable, of lowest `(1 − d²/1100²) · e²` (d from the
//!    anchor, e from its own root), is reserved, its arrival radius is the
//!    mean half-extent plus 250 cm, the look-at goal is set and a 8000 ms
//!    timer starts → 2; none left → 0, not busy.
//! 2. **Look.** While its attention is on the goal, the head sweeps (90° to
//!    one side, then the other, then the body turns 162°) until the goal is
//!    within 0.4 rad of the gaze or the timer runs out → 3. Attention on
//!    someone else attends to them and restarts the timer; on no one, the
//!    search lets the object go → 1.
//! 3. **Path** (still sweeping) to 100 cm under the object, stop 50 cm,
//!    mode 7, forced → 4 when planned.
//! 4. **Approach.** Paused while its own pack is open in the player's
//!    window. On arrival (vertical distance halved), a container is looted
//!    from its contents, a loose item is taken unless already searched, a
//!    dead character's worn items then its pack; it settles and waits
//!    1000 + draw(1000) ms → 5. Not there after the walk: three tries.
//! 5. **Kneel.** Once the timer passes → 6 (a distraction renews 2000 ms).
//! 6. **Loot.** Each 1000 + draw(500) ms one item is weighed. When the
//!    source is exhausted the object (a corpse's pack tagged `0x20000000`) is
//!    remembered, released → 1; a container is first tidied up, a loose
//!    item taken last.
//!
//! Gathering floods the 600 cm room grid from the anchor's cell through the
//! four neighbours that are inside the grid's border, flagged connected and
//! within √10 cells, collecting each cell's containers and loose items
//! (searchable, not fixed, not searched, with an item record that has an
//! apparel, weapon, operative or container part) and dead characters (not
//! searched). A character of the cell whose object is container-like is
//! weighed as such an object instead.
//!
//! Weighing an item: apparel is worth `min((s + 1) + (m + 1), over the
//! carried and worn pieces it overlaps of (m + 1)/(m' + 1) + (s + 1)/(s' + 1))
//! / 2` (s the weighted coverage sum, m its largest bone); a weapon is worth
//! `(v + 1)` over the best `(v' + 1)` of its class carried, when that best is
//! above 1. With a leader in sight (attention 50 or more, alive, holding the
//! Player role) the search comments instead of taking a special piece, or one
//! the leader gains more from: `(own − 1)/2 < theirs − 1` and theirs above
//! `max(0.55 − bulk/20, 0) + 0.9` (apparel, theirs above 1) or above
//! `(1.1 − v/10)² + 0.8` (weapons). Otherwise apparel worth more than 1 is
//! worn when it adds more than 15 % of its bulk over what it covers, else
//! carried; a weapon (not of the `0x06000000` kinds) worth more than 1.05 is
//! carried, and the best weapons are taken in hand again.
//!
//! The world edits the tick returns as deeds are carried out by
//! [`apply_deed`]: taking a loose item despawns its object, taking from a
//! container removes its first matching entry, taking a corpse's worn item
//! removes it from the worn list and from a hand that held it, and taking a
//! carried entry drops that entry. Carrying, wearing, wielding and stowing
//! change the pack; dropping appends to a container while its known
//! capacity has room. Comments change no state.
//!
//! Evidence: the original's search activity (its tick, gathering, picking,
//! look sweep, item weighing, comments, re-equipping and tidying), the item
//! value routines it calls, and its unit's registration and phrase tables.
//! The phrase wording is Exarust's own: the counts, placeholders and
//! no-repeat rule are the original's.

use std::collections::BTreeMap;

use bevy::math::Vec3;

/// The anchor must move this far (cm), squared, before it gathers again.
pub const REGATHER_CM2: f32 = 10_000.0;
/// Candidates and the held object must be within 1100 cm of the anchor.
pub const REACH_CM2: f32 = 1.21e6;
/// How long the look-over may take, ms.
pub const LOOK_MS: u32 = 8000;
/// The arrival radius is the mean half-extent (× 0.333) plus this, cm.
pub const ARRIVAL_CM: f32 = 250.0;
/// The path goal is this far under the object, cm.
pub const GOAL_DROP_CM: f32 = 100.0;
pub const GOAL_STOP_CM: i32 = 50;
pub const GOAL_RADIUS_CM: f32 = 100.0;
pub const GOAL_MODE: i32 = 7;
/// Walks that end short of the object before it gives up.
pub const TRIES: i32 = 3;
/// The room grid's cell is 600 cm: a place is scaled by this and floored.
pub const PER_ROOM_CM: f32 = 0.001_666_666_7;
/// Cells further than this (squared, in cells) are not flooded: < 11.
const FLOOD_CELLS2: i32 = 11;
/// The connected-cell flag of the room grid.
pub const CONNECTED: u32 = 0x8_0000;
/// A searched corpse is remembered by its pack id with this bit.
pub const CORPSE_TAG: u32 = 0x2000_0000;
/// Carried entries with this flag are skipped.
pub const SKIP_FLAG: u32 = 0x100;
/// The attend levels the tick asks for.
pub const ATTEND: f32 = 1.0;
pub const ATTEND_WALKING: f32 = 5.1;

/// The four flood neighbours, (dx, dy).
const NEIGHBOURS: [(i32, i32); 4] = [(0, -1), (-1, 0), (0, 1), (1, 0)];

/// The object kinds the search reads.
pub mod object {
    /// The kind bits a container-like object has (with `KIND_MASK`).
    pub const KIND_MASK: u32 = 0x7000;
    pub const CONTAINER_KIND: u32 = 0x4000;
    /// The object is in the scene.
    pub const PRESENT: u32 = 0x4_0000;
}

/// Where the tick is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Stand,
    Pick,
    Look,
    Path,
    Approach,
    Kneel,
    Loot,
}

/// What is being looted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A loose item: the object itself.
    Item,
    /// A container's contents.
    Container,
    /// A dead character's worn items.
    Worn,
    /// A dead character's pack.
    Carried,
}

/// An object of the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObject {
    /// Its item id (the level's or the world's object database).
    pub id: u32,
    pub kind: u32,
    /// Its class lets it be searched.
    pub searchable: bool,
    /// Marked as fixed: never gathered.
    pub fixed: bool,
    pub at: Vec3,
    pub half: Vec3,
    /// How many searches hold it.
    pub reserved: i32,
    /// The dead character it belongs to.
    pub corpse: Option<usize>,
}

/// One carried entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carried {
    pub item: u32,
    pub flags: u32,
}

/// What a character carries and wears.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pack {
    pub carried: Vec<Carried>,
    pub worn: Vec<u32>,
    /// The hand slots: main, second, and the two stowed.
    pub hands: [u32; 4],
    /// The pack is locked by another edit.
    pub locked: bool,
}

/// A character of the scene, as the search sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Corpse {
    /// It is a character at all.
    pub character: bool,
    pub dead: bool,
    /// Its pack's id (the character id).
    pub pack_id: u32,
    pub pack: Pack,
    /// Its object in the scene.
    pub object: usize,
}

/// A container's contents.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Container {
    pub items: Vec<u32>,
    /// The item records' capacity the container is limited by. Zero is
    /// treated as unlimited (the shipped records' own values are not read
    /// into the scene yet).
    pub capacity: u32,
}

/// The 600 cm room grid.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RoomGrid {
    pub width: i32,
    pub height: i32,
    /// The cell of the world origin.
    pub origin: (i32, i32),
    pub flags: Vec<u32>,
    /// Characters in each cell (indices into [`Scene::corpses`]).
    pub characters: Vec<Vec<usize>>,
    /// Objects in each cell (indices into [`Scene::objects`]).
    pub objects: Vec<Vec<usize>>,
}

/// What the search knows of an item.
#[derive(Debug, Clone, PartialEq)]
pub enum Loot {
    Apparel(Box<Apparel>),
    Weapon(Arm),
    /// An item with an operative or container part only.
    Other,
}

/// Apparel: coverage per bone (1 to 20), the four protection bytes, the
/// occupancy table, the special byte, and a model byte the wear room reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Apparel {
    pub coverage: [u32; 20],
    pub protection: [u8; 4],
    pub occupancy: [u64; 20],
    pub special: u8,
    /// The model byte above which a worn piece leaves no room [?].
    pub model_mark: u8,
}

/// A weapon: its flags, quality, damage, thrust byte, tier (low 5 bits of
/// the last stat byte) and special word.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arm {
    pub flags: u32,
    pub quality: u8,
    pub damage: u8,
    pub thrust: u8,
    pub tier: u8,
    pub special: u32,
}

/// The scene the search works in.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
    pub corpses: Vec<Corpse>,
    pub grid: RoomGrid,
    pub items: BTreeMap<u32, Loot>,
    pub containers: BTreeMap<u32, Container>,
    /// The packs and containers open in the player's window.
    pub open: Vec<u32>,
}

/// Bone weights of the apparel value, bones 1 to 20.
const BONE_WEIGHT: [f32; 20] = [
    1.0, 1.0, 1.0, 0.5, 1.0, 1.0, 0.5, 2.0, 2.5, 4.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 1.0,
    1.0,
];

impl Apparel {
    fn protection_sum(&self) -> f32 {
        self.protection.iter().map(|&b| u32::from(b)).sum::<u32>() as f32
    }

    /// The weighted coverage sum and the largest bone value.
    pub fn coverage(&self) -> (f32, f32) {
        let p = self.protection_sum();
        let (mut sum, mut most) = (0.0f32, 0.0f32);
        for (c, w) in self.coverage.iter().zip(BONE_WEIGHT) {
            if *c == 0 {
                continue;
            }
            let v = (f64::from(*c) * 0.01 * 0.005) as f32 * p;
            sum += v * w;
            if most < v {
                most = v;
            }
        }
        (sum, most)
    }

    /// Its bulk: `Σ p · w · √(c/100) · 0.005` over the covered bones.
    pub fn bulk(&self) -> f32 {
        let p = self.protection_sum();
        let mut bulk = 0.0f32;
        for (c, w) in self.coverage.iter().zip(BONE_WEIGHT) {
            if *c != 0 {
                bulk += p * (f64::from(w) * (f64::from(*c) * 0.01).sqrt() * 0.005) as f32;
            }
        }
        bulk
    }

    /// Whether two pieces share any occupancy bit.
    pub fn overlaps(&self, other: &Apparel) -> bool {
        self.occupancy
            .iter()
            .zip(other.occupancy.iter())
            .any(|(a, b)| a & b != 0)
    }
}

impl Arm {
    /// Its value: −10 for tier 0, else `thrust/140 + tier · ((q/255)²/2 +
    /// 1/2) · (1 − (damage/255) · 0.3)`, × 0.8 for the `0x10` hand bits.
    pub fn value(&self) -> f32 {
        if self.tier & 0x1F == 0 {
            return -10.0;
        }
        let q = f32::from(self.quality) / 255.0;
        let worn = f32::from(self.damage) / 255.0;
        let v = f32::from(self.thrust) / 140.0
            + f32::from(self.tier & 0x1F) * (q * q * 0.5 + 0.5) * (1.0 - worn * 0.3);
        if self.flags & 0x30 == 0x10 {
            v * 0.8
        } else {
            v
        }
    }

    /// Its class, 0 to 3, by the hand bits and the low nibble.
    pub fn class(&self) -> usize {
        match (self.flags & 0x30 == 0x10, self.flags & 0xF == 1) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 3,
        }
    }
}

fn apparel(items: &BTreeMap<u32, Loot>, id: u32) -> Option<&Apparel> {
    match items.get(&id) {
        Some(Loot::Apparel(a)) => Some(a.as_ref()),
        _ => None,
    }
}

fn arm(items: &BTreeMap<u32, Loot>, id: u32) -> Option<&Arm> {
    match items.get(&id) {
        Some(Loot::Weapon(w)) => Some(w),
        _ => None,
    }
}

/// What apparel `id` is worth to `pack`.
pub fn apparel_worth(items: &BTreeMap<u32, Loot>, pack: &Pack, id: u32) -> f32 {
    let Some(piece) = apparel(items, id) else {
        return 0.0;
    };
    let (sum, most) = piece.coverage();
    let (sum, most) = (sum + 1.0, most + 1.0);
    let mut worth = most + sum;
    let others = pack
        .carried
        .iter()
        .map(|c| c.item)
        .chain(pack.worn.iter().copied());
    for other in others.filter_map(|o| apparel(items, o)) {
        if piece.overlaps(other) {
            let (s, m) = other.coverage();
            let v = most / (m + 1.0) + sum / (s + 1.0);
            if v < worth {
                worth = v;
            }
        }
    }
    worth * 0.5
}

/// What weapon `id` is worth to `pack`: its value plus 1 over the best of
/// its class carried, when that best is above 1.
pub fn weapon_worth(items: &BTreeMap<u32, Loot>, pack: &Pack, id: u32) -> f32 {
    let Some(w) = arm(items, id) else {
        return 0.0;
    };
    let mine = w.value() + 1.0;
    let mut best = [0.0f32; 4];
    for other in pack.carried.iter().filter_map(|c| arm(items, c.item)) {
        let v = other.value() + 1.0;
        let b = &mut best[other.class()];
        if *b < v {
            *b = v;
        }
    }
    let b = best[w.class()];
    if 1.0 < b { mine / b } else { mine }
}

/// The room apparel `id` would add when worn: its bulk less the bulk of
/// the worn pieces it overlaps, or 0 when one of them has a model byte
/// above 50.
pub fn wear_room(items: &BTreeMap<u32, Loot>, pack: &Pack, piece: &Apparel) -> f32 {
    let mut room = piece.bulk();
    for worn in pack.worn.iter().filter_map(|w| apparel(items, *w)) {
        if worn.overlaps(piece) {
            if 50 < worn.model_mark {
                return 0.0;
            }
            room -= worn.bulk();
        }
    }
    room
}

/// What the search does to the world, to be carried out by the caller.
#[derive(Debug, Clone, PartialEq)]
pub enum Deed {
    /// Take `item` out of `source` (of object `object`).
    Take {
        object: usize,
        source: Source,
        item: u32,
    },
    /// Carry `item` (after taking it).
    Carry { item: u32 },
    /// Put `item` on (after taking it out of its source).
    Wear {
        object: usize,
        source: Source,
        item: u32,
    },
    /// Take `item` in the main hand.
    Wield { item: u32 },
    /// Stow `item` as the second weapon.
    Stow { item: u32 },
    /// Leave `item` in the container `container`.
    Drop { item: u32, container: u32 },
    /// Say `phrase` to the leader.
    Comment { phrase: String },
}

/// Who the attention is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attention {
    /// The search's look-at goal.
    Goal,
    Nobody,
    /// Someone else.
    Other,
}

/// A path request: the goal, its stop and radius, and the motion mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goal {
    pub at: Vec3,
    pub stop_cm: i32,
    pub radius_cm: f32,
    pub mode: i32,
}

/// The head and body headings the sweep turns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaze {
    /// The body's yaw (read).
    pub body_yaw: f32,
    /// The head's look direction (read).
    pub head: Vec3,
    /// The look yaw the head turns toward.
    pub look_yaw: f32,
    pub look_pitch: f32,
    /// The yaw the body turns to.
    pub turn_to: f32,
    /// Until when the body keeps looking, ms.
    pub look_until: u32,
}

/// The searching character: its clock, generator, places and the parts of
/// its brain the search calls.
pub trait Searcher {
    fn now_ms(&self) -> u32;
    /// The engine's generator.
    fn roll(&mut self) -> &mut u32;
    /// Its root, the point candidates are scored from.
    fn root(&self) -> Vec3;
    /// Its centre, the point arrival and looking are measured from.
    fn centre(&self) -> Vec3;
    /// Whether `to` can be reached from `from`.
    fn reachable(&self, from: Vec3, to: Vec3) -> bool;
    /// The alert look-around; true while it takes the turn.
    fn alert_look(&mut self) -> bool;
    /// Attends to a contact at `level`; true while it does.
    fn attend(&mut self, level: f32) -> bool;
    /// Runs the wait idle state.
    fn wait(&mut self);
    /// Stands and settles its look.
    fn settle(&mut self);
    /// The idle look.
    fn idle_look(&mut self);
    fn attention(&self) -> Attention;
    /// Plans a forced path; true when planned.
    fn plan(&mut self, goal: Goal) -> bool;
    fn walking(&self) -> bool;
    /// Drops the walk's urgency to 0.
    fn calm_walk(&mut self);
    /// Clears the path mark the roam also clears [?].
    fn clear_path_mark(&mut self);
    fn gaze(&mut self) -> &mut Gaze;
    /// Its own pack and its id.
    fn pack(&mut self) -> &mut Pack;
    fn pack_id(&self) -> u32;
    /// The searched-objects list it keeps.
    fn searched(&mut self) -> &mut Vec<u32>;
    /// Game time in seconds, for the comment window.
    fn now_s(&self) -> u32;
}

/// One contact, for finding the leader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub attention: f32,
    pub character: bool,
    pub dead: bool,
    pub player: bool,
}

/// The first contact with attention 50 or more that is a living character
/// holding the Player role.
pub fn find_leader(contacts: &[Contact]) -> Option<usize> {
    contacts
        .iter()
        .position(|c| 50.0 <= c.attention && c.character && !c.dead && c.player)
}

/// A phrase table: templates with one `[a]` or `[i]` placeholder, how many
/// of them are drawn from, and the last two drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct PhraseTable {
    pub templates: Vec<String>,
    pub drawn_from: u32,
    pub last: [u32; 2],
}

/// The four tables: first and repeated comments, one item or a pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Phrases {
    pub first_one: PhraseTable,
    pub first_pair: PhraseTable,
    pub more_one: PhraseTable,
    pub more_pair: PhraseTable,
}

fn table(templates: &[&str], drawn_from: u32, last: u32) -> PhraseTable {
    PhraseTable {
        templates: templates.iter().map(|t| (*t).into()).collect(),
        drawn_from,
        last: [last, last],
    }
}

impl Default for Phrases {
    /// Exarust's wording, in the original's shape: six templates for a
    /// first comment of which five are drawn, four for a further comment,
    /// the last-drawn marks starting outside the tables.
    fn default() -> Self {
        Self {
            first_one: table(
                &[
                    "Look, [a].",
                    "There is [a] over here.",
                    "Here lies [a].",
                    "I have [a] here.",
                    "[a], should you want it.",
                    "This [i] might serve.",
                ],
                5,
                8,
            ),
            first_pair: table(
                &[
                    "Look, some [i].",
                    "There are some [i] over here.",
                    "Here lie some [i].",
                    "I have some [i] here.",
                    "Some [i], should you want them.",
                    "These [i] might serve.",
                ],
                5,
                8,
            ),
            more_one: table(
                &[
                    "And [a].",
                    "[a] as well.",
                    "There is also [a].",
                    "Also [a] here.",
                ],
                4,
                5,
            ),
            more_pair: table(
                &[
                    "And some [i].",
                    "Some [i] as well.",
                    "There are also some [i].",
                    "Also some [i] here.",
                ],
                4,
                5,
            ),
        }
    }
}

fn draw(roll: &mut u32, n: u32) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*roll) * u64::from(n)) >> 32) as u32
}

/// The engine's lower-casing as the comment calls it: ASCII capitals
/// become small, all but the last character, which its loop never reaches.
pub fn lower_but_last(name: &str) -> String {
    let n = name.chars().count();
    name.chars()
        .enumerate()
        .map(|(i, c)| if i + 1 < n { c.to_ascii_lowercase() } else { c })
        .collect()
}

fn vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// The last item names commented on, and when.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Remarks {
    pub at_s: u32,
    pub names: Vec<String>,
}

/// Builds a comment about an item named `name`. Within 10 s of the last
/// comment the names said are remembered (three at most) and a further
/// comment uses the further tables; a name said again is not remembered
/// again. A name with "pair of" uses the pair tables. The draw rerolls the
/// table's last two, and the pair's partner table remembers the same draw.
/// `[a]` becomes the name with "a " or "an " (by its first letter), `[i]`
/// the name alone, and the phrase starts with a capital.
pub fn comment(
    phrases: &mut Phrases,
    remarks: &mut Remarks,
    name: &str,
    now_s: u32,
    roll: &mut u32,
) -> String {
    if u64::from(remarks.at_s) + 10 < u64::from(now_s) {
        remarks.names.clear();
    }
    remarks.at_s = now_s;
    let pair = name.contains("pair of");
    let lower = lower_but_last(name);
    if !remarks.names.contains(&lower) {
        if remarks.names.len() == 3 {
            remarks.names.pop();
        }
        remarks.names.insert(0, lower.clone());
    }
    let first = remarks.names.len() < 2;
    let (used, partner) = match (first, pair) {
        (true, false) => (&mut phrases.first_one, &mut phrases.first_pair),
        (true, true) => (&mut phrases.first_pair, &mut phrases.first_one),
        (false, false) => (&mut phrases.more_one, &mut phrases.more_pair),
        (false, true) => (&mut phrases.more_pair, &mut phrases.more_one),
    };
    let mut pick = draw(roll, used.drawn_from);
    while pick == used.last[0] || pick == used.last[1] {
        pick = draw(roll, used.drawn_from);
    }
    used.last = [pick, used.last[0]];
    partner.last = [pick, partner.last[0]];
    let template = used
        .templates
        .get(pick as usize)
        .cloned()
        .unwrap_or_default();
    let Some(at) = template.find('[') else {
        return template;
    };
    let mut item = lower;
    if template[at + 1..].starts_with('a') {
        let article = if item.chars().next().is_some_and(vowel) {
            "an "
        } else {
            "a "
        };
        item.insert_str(0, article);
    }
    let rest = template.get(at + 3..).unwrap_or("");
    let mut phrase = format!("{}{item}{rest}", &template[..at]);
    if let Some(c) = phrase.chars().next()
        && c.is_ascii_lowercase()
    {
        phrase.replace_range(..1, &c.to_ascii_uppercase().to_string());
    }
    phrase
}

/// The head sweep while looking an object over.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sweep {
    /// 0 unset, 1 and 2 the first side, 3 and 4 the other, 5 and 6 turning
    /// the body, 7 seen.
    pub phase: u8,
    pub heading: f32,
}

const HALF_PI: f32 = 1.570_796_4;
const TURN: f32 = 2.827_433_3;
const TAU: f32 = 6.283_185_5;

fn wrap(a: f32) -> f32 {
    if a < -std::f32::consts::PI {
        a + TAU
    } else if std::f32::consts::PI < a {
        a - TAU
    } else {
        a
    }
}

/// The yaw of a direction (x, z).
pub fn yaw_of(v: Vec3) -> f32 {
    v.x.atan2(v.z)
}

/// The engine's quick arcsine.
pub fn quick_asin(x: f32) -> f32 {
    if x < 0.0 {
        ((0.0535 * x + 0.2075) * x + HALF_PI) * (1.0 + x).sqrt() - HALF_PI
    } else {
        HALF_PI - (HALF_PI - (0.2075 - 0.0535 * x) * x) * (1.0 - x).sqrt()
    }
}

/// One step of the sweep. True once `target` (when there is one) is within
/// 0.4 rad of the head's look, which also stops the timer at `now`.
pub fn sweep(
    sweep: &mut Sweep,
    gaze: &mut Gaze,
    centre: Vec3,
    target: Option<Vec3>,
    now: u32,
    timer: &mut u32,
    roll: &mut u32,
) -> bool {
    if sweep.phase == 0 {
        *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
        let right = (u64::from(*roll) << 1) >> 32 == 0;
        if right {
            sweep.heading = wrap(gaze.body_yaw - HALF_PI);
            sweep.phase = 1;
        } else {
            sweep.heading = wrap(gaze.body_yaw + HALF_PI);
            sweep.phase = 2;
        }
        gaze.look_pitch = -quick_asin(gaze.head.y);
        gaze.look_yaw = yaw_of(gaze.head);
    }
    gaze.look_pitch *= 0.99;
    let off = wrap(sweep.heading - gaze.look_yaw);
    if 0.1 <= off.abs() || 4 < sweep.phase {
        if sweep.phase == 5 {
            sweep.heading = wrap(gaze.body_yaw + HALF_PI);
        } else if sweep.phase == 6 {
            sweep.heading = wrap(gaze.body_yaw - HALF_PI);
        }
        let sign = if off == 0.0 { 0.0 } else { off.signum() };
        gaze.look_yaw = wrap(gaze.look_yaw + sign * 0.001 + off * 0.015);
    } else {
        match sweep.phase {
            1 => {
                sweep.heading = wrap(gaze.body_yaw + HALF_PI);
                sweep.phase = 3;
            }
            2 => {
                sweep.phase = 4;
                sweep.heading = wrap(gaze.body_yaw - HALF_PI);
            }
            3 => {
                gaze.turn_to = wrap(gaze.body_yaw + TURN);
                sweep.phase = 5;
            }
            4 => {
                gaze.turn_to = wrap(gaze.body_yaw - TURN);
                sweep.phase = 6;
            }
            _ => {}
        }
    }
    let Some(target) = target else {
        return false;
    };
    let toward = yaw_of(target - centre);
    let seen = wrap(yaw_of(gaze.head) - toward);
    if seen.abs() < 0.4 {
        *timer = now;
        gaze.look_until = now;
        sweep.heading = toward;
        gaze.turn_to = toward;
        if sweep.phase < 7 {
            sweep.phase = 7;
        }
        return true;
    }
    false
}

/// The search's own state.
#[derive(Debug, Clone, PartialEq)]
pub struct Search {
    pub state: State,
    pub timer_ms: u32,
    /// Where it last gathered.
    pub origin: Vec3,
    pub candidates: Vec<usize>,
    /// How many candidates were picked.
    pub picked: usize,
    /// The object it holds.
    pub target: Option<usize>,
    pub arrival_cm: f32,
    pub tries: i32,
    pub source: Source,
    /// The corpse's pack, tagged.
    pub corpse_pack: u32,
    pub index: usize,
    pub sweep: Sweep,
    pub remarks: Remarks,
    /// The searching flag of the brain.
    pub busy: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl Search {
    /// A new search: standing, with the lookahead of 4 the original sets
    /// (unused by its tick).
    pub fn new() -> Self {
        Self {
            state: State::Stand,
            timer_ms: 0,
            origin: Vec3::ZERO,
            candidates: Vec::new(),
            picked: 0,
            target: None,
            arrival_cm: 0.0,
            tries: 0,
            source: Source::Item,
            corpse_pack: 0,
            index: 0,
            sweep: Sweep::default(),
            remarks: Remarks::default(),
            busy: false,
        }
    }

    /// Resets: standing, no candidates.
    pub fn reset(&mut self) {
        self.state = State::Stand;
        self.candidates.clear();
        self.picked = 0;
    }

    /// Stops: the origin goes back to the world origin, the state to
    /// standing, and the object held is let go.
    pub fn stop(&mut self, scene: &mut Scene) {
        self.origin = Vec3::ZERO;
        self.state = State::Stand;
        self.busy = false;
        self.release(scene);
    }

    /// Restarts from the world origin.
    pub fn restart(&mut self) {
        self.origin = Vec3::ZERO;
        self.reset();
    }

    fn release(&mut self, scene: &mut Scene) {
        if let Some(t) = self.target.take()
            && let Some(o) = scene.objects.get_mut(t)
        {
            o.reserved -= 1;
        }
    }

    /// Lets the object go and picks again.
    fn give_up(&mut self, scene: &mut Scene) {
        self.busy = false;
        self.release(scene);
        self.state = State::Pick;
    }

    /// Floods the room grid from the anchor and gathers the candidates.
    pub fn gather(&mut self, scene: &mut Scene, searched: &[u32]) {
        self.candidates.clear();
        self.picked = 0;
        let g = &scene.grid;
        let cx = (self.origin.x * PER_ROOM_CM).floor() as i32 + g.origin.0;
        let cy = (self.origin.z * PER_ROOM_CM).floor() as i32 + g.origin.1;
        let mut seen = [[false; 7]; 7];
        seen[3][3] = true;
        let mut stack = vec![(cx, cy)];
        let mut found = Vec::new();
        while let Some((x, y)) = stack.pop() {
            if let Some(cell) = g.index(x, y) {
                for &c in g.characters.get(cell).into_iter().flatten() {
                    found.push(Found::Character(c));
                }
                for &o in g.objects.get(cell).into_iter().flatten() {
                    found.push(Found::Object(o));
                }
            }
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx <= 0 || g.width - 1 <= nx || ny <= 0 || g.height - 1 <= ny {
                    continue;
                }
                let (ox, oy) = (nx - cx, ny - cy);
                if FLOOD_CELLS2 <= ox * ox + oy * oy {
                    continue;
                }
                let (sx, sy) = ((ox + 3) as usize, (oy + 3) as usize);
                if seen[sx][sy] {
                    continue;
                }
                let flagged = g
                    .index(nx, ny)
                    .and_then(|i| g.flags.get(i))
                    .is_some_and(|f| f & CONNECTED != 0);
                if flagged {
                    seen[sx][sy] = true;
                    stack.push((nx, ny));
                }
            }
        }
        for f in found {
            match f {
                // A character whose object is container-like is weighed as
                // an object, before the dead-character test.
                Found::Character(c) => match scene.corpses.get(c).map(|k| k.object) {
                    Some(o)
                        if scene.objects.get(o).is_some_and(|obj| {
                            obj.kind & object::KIND_MASK == object::CONTAINER_KIND
                        }) =>
                    {
                        self.consider_object(scene, o, searched)
                    }
                    _ => self.consider_corpse(scene, c, searched),
                },
                Found::Object(o) => self.consider_object(scene, o, searched),
            }
        }
    }

    fn consider_object(&mut self, scene: &mut Scene, o: usize, searched: &[u32]) {
        let Some(obj) = scene.objects.get(o) else {
            return;
        };
        if obj.kind & object::KIND_MASK != object::CONTAINER_KIND || !obj.searchable {
            return;
        }
        if self.candidates.contains(&o) || obj.fixed || searched.contains(&obj.id) {
            return;
        }
        let useful = scene.items.contains_key(&obj.id) || scene.containers.contains_key(&obj.id);
        if useful {
            scene.objects[o].reserved += 1;
            self.candidates.push(o);
        }
    }

    fn consider_corpse(&mut self, scene: &mut Scene, c: usize, searched: &[u32]) {
        let Some(corpse) = scene.corpses.get(c) else {
            return;
        };
        if !(corpse.character && corpse.dead) {
            return;
        }
        let o = corpse.object;
        if self.candidates.contains(&o) || searched.contains(&(corpse.pack_id | CORPSE_TAG)) {
            return;
        }
        if let Some(obj) = scene.objects.get_mut(o) {
            obj.reserved += 1;
            self.candidates.push(o);
        }
    }

    /// Picks the next candidate, moving it to the front of the unpicked
    /// ones.
    fn pick(&mut self, scene: &Scene, me: &dyn Searcher, anchor: Vec3) -> Option<usize> {
        let root = me.root();
        let mut best = None;
        let mut score = REACH_CM2;
        for i in self.picked..self.candidates.len() {
            let Some(obj) = scene.objects.get(self.candidates[i]) else {
                continue;
            };
            let d2 = anchor.distance_squared(obj.at);
            if REACH_CM2 < d2 || !me.reachable(anchor, obj.at) {
                continue;
            }
            let s = (1.0 - d2 / REACH_CM2) * root.distance_squared(obj.at);
            if s < score {
                best = Some(i);
                score = s;
            }
        }
        let i = best?;
        let picked = self.candidates[i];
        self.candidates.swap(i, self.picked);
        self.picked += 1;
        Some(picked)
    }

    /// One tick. Returns whether the search is busy, and what it does to
    /// the world.
    pub fn tick(
        &mut self,
        me: &mut dyn Searcher,
        scene: &mut Scene,
        phrases: &mut Phrases,
        anchor: Vec3,
        leader: Option<&Pack>,
        names: &dyn Fn(u32) -> String,
    ) -> (bool, Vec<Deed>) {
        let mut deeds = Vec::new();
        if me.alert_look() {
            return (false, deeds);
        }
        let mut busy = true;
        if self.state as u8 > State::Pick as u8 {
            let keep = self
                .target
                .and_then(|t| scene.objects.get(t))
                .is_some_and(|o| {
                    o.kind & object::PRESENT != 0
                        && anchor.distance_squared(o.at) <= REACH_CM2
                        && me.reachable(anchor, o.at)
                });
            if !keep {
                self.state = State::Pick;
            }
        }
        let now = me.now_ms();
        match self.state {
            State::Stand => {
                me.wait();
                if REGATHER_CM2 <= self.origin.distance_squared(anchor) {
                    self.origin = anchor;
                    let searched = me.searched().clone();
                    self.gather(scene, &searched);
                    self.state = State::Pick;
                } else {
                    busy = false;
                }
            }
            State::Pick => {
                me.wait();
                let picked = if self.picked < self.candidates.len() {
                    self.pick(scene, me, anchor)
                } else {
                    None
                };
                if let Some(picked) = picked {
                    // The object held before is replaced without being let go.
                    self.target = Some(picked);
                    self.state = State::Look;
                    self.timer_ms = now.wrapping_add(LOOK_MS);
                    let half = self
                        .target
                        .and_then(|t| scene.objects.get(t))
                        .map(|o| o.half);
                    let half = half.unwrap_or(Vec3::ZERO);
                    self.arrival_cm = (half.x + half.y + half.z) * 0.333 + ARRIVAL_CM;
                    self.busy = true;
                    me.gaze().look_until = self.timer_ms;
                    self.sweep.phase = 0;
                } else {
                    self.state = State::Stand;
                    busy = false;
                }
            }
            State::Look => match me.attention() {
                Attention::Goal => {
                    let timer = self.timer_ms;
                    me.gaze().look_until = timer;
                    if self.look(me, scene, now) || self.timer_ms <= now {
                        self.state = State::Path;
                        self.tries = 0;
                    }
                }
                Attention::Nobody => self.give_up(scene),
                Attention::Other => {
                    me.gaze().look_until = 0;
                    me.attend(ATTEND);
                    self.timer_ms = now.wrapping_add(LOOK_MS);
                    self.sweep.phase = 0;
                }
            },
            State::Path => {
                if me.attention() == Attention::Goal {
                    self.look(me, scene, now);
                    me.calm_walk();
                    let at = self.target.and_then(|t| scene.objects.get(t)).map(|o| o.at);
                    let at = at.unwrap_or(me.root());
                    let goal = Goal {
                        at: Vec3::new(at.x, at.y - GOAL_DROP_CM, at.z),
                        stop_cm: GOAL_STOP_CM,
                        radius_cm: GOAL_RADIUS_CM,
                        mode: GOAL_MODE,
                    };
                    if me.plan(goal) {
                        self.state = State::Approach;
                    }
                } else {
                    me.attend(ATTEND);
                }
            }
            State::Approach => self.approach(me, scene, now, &mut busy),
            State::Kneel => {
                if !me.attend(ATTEND) {
                    me.idle_look();
                }
                if me.attention() == Attention::Goal {
                    if self.timer_ms < now {
                        self.index = 0;
                        self.state = State::Loot;
                        me.clear_path_mark();
                    }
                } else {
                    self.timer_ms = now.wrapping_add(2000);
                }
            }
            State::Loot => {
                busy = self.loot(me, scene, phrases, leader, names, now, &mut deeds);
            }
        }
        (busy, deeds)
    }

    fn look(&mut self, me: &mut dyn Searcher, scene: &Scene, now: u32) -> bool {
        let target = self.target.and_then(|t| scene.objects.get(t)).map(|o| o.at);
        let centre = me.centre();
        let mut timer = self.timer_ms;
        let mut gaze = *me.gaze();
        let seen = sweep(
            &mut self.sweep,
            &mut gaze,
            centre,
            target,
            now,
            &mut timer,
            me.roll(),
        );
        *me.gaze() = gaze;
        self.timer_ms = timer;
        seen
    }

    fn approach(&mut self, me: &mut dyn Searcher, scene: &mut Scene, now: u32, busy: &mut bool) {
        let own = me.pack_id() | CORPSE_TAG;
        if scene.open.contains(&own) {
            *busy = false;
            return;
        }
        if me.attend(ATTEND_WALKING) {
            return;
        }
        me.calm_walk();
        if me.walking() {
            return;
        }
        let Some(t) = self.target else {
            return;
        };
        let Some(obj) = scene.objects.get(t).cloned() else {
            return;
        };
        let mut d = me.centre() - obj.at;
        d.y *= 0.5;
        if self.arrival_cm * self.arrival_cm < d.length_squared() {
            self.tries += 1;
            if self.tries < TRIES {
                self.state = State::Path;
            } else {
                self.give_up(scene);
            }
            return;
        }
        if obj.kind & object::KIND_MASK == object::CONTAINER_KIND {
            if scene.containers.contains_key(&obj.id) {
                self.source = Source::Container;
            } else {
                if me.searched().contains(&obj.id) {
                    self.give_up(scene);
                    return;
                }
                self.source = Source::Item;
            }
        } else {
            let corpse = obj.corpse.and_then(|c| scene.corpses.get(c));
            let Some(corpse) = corpse.filter(|c| c.character && c.dead) else {
                self.give_up(scene);
                return;
            };
            self.corpse_pack = corpse.pack_id | CORPSE_TAG;
            if scene.open.contains(&self.corpse_pack) {
                self.picked -= 1;
                self.state = State::Pick;
                return;
            }
            self.source = Source::Worn;
        }
        self.state = State::Kneel;
        me.settle();
        if !me.attend(ATTEND) {
            me.idle_look();
        }
        self.timer_ms = now.wrapping_add(1000).wrapping_add(draw(me.roll(), 1000));
    }

    #[allow(clippy::too_many_arguments)]
    fn loot(
        &mut self,
        me: &mut dyn Searcher,
        scene: &mut Scene,
        phrases: &mut Phrases,
        leader: Option<&Pack>,
        names: &dyn Fn(u32) -> String,
        now: u32,
        deeds: &mut Vec<Deed>,
    ) -> bool {
        if !me.attend(ATTEND) {
            me.idle_look();
        }
        if me.attention() != Attention::Goal {
            self.timer_ms = now.wrapping_add(2000);
            return true;
        }
        if now < self.timer_ms {
            return true;
        }
        if scene.open.contains(&(me.pack_id() | CORPSE_TAG)) {
            return false;
        }
        let Some(t) = self.target else {
            return true;
        };
        let Some(obj) = scene.objects.get(t).cloned() else {
            return true;
        };
        let corpse = obj.corpse.and_then(|c| scene.corpses.get(c)).cloned();
        let retry = |s: &mut Self| {
            s.picked -= 1;
            s.state = State::Pick;
        };
        let next = match self.source {
            Source::Worn => {
                if scene.open.contains(&self.corpse_pack) {
                    retry(self);
                    return true;
                }
                let item = corpse
                    .as_ref()
                    .and_then(|c| c.pack.worn.get(self.index).copied())
                    .unwrap_or(0);
                if item == 0 {
                    self.source = Source::Carried;
                    self.index = 0;
                    return true;
                }
                Some((Source::Worn, item))
            }
            Source::Carried => {
                if scene.open.contains(&self.corpse_pack) {
                    retry(self);
                    return true;
                }
                let entry = corpse
                    .as_ref()
                    .and_then(|c| c.pack.carried.get(self.index).copied());
                match entry.filter(|e| e.item != 0 || e.flags & SKIP_FLAG != 0) {
                    Some(e) if e.flags & SKIP_FLAG != 0 => {
                        self.index += 1;
                        return true;
                    }
                    Some(e) => Some((Source::Carried, e.item)),
                    None => None,
                }
            }
            Source::Container => {
                if scene.open.contains(&obj.id) {
                    retry(self);
                    return true;
                }
                let item = scene
                    .containers
                    .get(&obj.id)
                    .and_then(|c| c.items.get(self.index).copied())
                    .unwrap_or(0);
                if item == 0 {
                    deeds.extend(tidy(&scene.items, me.pack(), obj.id));
                    None
                } else {
                    Some((Source::Container, item))
                }
            }
            Source::Item => None,
        };
        if let Some((source, item)) = next {
            self.index += 1;
            let r = self.weigh(me, scene, phrases, leader, names, t, source, item, deeds);
            if 0 < r {
                self.timer_ms = now.wrapping_add(1000).wrapping_add(draw(me.roll(), 500));
                if r == 1 {
                    self.index -= 1;
                }
            }
            return true;
        }
        let tagged = if self.source == Source::Carried {
            obj.id | CORPSE_TAG
        } else {
            obj.id
        };
        me.searched().push(tagged);
        self.busy = false;
        self.state = State::Pick;
        if self.source == Source::Item {
            self.weigh(
                me,
                scene,
                phrases,
                leader,
                names,
                t,
                Source::Item,
                obj.id,
                deeds,
            );
        }
        if let Some(o) = scene.objects.get_mut(t) {
            o.reserved -= 1;
        }
        self.target = None;
        self.source = Source::Item;
        true
    }

    /// Weighs one item: 2 when it commented, 1 when it took the item, 0
    /// otherwise.
    #[allow(clippy::too_many_arguments)]
    pub fn weigh(
        &mut self,
        me: &mut dyn Searcher,
        scene: &Scene,
        phrases: &mut Phrases,
        leader: Option<&Pack>,
        names: &dyn Fn(u32) -> String,
        object: usize,
        source: Source,
        item: u32,
        deeds: &mut Vec<Deed>,
    ) -> u8 {
        let Some(loot) = scene.items.get(&item) else {
            return 0;
        };
        let mut say = |me: &mut dyn Searcher, deeds: &mut Vec<Deed>, remarks: &mut Remarks| {
            let now_s = me.now_s();
            let phrase = comment(phrases, remarks, &names(item), now_s, me.roll());
            deeds.push(Deed::Comment { phrase });
        };
        match loot {
            Loot::Apparel(piece) => {
                let own = apparel_worth(&scene.items, me.pack(), item);
                if let Some(theirs_pack) = leader {
                    if piece.special != 0 {
                        say(me, deeds, &mut self.remarks);
                        return 2;
                    }
                    let theirs = apparel_worth(&scene.items, theirs_pack, item);
                    if 1.0 < theirs && (own - 1.0) * 0.5 < theirs - 1.0 {
                        let b = piece.bulk();
                        let t = ((0.55 - f64::from(b) * 0.05) as f32).max(0.0);
                        if ((f64::from(t) + 0.9) as f32) < theirs {
                            say(me, deeds, &mut self.remarks);
                            return 2;
                        }
                    }
                }
                if 1.0 < own {
                    let room = wear_room(&scene.items, me.pack(), piece.as_ref());
                    if f64::from(piece.bulk()) * 0.15 < f64::from(room) {
                        deeds.push(Deed::Wear {
                            object,
                            source,
                            item,
                        });
                    } else {
                        deeds.push(Deed::Take {
                            object,
                            source,
                            item,
                        });
                        deeds.push(Deed::Carry { item });
                        me.pack().carried.push(Carried { item, flags: 0 });
                    }
                    return 1;
                }
                0
            }
            Loot::Weapon(w) => {
                if w.flags & 0x0600_0000 != 0 {
                    return 0;
                }
                let own = weapon_worth(&scene.items, me.pack(), item);
                if let Some(theirs_pack) = leader {
                    if w.special != 0 {
                        say(me, deeds, &mut self.remarks);
                        return 2;
                    }
                    let theirs = weapon_worth(&scene.items, theirs_pack, item);
                    let edge = 1.1 - f64::from(w.value()) * 0.1;
                    if (own - 1.0) * 0.5 < theirs - 1.0 && ((edge * edge + 0.8) as f32) < theirs {
                        say(me, deeds, &mut self.remarks);
                        return 2;
                    }
                }
                if 1.05 < f64::from(own) {
                    deeds.push(Deed::Take {
                        object,
                        source,
                        item,
                    });
                    deeds.push(Deed::Carry { item });
                    me.pack().carried.push(Carried { item, flags: 0 });
                    deeds.extend(rearm(&scene.items, me.pack()));
                    return 1;
                }
                0
            }
            Loot::Other => 0,
        }
    }
}

/// Carries out one world edit the search asked for. True when it happened.
///
/// Taking a loose item despawns its object; taking from a container removes
/// the first matching entry and compacts the list; taking a corpse's worn
/// item removes it from the worn list and from a hand that held it; taking
/// a carried entry removes that entry. Carrying adds to the pack, wearing
/// adds to the worn list after taking it out of its source, wielding fills
/// the main hand and stowing the first stowed slot. Dropping into a
/// container refuses when its capacity is known and full. A comment changes
/// no state.
pub fn apply_deed(scene: &mut Scene, pack: &mut Pack, deed: &Deed) -> bool {
    match deed {
        Deed::Take {
            object,
            source,
            item,
        } => take(scene, *object, *source, *item),
        Deed::Carry { item } => {
            if !pack.carried.iter().any(|c| c.item == *item) {
                pack.carried.push(Carried {
                    item: *item,
                    flags: 0,
                });
            }
            true
        }
        Deed::Wear {
            object,
            source,
            item,
        } => {
            if !take(scene, *object, *source, *item) {
                return false;
            }
            if !pack.worn.contains(item) {
                pack.worn.push(*item);
            }
            true
        }
        Deed::Wield { item } => {
            remove_from_pack(pack, *item);
            pack.hands[0] = *item;
            true
        }
        Deed::Stow { item } => {
            remove_from_pack(pack, *item);
            pack.hands[2] = *item;
            true
        }
        Deed::Drop { item, container } => {
            let Some(c) = scene.containers.get_mut(container) else {
                return false;
            };
            if c.capacity != 0 && c.items.len() as u32 >= c.capacity {
                return false;
            }
            c.items.push(*item);
            true
        }
        Deed::Comment { .. } => true,
    }
}

/// Carries out a list of world edits, answering how many happened.
pub fn apply_deeds(scene: &mut Scene, pack: &mut Pack, deeds: &[Deed]) -> usize {
    deeds
        .iter()
        .filter(|deed| apply_deed(scene, pack, deed))
        .count()
}

fn remove_from_pack(pack: &mut Pack, item: u32) {
    pack.carried.retain(|c| c.item != item);
    pack.worn.retain(|w| *w != item);
    for hand in 0..pack.hands.len() {
        if pack.hands[hand] == item {
            pack.hands[hand] = 0;
        }
    }
}

fn take(scene: &mut Scene, object: usize, source: Source, item: u32) -> bool {
    match source {
        Source::Item => {
            let Some(o) = scene.objects.get_mut(object) else {
                return false;
            };
            o.kind &= !object::PRESENT;
            despawn(scene, object);
            true
        }

        Source::Container => {
            let Some(id) = scene.objects.get(object).map(|o| o.id) else {
                return false;
            };
            let Some(c) = scene.containers.get_mut(&id) else {
                return false;
            };
            let Some(at) = c.items.iter().position(|i| *i == item) else {
                return false;
            };
            c.items.remove(at);
            true
        }
        Source::Worn | Source::Carried => {
            let Some(at) = scene.objects.get(object).and_then(|o| o.corpse) else {
                return false;
            };
            let Some(corpse) = scene.corpses.get_mut(at) else {
                return false;
            };
            match source {
                Source::Worn => {
                    let Some(at) = corpse.pack.worn.iter().position(|w| *w == item) else {
                        return false;
                    };
                    corpse.pack.worn.remove(at);
                    for hand in 0..corpse.pack.hands.len() {
                        if corpse.pack.hands[hand] == item {
                            corpse.pack.hands[hand] = 0;
                        }
                    }
                    true
                }
                _ => {
                    let Some(at) = corpse.pack.carried.iter().position(|c| c.item == item) else {
                        return false;
                    };
                    corpse.pack.carried.remove(at);
                    true
                }
            }
        }
    }
}

/// Removes a despawned object from the room grid's cell lists.
fn despawn(scene: &mut Scene, object: usize) {
    for cell in &mut scene.grid.objects {
        cell.retain(|o| *o != object);
    }
}

enum Found {
    Character(usize),
    Object(usize),
}

impl RoomGrid {
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || self.width <= x || self.height <= y {
            return None;
        }
        Some((self.width * y + x) as usize)
    }
}

/// Takes the best weapon in the main hand, and stows the next best (one
/// of the same hand group counting 0.6).
pub fn rearm(items: &BTreeMap<u32, Loot>, pack: &Pack) -> Vec<Deed> {
    let weapons: Vec<(u32, usize, f32)> = pack
        .carried
        .iter()
        .filter_map(|c| arm(items, c.item).map(|w| (c.item, w.class(), w.value())))
        .collect();
    if weapons.is_empty() {
        return Vec::new();
    }
    let mut best = [0.0f32; 4];
    let mut best_at = [0usize; 4];
    for (i, (_, class, v)) in weapons.iter().enumerate() {
        if best[*class] < *v {
            best[*class] = *v;
            best_at[*class] = i;
        }
    }
    let pick = if pack.hands[1] == 0 || pack.hands[1] == pack.hands[0] {
        let mut top = 0.0f32;
        let mut at = 0;
        for c in 0..4 {
            if top < best[c] {
                top = best[c];
                at = best_at[c];
            }
        }
        at
    } else if best[0] < best[1] {
        best_at[1]
    } else {
        best_at[0]
    };
    let mut deeds = Vec::new();
    let main = weapons[pick].0;
    if main != pack.hands[0] {
        deeds.push(Deed::Wield { item: main });
    }
    let mut second = 0;
    let mut top = 0.0f32;
    for (i, (_, class, v)) in weapons.iter().enumerate() {
        if i == pick {
            continue;
        }
        let v = if class & 1 == weapons[pick].1 & 1 {
            v * 0.6
        } else {
            *v
        };
        if top < v {
            second = i;
            top = v;
        }
    }
    if 0.0 < top {
        deeds.push(Deed::Stow {
            item: weapons[second].0,
        });
    }
    deeds
}

/// What tidying a pack leaves in `container` once that container is
/// emptied: the weapon of lowest value over its class's best, when under
/// 0.8 (a special weapon counts 1000), and each carried piece of apparel,
/// not special, that another piece it overlaps makes redundant:
/// `m/m' + s/s'` under 2 (from 10).
pub fn tidy(items: &BTreeMap<u32, Loot>, pack: &Pack, container: u32) -> Vec<Deed> {
    let mut deeds = Vec::new();
    let mut pieces: Vec<(u32, &Apparel, (f32, f32))> = Vec::new();
    let mut weapons: Vec<(u32, usize, f32)> = Vec::new();
    let mut best = [0.0f32; 4];
    let mut note = |id: u32, w: &Arm, weapons: &mut Vec<(u32, usize, f32)>| {
        let v = w.value();
        let b = &mut best[w.class()];
        if *b < v {
            *b = v;
        }
        let v = if w.special != 0 { 1000.0 } else { v };
        weapons.push((id, w.class(), v));
    };
    for c in &pack.carried {
        if let Some(a) = apparel(items, c.item) {
            pieces.push((c.item, a, a.coverage()));
        }
        if let Some(w) = arm(items, c.item) {
            note(c.item, w, &mut weapons);
        }
    }
    let carried_pieces = pieces.len();
    for w in &pack.worn {
        if let Some(a) = apparel(items, *w) {
            pieces.push((*w, a, a.coverage()));
        }
    }
    for slot in [0, 2, 1, 3] {
        let id = pack.hands[slot];
        if id != 0
            && let Some(w) = arm(items, id)
        {
            note(id, w, &mut weapons);
        }
    }
    let mut worst = None;
    let mut low = 1.0f32;
    for (i, (_, class, v)) in weapons.iter().enumerate() {
        let r = v / best[*class];
        if r < low {
            low = r;
            worst = Some(i);
        }
    }
    if low < 0.8
        && let Some(i) = worst
    {
        deeds.push(Deed::Drop {
            item: weapons[i].0,
            container,
        });
    }
    for i in 0..carried_pieces {
        let (id, piece, (s, m)) = pieces[i];
        if piece.special != 0 {
            continue;
        }
        let mut least = 10.0f32;
        for (j, (_, other, (s2, m2))) in pieces.iter().enumerate() {
            if j != i && piece.overlaps(other) {
                let r = m / m2 + s / s2;
                if r < least {
                    least = r;
                }
            }
        }
        if least < 2.0 {
            deeds.push(Deed::Drop {
                item: id,
                container,
            });
        }
    }
    deeds
}

#[cfg(test)]
#[path = "search_test.rs"]
mod tests;
