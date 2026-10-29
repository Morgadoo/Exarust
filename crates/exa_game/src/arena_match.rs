//! An arena career match, from its start to the return to the hub.
//!
//! A match starts by drawing its arena. The roll steps by
//! `× 0x08088405 + 1`, and the high half of the word times 6 is compared
//! with the variety count plus 2: above it, an indoor arena is drawn and the
//! count rises; otherwise an outdoor arena is drawn and the count falls. The
//! Beast takes arena5 or arena7 when the roll says indoor. A draw never
//! repeats the last five indoor or the last three outdoor arenas. The music
//! is one of eleven tracks, never one of the last five, at its own volume;
//! the Beast plays its own track and leaves the list alone. A match ticks
//! once its first second has passed: the player's team is out when none of
//! it stands, the other side when none of it stands. A won round continues
//! while the rounds won so far are fewer than the extra rounds, a lost round
//! while the losses are fewer than the lives. A fallen company fighter is
//! dead below 0.01 health, or with a physician hired below −0.15 of the
//! maximum; a fighter who is not dead but is below 0.01 was saved. Health
//! goes back to the record as a word of 65535 times its share.
//!
//! Evidence: the retail arena's match start, arena loader, match tick, round
//! end, fall, continue, forfeit and return routines, read from `Exanima.exe`
//! with arithmetic from the instructions (`context/re/arena-match.md`).

use bevy::prelude::*;
use exa_combat::{Condition, Fighter};

use crate::arena_log::ArenaLog;
use crate::effects::EffectClock;
use crate::player::PlayerBody;

/// The company log's record of a won match, with the template id.
pub const LOG_WON: i32 = 0x20000;
/// A lost match, with the template id.
pub const LOG_LOST: i32 = 0x30000;
/// A forfeited match, with the template id.
pub const LOG_FORFEIT: i32 = 0x40000;
/// A company fighter who died, with the name's sixteen bytes.
pub const LOG_DIED: i32 = 0x60000;
/// A company fighter saved from a lethal injury, with the fighter's id.
pub const LOG_SAVED: i32 = 0x70000;
/// The Beast's template id.
pub const BEAST: i32 = 11;
/// Matches the board runs a day. At three its Begin is disabled.
pub const MATCHES_PER_DAY: u32 = 3;
/// A match does not tick before this much game time has passed, in ms.
pub const FIRST_TICK_MS: u64 = 1000;
/// The team-id field's value for the player's team.
pub const PLAYER_TEAM: i32 = 1;
/// Indoor arenas come first in the table, outdoor arenas after them.
pub const INDOOR_ARENAS: i32 = 7;
pub const OUTDOOR_ARENAS: i32 = 5;
/// Below this health (compared in double precision) a fighter is dead or
/// was saved.
pub const DEAD_HEALTH: f64 = 0.01;
/// With a physician hired, a fighter is dead below this share of the
/// maximum (compared in double precision).
pub const PHYSICIAN_DEAD_SHARE: f64 = -0.15;
/// The double the record's health word is multiplied by to read it back.
pub const WORD_SHARE: f64 = f64::from_bits(0x3ef0_0010_0010_0010);

/// One arena a career match is fought in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArenaMap {
    /// The level's name.
    pub name: &'static str,
    /// Where the player's side stands, in centimetres.
    pub spawn: [f32; 3],
    /// An indoor arena's amount for the view's post-process (copied to the
    /// post processor; what it drives there is not traced). Outdoor 0.
    pub view_amount: f32,
    /// An outdoor arena lets the environment draw take its one-in-six
    /// branch (time of day from 0.76 to 1).
    pub late_draw: bool,
    /// An outdoor arena plays the `NtrA` bed by day instead of the wind.
    pub steady_bed: bool,
}

const fn map(
    name: &'static str,
    spawn: [f32; 3],
    view_amount: f32,
    late_draw: bool,
    steady_bed: bool,
) -> ArenaMap {
    ArenaMap {
        name,
        spawn,
        view_amount,
        late_draw,
        steady_bed,
    }
}

/// The twelve arenas: seven indoor, then five outdoor.
pub const ARENA_MAPS: [ArenaMap; 12] = [
    map("arena1", [0.0, 0.0, 300.0], 0.0, false, false),
    map("arena2", [0.0, 0.0, 300.0], 0.3, false, false),
    map("arena3", [-300.0, 0.0, 300.0], 0.0, false, false),
    map("arena4", [-1500.0, 0.0, -450.0], 0.6, false, false),
    map("arena5", [-300.0, 0.0, -300.0], 0.0, false, false),
    map("arena6", [300.0, 0.0, 300.0], 0.0, false, false),
    map("arena7", [0.0, 0.0, 0.0], 0.0, false, false),
    map("arenao1", [0.0, 0.0, 0.0], 0.0, true, true),
    map("arenao2", [0.0, 0.0, 0.0], 0.0, true, false),
    map("arenao3", [900.0, 0.0, 100.0], 0.0, true, true),
    map("arenao4", [0.0, 0.0, 0.0], 0.0, true, false),
    map("arenao5", [800.0, 0.0, 100.0], 0.0, false, true),
];

/// The arena a level name names, as an index into [`ARENA_MAPS`].
pub fn arena_index(level: &str) -> Option<usize> {
    let stem = level.strip_suffix(".rfc").unwrap_or(level);
    ARENA_MAPS
        .iter()
        .position(|arena| arena.name.eq_ignore_ascii_case(stem))
}

fn step(roll: &mut u32) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    *roll
}

/// The high half of the roll times `count`, both unsigned.
fn below(roll: u32, count: u32) -> u32 {
    ((u64::from(roll) * u64::from(count)) >> 32) as u32
}

/// Draw one of `count` values that is not among the last `count − 2`.
///
/// Each try steps the roll and takes the high half of the word times
/// `count`. A value found among the first `count − 2` entries, or one that
/// reads as negative, is drawn again. The entries move one place along,
/// the last falls off, and the value goes first.
fn draw_unrepeated(recent: &mut [i32], count: i32, roll: &mut u32) -> i32 {
    let checked = count.wrapping_sub(3);
    loop {
        step(roll);
        let product = (i64::from(count) as u64).wrapping_mul(u64::from(*roll));
        let mut value = (product >> 32) as u32 as i32;
        if checked >= 0 {
            for index in 0..=checked as usize {
                if recent.get(index) == Some(&value) {
                    value = -1;
                    break;
                }
            }
        }
        if value < 0 {
            continue;
        }
        if checked >= 1 {
            for index in (1..=checked as usize).rev() {
                if index < recent.len() {
                    recent[index] = recent[index - 1];
                }
            }
        }
        if let Some(first) = recent.first_mut() {
            *first = value;
        }
        return value;
    }
}

/// What the arena draw remembers from match to match. The original never
/// sets these before the first match, so they start at zero, and arena1,
/// arenao1 and the first track are refused until the lists move on.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArenaPicks {
    /// Rises with each indoor arena and falls with each outdoor one.
    pub variety: i32,
    /// The last five indoor arenas, newest first.
    pub indoor: [i32; 5],
    /// The last three outdoor arenas (indices from 0), newest first.
    pub outdoor: [i32; 3],
    /// The last five tracks, newest first.
    pub tracks: [i32; 5],
}

/// Draw a match's arena, an index into [`ARENA_MAPS`].
///
/// The roll steps first. When the high half of the word times 6 is above
/// the variety count plus 2 the arena is indoor and the count rises: the
/// Beast steps again and takes arena5 when the high half of the word
/// times 2 is 0, else arena7; any other match draws one of the seven indoor
/// arenas not among the last five. Otherwise the count falls and one of the
/// five outdoor arenas not among the last three is drawn.
pub fn pick_arena(picks: &mut ArenaPicks, beast: bool, roll: &mut u32) -> usize {
    step(roll);
    let draw = i64::from(below(*roll, 6) as i32);
    if draw > i64::from(picks.variety) + 2 {
        let index = if beast {
            step(roll);
            if below(*roll, 2) == 0 { 4 } else { 6 }
        } else {
            draw_unrepeated(&mut picks.indoor, INDOOR_ARENAS, roll)
        };
        picks.variety = picks.variety.wrapping_add(1);
        index as usize
    } else {
        picks.variety = picks.variety.wrapping_sub(1);
        let outdoor = draw_unrepeated(&mut picks.outdoor, OUTDOOR_ARENAS, roll);
        (outdoor + INDOOR_ARENAS) as usize
    }
}

/// A music track and the volume it plays at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track {
    pub name: &'static str,
    pub volume: f32,
}

const fn track(name: &'static str, volume_bits: u32) -> Track {
    Track {
        name,
        volume: f32::from_bits(volume_bits),
    }
}

const TENTH: u32 = 0x3dcc_cccd;
const FIFTEENTH: u32 = 0x3e19_999a;

/// The eleven match tracks. The first two keep the original's spelling.
pub const TRACKS: [Track; 11] = [
    track("Msc_AreanNov", TENTH),
    track("Msc_AreanExp", FIFTEENTH),
    track("Msc_ArenaBossN", TENTH),
    track("Msc_ArenaBossE", 0x3d8f_5c29),
    track("Msc_Arena2", TENTH),
    track("Msc_Arena3", FIFTEENTH),
    track("Msc_Arena4", FIFTEENTH),
    track("Msc_Fortitude", TENTH),
    track("Msc_OreSum", FIFTEENTH),
    track("Msc_Tourney01", FIFTEENTH),
    track("Msc_Tourney02", 0x3df5_c28f),
];

/// The same eleven with the alternative music option set.
pub const ALTERNATIVE_TRACKS: [Track; 11] = [
    track("Msc_ArenaNov_DnB", FIFTEENTH),
    track("Msc_ArenaExp_DnB", FIFTEENTH),
    track("Msc_ArenaBsN_DnB", FIFTEENTH),
    track("Msc_ArenaBsE_DnB", FIFTEENTH),
    track("Msc_Arena2_DnB", FIFTEENTH),
    track("Msc_Arena3_DnB", FIFTEENTH),
    track("Msc_Arena4_DnB", FIFTEENTH),
    track("Msc_Fortitud_DnB", 0x3e4c_cccd),
    track("Msc_OreSum_DnB", FIFTEENTH),
    track("Msc_Tourn01_DnB", FIFTEENTH),
    track("Msc_Tourn02_DnB", FIFTEENTH),
];

/// The Beast's track, and its alternative.
pub const BEAST_TRACK: Track = track("Msc_ArenaO", 0x3e99_999a);
pub const BEAST_ALTERNATIVE: Track = track("Msc_ArenaO_DnB", 0x3e99_999a);

/// Draw a match's music. The Beast plays its own track and neither steps
/// the roll nor moves the list. Any other match steps the roll until the
/// high half of the word times 11 is none of the last five tracks; the
/// list moves one place along and the new track goes first.
pub fn pick_track(tracks: &mut [i32; 5], roll: &mut u32, beast: bool, alternative: bool) -> Track {
    if beast {
        return if alternative {
            BEAST_ALTERNATIVE
        } else {
            BEAST_TRACK
        };
    }
    let pick = loop {
        step(roll);
        let value = below(*roll, 11) as i32;
        if !tracks.contains(&value) {
            break value;
        }
    };
    tracks.copy_within(0..4, 1);
    tracks[0] = pick;
    let table = if alternative {
        &ALTERNATIVE_TRACKS
    } else {
        &TRACKS
    };
    table[pick as usize]
}

/// What a match's start drew: its arena and its music.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatchStart {
    pub arena: usize,
    pub track: Track,
    /// The roll after both draws.
    pub roll: u32,
}

/// Start a match from a performance counter: the roll is the counter's low
/// word with its bytes reversed, then the arena is drawn and then the music,
/// both from that roll.
pub fn start_match(
    picks: &mut ArenaPicks,
    template: i32,
    counter: u64,
    alternative: bool,
) -> MatchStart {
    let mut roll = (counter as u32).swap_bytes();
    let beast = template == BEAST;
    let arena = pick_arena(picks, beast, &mut roll);
    let track = pick_track(&mut picks.tracks, &mut roll, beast, alternative);
    MatchStart { arena, track, roll }
}

/// The bed an outdoor arena plays once the environment is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutdoorBed {
    /// Rain over 0.01 (in double), or a calm day without the steady bed.
    Silent,
    /// A time of day under 0.5: `NtrAN` at 0.3.
    Night,
    /// By day with the steady bed: `NtrA` at 1.
    Steady,
    /// By day, wind over 0.1 (in double): `NtrW` at 1, its gain the double
    /// 0.2 times the wind squared and its pitch the double 0.6 plus half
    /// the wind.
    Wind { gain: f32, pitch: f32 },
}

/// The outdoor bed for an arena from the drawn rain, time of day and wind.
pub fn outdoor_bed(arena: &ArenaMap, rain: f32, time_of_day: f32, wind: f32) -> OutdoorBed {
    if f64::from(rain) > DEAD_HEALTH {
        return OutdoorBed::Silent;
    }
    if 0.5f32 > time_of_day {
        return OutdoorBed::Night;
    }
    if arena.steady_bed {
        return OutdoorBed::Steady;
    }
    if f64::from(wind) > 0.1 {
        let gain = (f64::from(wind * wind) * 0.2) as f32;
        let pitch = (f64::from(wind * 0.5) + 0.6) as f32;
        return OutdoorBed::Wind { gain, pitch };
    }
    OutdoorBed::Silent
}

/// Whether the board still runs a match today.
pub fn board_open(matches_today: u32) -> bool {
    matches_today < MATCHES_PER_DAY
}

/// Where a match stands, as the session's state word names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchState {
    /// Fighting (3).
    Fighting,
    /// The match is won (4).
    Won,
    /// The match is lost (5).
    Lost,
    /// Another round follows (8).
    NextFight,
}

impl MatchState {
    pub fn word(self) -> u32 {
        match self {
            Self::Fighting => 3,
            Self::Won => 4,
            Self::Lost => 5,
            Self::NextFight => 8,
        }
    }
}

/// A match's rounds. A start sets no extra rounds and one life; Elimination
/// sets four extra rounds, Reserve two extra rounds and three lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchRounds {
    /// The round being fought, from 0.
    pub round: i32,
    /// Rounds lost so far.
    pub losses: i32,
    /// Rounds a win continues into.
    pub extra_rounds: i32,
    /// Losses that end the match.
    pub lives: i32,
}

impl Default for MatchRounds {
    fn default() -> Self {
        Self {
            round: 0,
            losses: 0,
            extra_rounds: 0,
            lives: 1,
        }
    }
}

impl MatchRounds {
    /// The rounds a template's setup sets: Elimination (7) and Reserve (5)
    /// their own, every other template the default.
    pub fn for_template(template: i32) -> Self {
        match template {
            7 => Self::elimination(),
            5 => Self::reserve(),
            _ => Self::default(),
        }
    }

    /// Elimination's rounds: five wins in a row.
    pub fn elimination() -> Self {
        Self {
            extra_rounds: 4,
            ..Self::default()
        }
    }

    /// Reserve's rounds: three wins before three losses.
    pub fn reserve() -> Self {
        Self {
            extra_rounds: 2,
            lives: 3,
            ..Self::default()
        }
    }

    /// End a round. A win continues while the round less the losses is
    /// below the extra rounds (compared in 64 bits), else the match is won.
    /// A loss counts one more loss and continues while the losses are below
    /// the lives, else the match is lost.
    pub fn finish_round(&mut self, won: bool) -> MatchState {
        if won {
            let ahead = i64::from(self.round) - i64::from(self.losses);
            if ahead < i64::from(self.extra_rounds) {
                MatchState::NextFight
            } else {
                MatchState::Won
            }
        } else {
            self.losses = self.losses.wrapping_add(1);
            if self.losses < self.lives {
                MatchState::NextFight
            } else {
                MatchState::Lost
            }
        }
    }
}

/// Whether a match ticks: its first second has passed. The stamp plus
/// 1000 is compared with the clock, both widened to 64 bits.
pub fn tick_due(started_ms: u32, now_ms: u32) -> bool {
    u64::from(started_ms) + FIRST_TICK_MS <= u64::from(now_ms)
}

/// One fighter in the scene, as a match tick reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneFighter {
    /// The team-id field; [`PLAYER_TEAM`] is the player's team.
    pub team: i32,
    /// Its condition word is dead (0xA00).
    pub dead: bool,
    /// A company fighter: its record carries the match-participant flag.
    pub company: bool,
    /// Its fall was already resolved (the record's flag was set).
    pub resolved: bool,
    /// Its red health.
    pub health: f32,
    /// Its maximum health.
    pub max: f32,
    /// The record's low byte; 0 is the manager.
    pub id: u8,
    /// A company fighter's name's sixteen bytes, for a death log.
    pub name: Option<[u8; 16]>,
    /// This fighter is the manager (the original's player pointer).
    pub manager: bool,
}

impl SceneFighter {
    /// On the player's team, judged by the team-id field.
    pub fn ours(&self) -> bool {
        self.team == PLAYER_TEAM
    }
}

/// What one tick counted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Every fighter on the player's team, dead or not.
    pub ours: i32,
    /// The player's team still standing.
    pub ours_standing: i32,
    /// Everyone else still standing.
    pub theirs_standing: i32,
    /// Company fighters newly dead, in scene order; they are now resolved.
    pub fallen: Vec<usize>,
}

impl Tally {
    /// The round's end, if this tick ends it: lost when none of the
    /// player's team stands and it has anyone, else won when nobody else
    /// stands.
    pub fn round_end(&self) -> Option<bool> {
        if self.ours_standing == 0 && self.ours > 0 {
            Some(false)
        } else if self.theirs_standing == 0 {
            Some(true)
        } else {
            None
        }
    }
}

/// Count the scene's fighters for one tick. A dead company fighter not yet
/// resolved is listed and marked resolved; a dead fighter counts as standing
/// for nobody.
pub fn tally(fighters: &mut [SceneFighter]) -> Tally {
    let mut tally = Tally::default();
    for (index, fighter) in fighters.iter_mut().enumerate() {
        if fighter.ours() {
            tally.ours += 1;
        }
        if fighter.dead {
            if fighter.company && !fighter.resolved {
                tally.fallen.push(index);
                fighter.resolved = true;
            }
        } else if fighter.ours() {
            tally.ours_standing += 1;
        } else {
            tally.theirs_standing += 1;
        }
    }
    tally
}

/// Whether a fallen fighter is dead. With a physician hired it is the
/// share of the maximum below −0.15 (the share divided in single precision,
/// compared in double), otherwise the health below 0.01. NaN is not dead.
pub fn fatal(health: f32, max: f32, physician: bool) -> bool {
    if physician {
        f64::from(health / max) < PHYSICIAN_DEAD_SHARE
    } else {
        f64::from(health) < DEAD_HEALTH
    }
}

/// How a fall ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fall {
    /// The manager (id 0) is dead: the career's saves are wiped.
    ManagerDied,
    /// A hired fighter is dead: the death is logged and counted and the
    /// fighter leaves.
    Died,
    /// Not dead, but under 0.01: logged as saved from a lethal injury.
    Saved,
    /// Neither: nothing is logged.
    Survived,
}

/// Resolve a fall: the fighter's id is its record's low byte, 0 for the
/// manager.
pub fn resolve_fall(health: f32, max: f32, physician: bool, id: u8) -> Fall {
    if fatal(health, max, physician) {
        if id == 0 {
            Fall::ManagerDied
        } else {
            Fall::Died
        }
    } else if f64::from(health) < DEAD_HEALTH {
        Fall::Saved
    } else {
        Fall::Survived
    }
}

/// Log a fall. A death names the fighter by the name's sixteen bytes and
/// raises the death count; a save names the fighter by id. A death whose id
/// is on nobody hired logs nothing.
pub fn log_fall(
    log: &mut ArenaLog,
    fall: Fall,
    id: u8,
    name: Option<[u8; 16]>,
    deaths: &mut u32,
) -> bool {
    match fall {
        Fall::Died => {
            let Some(name) = name else {
                return false;
            };
            let words = std::array::from_fn(|index| {
                let at = index * 4;
                i32::from_le_bytes([name[at], name[at + 1], name[at + 2], name[at + 3]])
            });
            log.append_words(LOG_DIED, words);
            *deaths = deaths.wrapping_add(1);
            true
        }
        Fall::Saved => {
            log.append_value(LOG_SAVED, i32::from(id));
            true
        }
        Fall::ManagerDied | Fall::Survived => false,
    }
}

/// What a round's end set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoundEnd {
    /// The player's team won.
    pub won: bool,
    /// The state the round's end set ([`MatchRounds::finish_round`]).
    pub state: MatchState,
}

/// The manager's fall, as one tick found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManagerDown {
    /// The fall was fatal: the career's saves are wiped (permadeath).
    pub fatal: bool,
    /// The end screen the original schedules: 3 when fatal, else 2.
    pub screen: u32,
    /// How many of the player's team still stand. Above 0 the original
    /// also hands the camera back to the next fighter.
    pub ours_standing: i32,
    /// The round's end this tick also made, if any.
    pub round: Option<RoundEnd>,
}

/// What one match tick did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchTick {
    /// The first second has not passed; nothing was counted.
    NotDue,
    /// The tick counted the teams, but neither a round nor the manager
    /// ended.
    Nothing,
    /// The round ended and set the match's state.
    RoundEnded(RoundEnd),
    /// The manager went down; a round may also have ended this tick. The
    /// manager branch runs after the round's, as in the original.
    ManagerDown(ManagerDown),
}

/// One match tick: the common arena tick.
///
/// Once the first second has passed, it walks the scene in order, counts
/// each fighter by its team-id field ([`PLAYER_TEAM`] is the player's
/// team), and resolves each newly dead company fighter's fall (logging a
/// death or a save, counting a death and calling `remove` for a fighter on
/// the company's books). It then ends the round when the player's team has
/// nobody standing but has fighters, else when nobody else stands. Last,
/// once, it handles a dead manager: its fall is resolved if the scene did
/// not already resolve it, `manager_handled` is set, and the end screen is
/// 3 for a fatal fall (the permadeath branch) or 2 otherwise.
///
/// `remove` is the original's employee-removal call: it takes the scene
/// index and the record id of a company fighter who died.
#[allow(clippy::too_many_arguments)]
pub fn match_tick(
    fighters: &mut [SceneFighter],
    rounds: &mut MatchRounds,
    state: &mut MatchState,
    manager_handled: &mut bool,
    physician: bool,
    log: &mut ArenaLog,
    deaths: &mut u32,
    mut remove: impl FnMut(usize, u8),
    started_ms: u32,
    now_ms: u32,
) -> MatchTick {
    if !tick_due(started_ms, now_ms) {
        return MatchTick::NotDue;
    }
    let counted = tally(fighters);
    for &index in &counted.fallen {
        let fallen = fighters[index];
        let fall = resolve_fall(fallen.health, fallen.max, physician, fallen.id);
        if log_fall(log, fall, fallen.id, fallen.name, deaths) && fall == Fall::Died {
            remove(index, fallen.id);
        }
    }
    let round = counted.round_end().map(|won| {
        let next = rounds.finish_round(won);
        *state = next;
        RoundEnd { won, state: next }
    });
    let manager = fighters.iter().position(|fighter| fighter.manager);
    if let Some(index) = manager {
        let fallen = fighters[index];
        if fallen.dead && !*manager_handled {
            *manager_handled = true;
            // The counting pass resolves the manager too when it is on the
            // company's books; the record's flag makes this a no-op then.
            if !fallen.resolved {
                let fall = resolve_fall(fallen.health, fallen.max, physician, fallen.id);
                log_fall(log, fall, fallen.id, fallen.name, deaths);
                fighters[index].resolved = true;
            }
            let fatal = fatal(fallen.health, fallen.max, physician);
            return MatchTick::ManagerDown(ManagerDown {
                fatal,
                screen: if fatal { 3 } else { 2 },
                ours_standing: counted.ours_standing,
                round,
            });
        }
    }
    match round {
        Some(round) => MatchTick::RoundEnded(round),
        None => MatchTick::Nothing,
    }
}

/// The health word a record keeps after a match: 0 at no health, else the
/// share of the maximum times 65535 converted to the nearest even integer
/// in single precision, kept to its low sixteen bits.
pub fn health_word(health: f32, max: f32) -> u16 {
    if 0.0f32 < health {
        let scaled = health / max * 65535.0;
        convert(scaled) as u16
    } else {
        0
    }
}

/// The health a record's word gives back: the maximum times the word times
/// 1/65535, in double, stored as single.
pub fn health_from_word(word: u16, max: f32) -> f32 {
    (f64::from(max) * (f64::from(word) * WORD_SHARE)) as f32
}

/// The processor's single-to-integer conversion: ties to even, and a NaN or
/// out-of-range value becomes the 64-bit minimum.
fn convert(value: f32) -> i64 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || rounded >= 9.223_372e18 || rounded < -9.223_372e18 {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// The processor's 32-bit single-to-integer conversion: ties to even, and a
/// NaN or a value outside the signed 32-bit range becomes the indefinite
/// integer, `i32::MIN`.
pub(crate) fn cvtss2si32(value: f32) -> i32 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || rounded >= 2_147_483_648.0 || rounded < -2_147_483_648.0 {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// The share of a full bar a record's health word fills in the company
/// tables: the word times the double 1/65535, stored as single.
pub fn health_bar(word: u16) -> f32 {
    (f64::from(word) * WORD_SHARE) as f32
}

/// Whether any of the week's six events has a fighter entered.
pub fn any_entry(rows: &crate::arena::Roster) -> bool {
    rows.iter().any(|row| (row.count as i32) > 0)
}

/// Whether today is a tournament day (a multiple of 7) with an entered
/// event that has no winner yet.
pub fn event_pending(day: u32, rows: &crate::arena::Roster) -> bool {
    day.is_multiple_of(7)
        && rows
            .iter()
            .any(|row| (row.count as i32) > 0 && row.team_id == 0)
}

/// Whether the manage window lets the day end: on a tournament day with
/// entries, once every entered event is decided; any other day, once a
/// match was fought.
pub fn day_can_end(day: u32, rows: &crate::arena::Roster, matches_today: u32) -> bool {
    if day.is_multiple_of(7) && any_entry(rows) {
        return !event_pending(day, rows);
    }
    0 < matches_today as i32
}

/// The tournament event the board's Begin starts: the first with no winner
/// and a fighter entered.
pub fn next_event(rows: &crate::arena::Roster) -> Option<usize> {
    rows.iter()
        .position(|row| row.team_id == 0 && (row.count as i32) > 0)
}

/// The board's Begin warns once that the manager is entered (the tip
/// "You are about to enter your manager into a match..."): while the
/// warning is armed and the manager is a participant, it is disarmed and,
/// when the tip shows, the Begin waits.
pub fn warn_manager(armed: &mut bool, manager_entered: bool) -> bool {
    if *armed && manager_entered {
        *armed = false;
        return true;
    }
    false
}

/// What the end screen's Continue does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Continue {
    /// The next round is spawned; the round count rose.
    NextRound,
    /// Back to the hub, the match settled.
    Hub,
    /// The manager is dead: back to the main menu.
    MainMenu,
}

/// Continue after a round. A dead manager goes to the main menu. Another
/// round follows when the round less the losses is below the extra rounds
/// and the state is the next fight; the round count rises. Anything else
/// goes back to the hub.
pub fn continue_match(rounds: &mut MatchRounds, state: MatchState, manager_dead: bool) -> Continue {
    if manager_dead {
        return Continue::MainMenu;
    }
    let ahead = i64::from(rounds.round) - i64::from(rounds.losses);
    if ahead < i64::from(rounds.extra_rounds) && state == MatchState::NextFight {
        rounds.round = rounds.round.wrapping_add(1);
        Continue::NextRound
    } else {
        Continue::Hub
    }
}

/// Forfeit a match. A dead manager leaves it alone. Otherwise a match that
/// is not a tournament logs the forfeit with the template id; either way the
/// healths are written back and only the player data is saved.
pub fn forfeit(log: &mut ArenaLog, manager_dead: bool, tournament: bool, template: i32) -> bool {
    if manager_dead {
        return false;
    }
    if !tournament {
        log.append_value(LOG_FORFEIT, template);
    }
    true
}

/// The company's lifetime records, in the order the records window lists
/// them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompanyRecords {
    pub matches_won: u32,
    pub matches_lost: u32,
    pub matches_forfeited: u32,
    pub tournaments_won: u32,
    pub tournaments_lost: u32,
    pub seasons_won: u32,
    pub beasts_fought: u32,
    pub beasts_defeated: u32,
    pub hires: u32,
    pub deaths: u32,
    pub fired: u32,
    pub earned: u32,
    pub spent: u32,
}

impl CompanyRecords {
    /// The records window's thirteen labels, in order.
    pub const LABELS: [&'static str; 13] = [
        "Matches won",
        "Matches lost",
        "Matches forfeited",
        "Tournaments won",
        "Tournaments lost",
        "Seasons won",
        "Beasts fought",
        "Beasts defeated",
        "Total Hires",
        "Deaths",
        "Fired",
        "Total earned",
        "Total spent",
    ];

    /// The thirteen values in the window's order.
    pub fn values(&self) -> [u32; 13] {
        [
            self.matches_won,
            self.matches_lost,
            self.matches_forfeited,
            self.tournaments_won,
            self.tournaments_lost,
            self.seasons_won,
            self.beasts_fought,
            self.beasts_defeated,
            self.hires,
            self.deaths,
            self.fired,
            self.earned,
            self.spent,
        ]
    }

    /// A match starts: the Beast counts one more fought.
    pub fn start(&mut self, template: i32) {
        if template == BEAST {
            self.beasts_fought = self.beasts_fought.wrapping_add(1);
        }
    }
}

/// How a match came back to the hub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    /// The session's result word 4.
    Won,
    /// The result word 0x10.
    Forfeited,
    /// Anything else.
    Lost,
}

/// What the return to the hub paid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Return {
    /// Copper added to the purse and to today's earnings.
    pub prize: u32,
}

/// Come back to the hub. Outside a tournament a win pays the prize, adds it
/// to today's earnings and logs the win, and a Beast win counts one more
/// beast defeated; a forfeit and a loss are logged. Then, tournament or
/// not, a win counts one more match won; a forfeit counts one more forfeit
/// and every other finish one more loss.
pub fn return_to_hub(
    records: &mut CompanyRecords,
    log: &mut ArenaLog,
    finish: Finish,
    template: i32,
    tournament: bool,
    prize: u32,
) -> Return {
    let mut paid = 0;
    if !tournament {
        match finish {
            Finish::Won => {
                paid = prize;
                log.append_value(LOG_WON, template);
                if template == BEAST {
                    records.beasts_defeated = records.beasts_defeated.wrapping_add(1);
                }
            }
            Finish::Forfeited => {
                log.append_value(LOG_FORFEIT, template);
            }
            Finish::Lost => {
                log.append_value(LOG_LOST, template);
            }
        }
    }
    if finish == Finish::Won {
        records.matches_won = records.matches_won.wrapping_add(1);
    } else {
        if finish == Finish::Forfeited {
            records.matches_forfeited = records.matches_forfeited.wrapping_add(1);
        }
        records.matches_lost = records.matches_lost.wrapping_add(1);
    }
    Return { prize: paid }
}

/// After the return, the day's match count. On a day that is a multiple of
/// 7 with any fighter entered in the week's events the count stays (the
/// unfit are dropped from the entries instead); otherwise it rises. On
/// such a day the events before the first open entered one are then
/// settled (`arena::assign_round_ids`).
pub fn count_returned_match(day: u32, entered: bool, matches_today: &mut u32) {
    if day.is_multiple_of(7) && entered {
        return;
    }
    *matches_today = matches_today.wrapping_add(1);
}

/// A match being fought in the running game.
///
/// Play mode inserts it on an arena with an opponent. Its tick waits for
/// the first second of game time, then counts the player's team (the
/// player) against everyone else who fights, resolves each fallen company
/// fighter once, and ends the round as the original does. The transition
/// screen and the Continue/forfeit branch are kept here
/// ([`ArenaMatch::end_action`], [`ArenaMatch::forfeit`], and
/// [`crate::arena_session`]); the next round's bodies and the return to the
/// hub still have no scene caller.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ArenaMatch {
    /// The game clock at the start; stamped on the first frame.
    pub started_ms: Option<u32>,
    pub template: i32,
    pub rounds: MatchRounds,
    pub state: MatchState,
    pub physician: bool,
    pub log: ArenaLog,
    pub deaths: u32,
    /// Fighters whose fall was resolved.
    pub resolved: Vec<Entity>,
    /// The manager's own fall was handled: the end screen it showed, 3 when
    /// the fall was fatal, else 2.
    pub manager_screen: Option<u32>,
    /// The transition screen the round's result opened: 3 for the next
    /// round, 2 for the match's end.
    pub screen: Option<crate::arena_session::Screen>,
    /// The arena's spawn spot, centimetres: the layout's centre.
    pub spawn_at: [f32; 3],
    /// The match's rank, the generated fighters' level.
    pub level: i32,
    /// The event's gear tier.
    pub tier: i32,
    /// A tournament event: its next round raises the bracket's fight.
    pub tournament: bool,
    /// The company entry that is the player's own.
    pub player_entry: Option<usize>,
}

impl ArenaMatch {
    /// A match of `template`: Elimination's and Reserve's rounds, else one
    /// round and one life.
    pub fn begin(template: i32) -> Self {
        Self::begin_match(template, [0.0; 3], 0, 0, false, None)
    }

    /// A match of `template` in the arena at `spawn_at` (centimetres), at
    /// the event's rank and gear tier, a tournament or not, with the
    /// company entry that is the player's own.
    pub fn begin_match(
        template: i32,
        spawn_at: [f32; 3],
        level: i32,
        tier: i32,
        tournament: bool,
        player_entry: Option<usize>,
    ) -> Self {
        Self {
            started_ms: None,
            template,
            rounds: MatchRounds::for_template(template),
            state: MatchState::Fighting,
            physician: false,
            log: ArenaLog::default(),
            deaths: 0,
            resolved: Vec::new(),
            manager_screen: None,
            screen: None,
            spawn_at,
            level,
            tier,
            tournament,
            player_entry,
        }
    }

    /// The end screen's Continue, from the match's state. A manager whose
    /// fall was fatal goes to the main menu.
    pub fn end_action(&self, tournament: bool) -> crate::arena_session::EndAction {
        crate::arena_session::end_action(
            self.manager_screen == Some(3),
            self.state,
            &self.rounds,
            tournament,
        )
    }

    /// The forfeit guard, from the match's manager state.
    pub fn forfeit(&self, tournament: bool) -> crate::arena_session::ForfeitResult {
        crate::arena_session::forfeit(self.manager_screen == Some(3), tournament)
    }
}

/// A performance counter reading for the arena's draws. The original reads
/// the Windows counter; this stands in with the system clock in 100 ns
/// steps, the counter's usual rate.
pub fn counter_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| (elapsed.as_nanos() / 100) as u64)
        .unwrap_or(0)
}

/// Runs the match tick while an [`ArenaMatch`] is being fought.
pub struct ArenaMatchPlugin;

impl Plugin for ArenaMatchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArenaPicks>()
            .add_systems(Update, tick_arena_match);
    }
}

type MatchFighter<'a> = (
    Entity,
    &'a Fighter,
    &'a Condition,
    Has<PlayerBody>,
    Option<&'a crate::scene_bodies::SceneFighter>,
);

fn tick_arena_match(
    arena: Option<ResMut<ArenaMatch>>,
    clock: Option<Res<EffectClock>>,
    fighters: Query<MatchFighter>,
) {
    let (Some(mut arena), Some(clock)) = (arena, clock) else {
        return;
    };
    let arena = &mut *arena;
    if arena.state != MatchState::Fighting {
        return;
    }
    let now = clock.now_ms;
    let started = *arena.started_ms.get_or_insert(now);
    let mut entities = Vec::new();
    let mut scene = Vec::new();
    let mut index = 0u8;
    for (entity, fighter, condition, player, marker) in &fighters {
        entities.push(entity);
        // The player's body is the manager; a generated body carries the
        // side its plan stood it on.
        let company = player || marker.is_some_and(|marker| marker.company);
        scene.push(SceneFighter {
            team: if company {
                PLAYER_TEAM
            } else {
                marker.map_or(0, |marker| marker.team as i32)
            },
            dead: *condition == Condition::DEAD,
            company,
            resolved: arena.resolved.contains(&entity),
            health: fighter.health.red,
            max: fighter.health.max,
            // The player is the manager, id 0; a generated body its plan's
            // id, and anything else a placeholder.
            id: marker.map_or(if player { 0 } else { index.wrapping_add(1) }, |marker| {
                u8::try_from(marker.id).unwrap_or(u8::MAX)
            }),
            name: None,
            manager: player,
        });
        index = index.wrapping_add(1);
    }
    let mut handled = arena.manager_screen.is_some();
    let result = match_tick(
        &mut scene,
        &mut arena.rounds,
        &mut arena.state,
        &mut handled,
        arena.physician,
        &mut arena.log,
        &mut arena.deaths,
        |_, id| info!("arena match: company fighter {id} died"),
        started,
        now,
    );
    for (index, entity) in entities.iter().enumerate() {
        if scene[index].resolved && !arena.resolved.contains(entity) {
            arena.resolved.push(*entity);
        }
    }
    match result {
        MatchTick::NotDue | MatchTick::Nothing => {}
        MatchTick::RoundEnded(round) => {
            arena.screen = Some(crate::arena_session::Screen::after(round.state));
            info!(
                "arena match: round {} {} -> state {}",
                arena.rounds.round,
                if round.won { "won" } else { "lost" },
                round.state.word()
            );
        }
        MatchTick::ManagerDown(down) => {
            if let Some(round) = down.round {
                info!(
                    "arena match: round {} {} -> state {}",
                    arena.rounds.round,
                    if round.won { "won" } else { "lost" },
                    round.state.word()
                );
                // The manager's own menu is a separate screen; the
                // transition is only the one this tick's round end opened.
                arena.screen = Some(crate::arena_session::Screen::after(round.state));
            }
            arena.manager_screen = Some(down.screen);
            info!(
                "arena match: the manager fell (screen {}, fatal {})",
                down.screen, down.fatal
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Player;

    #[test]
    fn the_twelve_arenas_carry_their_tables() {
        assert_eq!(ARENA_MAPS[3].name, "arena4");
        assert_eq!(ARENA_MAPS[3].spawn, [-1500.0, 0.0, -450.0]);
        assert_eq!(ARENA_MAPS[1].view_amount.to_bits(), 0x3e99_999a);
        assert_eq!(ARENA_MAPS[3].view_amount.to_bits(), 0x3f19_999a);
        let late: Vec<_> = ARENA_MAPS.iter().map(|a| a.late_draw).collect();
        assert_eq!(late.iter().filter(|l| **l).count(), 4);
        assert!(!ARENA_MAPS[11].late_draw);
        let steady: Vec<usize> = (0..12).filter(|i| ARENA_MAPS[*i].steady_bed).collect();
        assert_eq!(steady, vec![7, 9, 11]);
        assert_eq!(arena_index("ArenaO3.rfc"), Some(9));
        assert_eq!(arena_index("arenahub"), None);
    }

    #[test]
    fn a_zero_roll_draws_the_last_outdoor_arena() {
        let mut picks = ArenaPicks::default();
        let mut roll = 0;
        assert_eq!(pick_arena(&mut picks, false, &mut roll), 11);
        assert_eq!(picks.variety, -1);
        assert_eq!(picks.outdoor, [4, 0, 0]);
        assert_eq!(picks.indoor, [0; 5]);
        assert_eq!(roll, 0xdc6d_ac1f);
    }

    #[test]
    fn the_beast_takes_arena5_or_arena7_indoors() {
        let draw = |start: u32| {
            let mut picks = ArenaPicks::default();
            let mut roll = start;
            let arena = pick_arena(&mut picks, true, &mut roll);
            (arena, picks.variety, roll)
        };
        assert_eq!(draw(16), (4, 1, 0x4e5b_0596));
        assert_eq!(draw(18), (6, 1, 0xf725_55c8));
        // The same draw outdoors does not look at the Beast.
        assert_eq!(draw(2), (10, -1, 0xb0d2_d438));
    }

    #[test]
    fn six_matches_never_repeat_recent_arenas_or_tracks() {
        let mut picks = ArenaPicks::default();
        let mut roll = 0;
        let mut drawn = Vec::new();
        for _ in 0..6 {
            let arena = pick_arena(&mut picks, false, &mut roll);
            let track = pick_track(&mut picks.tracks, &mut roll, false, false);
            let index = TRACKS.iter().position(|t| *t == track).unwrap();
            drawn.push((arena, index));
        }
        assert_eq!(
            drawn,
            vec![(11, 2), (10, 3), (8, 4), (3, 9), (7, 5), (5, 1)]
        );
        assert_eq!(picks.variety, -2);
        assert_eq!(picks.indoor, [5, 3, 0, 0, 0]);
        assert_eq!(picks.outdoor, [0, 1, 3]);
        assert_eq!(picks.tracks, [1, 5, 9, 4, 3]);
        assert_eq!(roll, 0x2630_fbe0);
    }

    #[test]
    fn a_track_skips_the_last_five() {
        let mut tracks = [0, 1, 2, 3, 4];
        let mut roll = 0;
        let track = pick_track(&mut tracks, &mut roll, false, false);
        assert_eq!(track.name, "Msc_Tourney01");
        assert_eq!(track.volume.to_bits(), 0x3e19_999a);
        assert_eq!(tracks, [9, 0, 1, 2, 3]);
        assert_eq!(roll, 0xdc6d_ac1f);
        let mut tracks = [0, 1, 2, 3, 4];
        let mut roll = 0;
        let alternative = pick_track(&mut tracks, &mut roll, false, true);
        assert_eq!(alternative.name, "Msc_Tourn01_DnB");
        assert_eq!(TRACKS[3].volume.to_bits(), 0x3d8f_5c29);
        assert_eq!(TRACKS[10].volume.to_bits(), 0x3df5_c28f);
        assert_eq!(ALTERNATIVE_TRACKS[7].volume.to_bits(), 0x3e4c_cccd);
    }

    #[test]
    fn a_start_draws_from_the_reversed_counter() {
        let mut picks = ArenaPicks::default();
        let start = start_match(&mut picks, 1, 0x1234_5678, false);
        assert_eq!(start.arena, 4);
        assert_eq!(start.track.name, "Msc_Arena3");
        assert_eq!(start.roll, 0x80b2_188e);
        assert_eq!(picks.variety, 1);
        assert_eq!(picks.indoor, [4, 0, 0, 0, 0]);
        assert_eq!(picks.tracks, [5, 0, 0, 0, 0]);
        // The Beast: arena5 from roll 16, its own track, the list untouched.
        let mut picks = ArenaPicks::default();
        let start = start_match(&mut picks, BEAST, 0x1000_0000, false);
        assert_eq!(start.arena, 4);
        assert_eq!(start.track, BEAST_TRACK);
        assert_eq!(start.track.volume.to_bits(), 0x3e99_999a);
        assert_eq!(start.roll, 0x4e5b_0596);
        assert_eq!(picks.tracks, [0; 5]);
        let alternative = start_match(&mut ArenaPicks::default(), BEAST, 0, true);
        assert_eq!(alternative.track.name, "Msc_ArenaO_DnB");
    }

    #[test]
    fn the_outdoor_bed_follows_rain_time_and_wind() {
        let steady = ARENA_MAPS[7];
        let windy = ARENA_MAPS[8];
        assert_eq!(outdoor_bed(&steady, 0.02, 1.0, 1.0), OutdoorBed::Silent);
        // 0.01 in single widens below the double 0.01: not raining.
        assert_eq!(outdoor_bed(&steady, 0.01, 0.4, 0.0), OutdoorBed::Night);
        assert_eq!(outdoor_bed(&steady, 0.0, 0.5, 0.0), OutdoorBed::Steady);
        assert_eq!(outdoor_bed(&windy, 0.0, 0.5, 0.05), OutdoorBed::Silent);
        // 0.1 in single widens above the double 0.1: windy.
        let OutdoorBed::Wind { gain, pitch } = outdoor_bed(&windy, 0.0, 0.5, 0.1) else {
            panic!("wind");
        };
        assert_eq!(
            (gain.to_bits(), pitch.to_bits()),
            (0x3b03_126f, 0x3f26_6666)
        );
        let OutdoorBed::Wind { gain, pitch } = outdoor_bed(&windy, 0.0, 0.9, 0.5) else {
            panic!("wind");
        };
        assert_eq!(
            (gain.to_bits(), pitch.to_bits()),
            (0x3d4c_cccd, 0x3f59_999a)
        );
    }

    #[test]
    fn a_day_runs_three_matches() {
        assert!(board_open(2));
        assert!(!board_open(3));
        let mut today = 0;
        count_returned_match(7, true, &mut today);
        assert_eq!(today, 0);
        count_returned_match(7, false, &mut today);
        count_returned_match(8, true, &mut today);
        assert_eq!(today, 2);
    }

    #[test]
    fn a_single_round_match_is_won_or_lost_at_once() {
        let mut rounds = MatchRounds::default();
        assert_eq!(rounds.finish_round(true), MatchState::Won);
        assert_eq!(rounds.finish_round(false), MatchState::Lost);
        assert_eq!(rounds.losses, 1);
        assert_eq!(MatchState::NextFight.word(), 8);
        assert_eq!(MatchState::Won.word(), 4);
        assert_eq!(MatchState::Lost.word(), 5);
        assert_eq!(MatchRounds::for_template(7), MatchRounds::elimination());
        assert_eq!(MatchRounds::for_template(5).lives, 3);
        assert_eq!(MatchRounds::for_template(1), MatchRounds::default());
        assert_eq!(ArenaMatch::begin(7).rounds.extra_rounds, 4);
    }

    #[test]
    fn elimination_needs_five_wins_in_a_row() {
        let mut rounds = MatchRounds::elimination();
        for round in 0..4 {
            assert_eq!(rounds.finish_round(true), MatchState::NextFight, "{round}");
            assert_eq!(
                continue_match(&mut rounds, MatchState::NextFight, false),
                Continue::NextRound
            );
        }
        assert_eq!(rounds.round, 4);
        assert_eq!(rounds.finish_round(true), MatchState::Won);
        assert_eq!(
            continue_match(&mut rounds, MatchState::Won, false),
            Continue::Hub
        );
        let mut rounds = MatchRounds::elimination();
        rounds.round = 2;
        assert_eq!(rounds.finish_round(false), MatchState::Lost);
    }

    #[test]
    fn reserve_wins_three_before_three_losses() {
        let mut rounds = MatchRounds::reserve();
        let fight = |rounds: &mut MatchRounds, won| {
            let state = rounds.finish_round(won);
            (state, continue_match(rounds, state, false))
        };
        assert_eq!(
            fight(&mut rounds, false),
            (MatchState::NextFight, Continue::NextRound)
        );
        assert_eq!(
            fight(&mut rounds, false),
            (MatchState::NextFight, Continue::NextRound)
        );
        assert_eq!(
            fight(&mut rounds, true),
            (MatchState::NextFight, Continue::NextRound)
        );
        assert_eq!(
            fight(&mut rounds, true),
            (MatchState::NextFight, Continue::NextRound)
        );
        assert_eq!((rounds.round, rounds.losses), (4, 2));
        assert_eq!(fight(&mut rounds, true), (MatchState::Won, Continue::Hub));
        let mut rounds = MatchRounds::reserve();
        for _ in 0..2 {
            fight(&mut rounds, false);
        }
        assert_eq!(rounds.finish_round(false), MatchState::Lost);
        // A dead manager always goes to the menu.
        assert_eq!(
            continue_match(&mut rounds, MatchState::NextFight, true),
            Continue::MainMenu
        );
    }

    #[test]
    fn a_match_ticks_after_its_first_second() {
        assert!(!tick_due(500, 1499));
        assert!(tick_due(500, 1500));
        // The stamp is widened, so a stamp near the top never wraps.
        assert!(!tick_due(u32::MAX, 0));
    }

    fn fighter(ours: bool, dead: bool) -> SceneFighter {
        SceneFighter {
            team: if ours { PLAYER_TEAM } else { 0 },
            dead,
            company: ours,
            resolved: false,
            health: if dead { 0.0 } else { 0.25 },
            max: 0.25,
            id: if ours { 2 } else { 3 },
            name: None,
            manager: false,
        }
    }

    fn teammate(dead: bool, health: f32) -> SceneFighter {
        SceneFighter {
            team: PLAYER_TEAM,
            dead,
            company: false,
            resolved: false,
            health,
            max: 0.25,
            id: 4,
            name: None,
            manager: false,
        }
    }

    fn manager(dead: bool, health: f32) -> SceneFighter {
        SceneFighter {
            team: PLAYER_TEAM,
            dead,
            company: true,
            resolved: false,
            health,
            max: 0.25,
            id: 0,
            name: None,
            manager: true,
        }
    }

    fn company(id: u8, name: &[u8], dead: bool, health: f32) -> SceneFighter {
        let mut bytes = [0u8; 16];
        let len = name.len().min(bytes.len());
        bytes[..len].copy_from_slice(&name[..len]);
        SceneFighter {
            team: PLAYER_TEAM,
            dead,
            company: true,
            resolved: false,
            health,
            max: 0.25,
            id,
            name: Some(bytes),
            manager: false,
        }
    }

    /// Call [`match_tick`] with the tests' clock and a collecting `remove`.
    #[allow(clippy::too_many_arguments)]
    fn run_tick(
        scene: &mut [SceneFighter],
        rounds: &mut MatchRounds,
        state: &mut MatchState,
        handled: &mut bool,
        physician: bool,
        log: &mut ArenaLog,
        deaths: &mut u32,
        removed: &mut Vec<(usize, u8)>,
        started: u32,
        now: u32,
    ) -> MatchTick {
        match_tick(
            scene,
            rounds,
            state,
            handled,
            physician,
            log,
            deaths,
            |index, id| removed.push((index, id)),
            started,
            now,
        )
    }

    #[test]
    fn a_tick_counts_who_stands() {
        let mut scene = [
            fighter(true, false),
            fighter(false, true),
            fighter(false, false),
        ];
        let counted = tally(&mut scene);
        assert_eq!(
            (counted.ours, counted.ours_standing, counted.theirs_standing),
            (1, 1, 1)
        );
        assert_eq!(counted.round_end(), None);
        scene[2].dead = true;
        assert_eq!(tally(&mut scene).round_end(), Some(true));
        scene[0].dead = true;
        let counted = tally(&mut scene);
        assert_eq!(counted.fallen, vec![0]);
        assert!(scene[0].resolved);
        assert_eq!(counted.round_end(), Some(false));
        // Resolved once.
        assert!(tally(&mut scene).fallen.is_empty());
        // Nobody of ours and nobody else standing: won.
        assert_eq!(tally(&mut []).round_end(), Some(true));
        let mut theirs = [fighter(false, false)];
        assert_eq!(tally(&mut theirs).round_end(), None);
    }

    #[test]
    fn a_match_tick_waits_for_the_first_second() {
        let mut scene = [company(2, b"Ash", true, 0.0)];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let mut removed = Vec::new();
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |index, id| removed.push((index, id)),
            500,
            1499,
        );
        assert_eq!(result, MatchTick::NotDue);
        assert_eq!(state, MatchState::Fighting);
        assert!(log.records.is_empty());
        assert!(removed.is_empty());
        assert!(!scene[0].resolved);
        // One millisecond later the same scene is counted and lost.
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |index, id| removed.push((index, id)),
            500,
            1500,
        );
        assert_eq!(
            result,
            MatchTick::RoundEnded(RoundEnd {
                won: false,
                state: MatchState::Lost,
            })
        );
        assert_eq!(removed, vec![(0, 2)]);
        assert_eq!(deaths, 1);
    }

    #[test]
    fn a_tick_counts_three_against_two_by_team_id() {
        let mut scene = [
            manager(false, 0.25),
            company(2, b"Ash", true, 0.0),
            teammate(false, 0.25),
            fighter(false, true),
            fighter(false, false),
        ];
        let counted = tally(&mut scene.clone());
        assert_eq!(
            (counted.ours, counted.ours_standing, counted.theirs_standing),
            (3, 2, 1)
        );
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let mut removed = Vec::new();
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |index, id| removed.push((index, id)),
            1000,
            2000,
        );
        assert_eq!(result, MatchTick::Nothing);
        assert_eq!(
            (state, rounds.round, rounds.losses),
            (MatchState::Fighting, 0, 0)
        );
        // The KO was still resolved even though it did not end the round.
        assert_eq!(removed, vec![(1, 2)]);
        assert_eq!(deaths, 1);
        assert_eq!(
            (log.records[0].id, log.records[0].words[0]),
            (LOG_DIED, 0x0068_7341)
        );
    }

    #[test]
    fn a_newly_ko_company_fighter_is_resolved_once() {
        let mut scene = [
            manager(false, 0.25),
            company(2, b"Ash", true, 0.0),
            fighter(false, false),
        ];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let mut removed = Vec::new();
        assert_eq!(
            run_tick(
                &mut scene,
                &mut rounds,
                &mut state,
                &mut handled,
                false,
                &mut log,
                &mut deaths,
                &mut removed,
                1000,
                2000,
            ),
            MatchTick::Nothing
        );
        assert_eq!(removed, vec![(1, 2)]);
        assert!(scene[1].resolved);
        assert_eq!((deaths, log.records.len()), (1, 1));
        // The record's resolved flag keeps the next tick from resolving it
        // or removing the employee a second time.
        assert_eq!(
            run_tick(
                &mut scene,
                &mut rounds,
                &mut state,
                &mut handled,
                false,
                &mut log,
                &mut deaths,
                &mut removed,
                1000,
                2000,
            ),
            MatchTick::Nothing
        );
        assert_eq!((deaths, log.records.len(), removed.len()), (1, 1, 1));
    }

    #[test]
    fn a_physician_saves_a_company_fighter_from_a_lethal_injury() {
        let mut scene = [
            company(2, b"Ash", true, -0.03),
            teammate(false, 0.25),
            fighter(false, false),
        ];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let mut removed = Vec::new();
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            true,
            &mut log,
            &mut deaths,
            |index, id| removed.push((index, id)),
            1000,
            2000,
        );
        assert_eq!(result, MatchTick::Nothing);
        assert!(removed.is_empty());
        assert_eq!(deaths, 0);
        assert_eq!((log.records[0].id, log.records[0].words[0]), (LOG_SAVED, 2));
        assert!(scene[0].resolved);
    }

    #[test]
    fn a_tick_loses_the_round_when_no_one_of_ours_stands() {
        let mut scene = [company(2, b"Ash", true, 0.0), fighter(false, false)];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let mut removed = Vec::new();
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |index, id| removed.push((index, id)),
            1000,
            2000,
        );
        assert_eq!(
            result,
            MatchTick::RoundEnded(RoundEnd {
                won: false,
                state: MatchState::Lost,
            })
        );
        assert_eq!(state, MatchState::Lost);
        assert_eq!(rounds.losses, 1);
        assert_eq!(removed, vec![(0, 2)]);
    }

    #[test]
    fn a_tick_wins_the_round_when_nobody_else_stands() {
        let mut scene = [manager(false, 0.25), fighter(false, true)];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |_, _| {},
            1000,
            2000,
        );
        assert_eq!(
            result,
            MatchTick::RoundEnded(RoundEnd {
                won: true,
                state: MatchState::Won,
            })
        );
        assert_eq!(state, MatchState::Won);
        assert!(log.records.is_empty());
    }

    #[test]
    fn a_won_round_of_elimination_continues() {
        let mut scene = [manager(false, 0.25), fighter(false, true)];
        let mut rounds = MatchRounds::elimination();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |_, _| {},
            1000,
            2000,
        );
        assert_eq!(
            result,
            MatchTick::RoundEnded(RoundEnd {
                won: true,
                state: MatchState::NextFight,
            })
        );
        assert_eq!(state, MatchState::NextFight);
        assert_eq!(
            continue_match(&mut rounds, state, false),
            Continue::NextRound
        );
    }

    #[test]
    fn a_fatal_manager_fall_loses_the_round_and_shows_screen_three() {
        let mut scene = [manager(true, 0.0), fighter(false, false)];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let mut removed = Vec::new();
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |index, id| removed.push((index, id)),
            1000,
            2000,
        );
        assert_eq!(
            result,
            MatchTick::ManagerDown(ManagerDown {
                fatal: true,
                screen: 3,
                ours_standing: 0,
                round: Some(RoundEnd {
                    won: false,
                    state: MatchState::Lost,
                }),
            })
        );
        assert!(handled);
        assert_eq!(state, MatchState::Lost);
        assert!(scene[0].resolved);
        // The manager's death wipes the career: nothing is logged for it.
        assert!(log.records.is_empty());
        assert_eq!(deaths, 0);
        assert!(removed.is_empty());
    }

    #[test]
    fn a_saved_manager_shows_screen_two() {
        let mut scene = [manager(true, -0.03), fighter(false, false)];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            true,
            &mut log,
            &mut deaths,
            |_, _| {},
            1000,
            2000,
        );
        assert_eq!(
            result,
            MatchTick::ManagerDown(ManagerDown {
                fatal: false,
                screen: 2,
                ours_standing: 0,
                round: Some(RoundEnd {
                    won: false,
                    state: MatchState::Lost,
                }),
            })
        );
        // A manager below 0.01 but above the physician's line was saved.
        assert_eq!((log.records[0].id, log.records[0].words[0]), (LOG_SAVED, 0));
    }

    #[test]
    fn a_manager_down_leaves_the_round_alone_while_a_teammate_stands() {
        let mut scene = [
            manager(true, 0.0),
            teammate(false, 0.25),
            fighter(false, false),
        ];
        let mut rounds = MatchRounds::default();
        let mut state = MatchState::Fighting;
        let mut handled = false;
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let result = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |_, _| {},
            1000,
            2000,
        );
        assert_eq!(
            result,
            MatchTick::ManagerDown(ManagerDown {
                fatal: true,
                screen: 3,
                ours_standing: 1,
                round: None,
            })
        );
        assert_eq!((state, rounds.losses), (MatchState::Fighting, 0));
        // The end screen is handled once; the dead manager is not shown
        // again on the next tick.
        let again = match_tick(
            &mut scene,
            &mut rounds,
            &mut state,
            &mut handled,
            false,
            &mut log,
            &mut deaths,
            |_, _| {},
            1000,
            2100,
        );
        assert_eq!(again, MatchTick::Nothing);
    }

    #[test]
    fn a_fall_is_dead_saved_or_survived() {
        // 0.01 in single is below the double 0.01.
        assert!(fatal(0.01, 0.25, false));
        assert!(!fatal(0.010_000_1, 0.25, false));
        // With a physician: −0.0375 of 0.25 is −0.15 in single, below −0.15.
        assert!(fatal(-0.0375, 0.25, true));
        assert!(!fatal(-0.0374, 0.25, true));
        assert_eq!(resolve_fall(-0.03, 0.25, true, 2), Fall::Saved);
        assert_eq!(resolve_fall(-0.04, 0.25, true, 2), Fall::Died);
        assert_eq!(resolve_fall(0.0, 0.25, false, 0), Fall::ManagerDied);
        assert_eq!(resolve_fall(0.2, 0.25, false, 2), Fall::Survived);
        assert_eq!(resolve_fall(f32::NAN, 0.25, false, 2), Fall::Survived);
        assert!(!fatal(f32::NAN, 0.25, true));
    }

    #[test]
    fn a_fall_is_logged() {
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let mut name = [0u8; 16];
        name[..3].copy_from_slice(b"Ash");
        assert!(log_fall(&mut log, Fall::Died, 3, Some(name), &mut deaths));
        assert_eq!(log.records[0].id, LOG_DIED);
        assert_eq!(log.records[0].words, [0x0068_7341, 0, 0, 0]);
        assert_eq!(deaths, 1);
        assert!(log_fall(&mut log, Fall::Saved, 3, None, &mut deaths));
        assert_eq!(log.records[1].id, LOG_SAVED);
        assert_eq!(log.records[1].words[0], 3);
        assert!(!log_fall(&mut log, Fall::Died, 4, None, &mut deaths));
        assert!(!log_fall(&mut log, Fall::Survived, 4, None, &mut deaths));
        assert_eq!((log.records.len(), deaths), (2, 1));
    }

    #[test]
    fn the_board_and_the_day_follow_the_entries() {
        use crate::arena::{Roster, RosterRow};
        let mut rows: Roster = Default::default();
        assert!(!any_entry(&rows));
        assert_eq!(next_event(&rows), None);
        assert!(!day_can_end(3, &rows, 0));
        assert!(day_can_end(3, &rows, 1));
        rows[2] = RosterRow {
            count: 1,
            ..RosterRow::default()
        };
        rows[4] = RosterRow {
            count: 2,
            ..RosterRow::default()
        };
        assert!(any_entry(&rows));
        assert_eq!(next_event(&rows), Some(2));
        assert!(event_pending(7, &rows));
        assert!(!event_pending(8, &rows));
        assert!(!day_can_end(7, &rows, 3));
        rows[2].team_id = 1;
        assert_eq!(next_event(&rows), Some(4));
        rows[4].team_id = 5;
        assert!(day_can_end(7, &rows, 0));
        assert_eq!(next_event(&rows), None);
        // Not a tournament day: a fought match ends it.
        assert!(!day_can_end(8, &rows, 0));
        let mut armed = true;
        assert!(!warn_manager(&mut armed, false));
        assert!(warn_manager(&mut armed, true));
        assert!(!armed && !warn_manager(&mut armed, true));
        assert_eq!(health_bar(65535).to_bits(), 0x3f80_0000);
        assert_eq!(health_bar(32768).to_bits(), 0x3f00_0080);
    }

    #[test]
    fn health_goes_back_as_a_word() {
        // 32767.5 ties to the even 32768.
        assert_eq!(health_word(0.125, 0.25), 32768);
        assert_eq!(health_word(0.25, 0.25), 65535);
        assert_eq!(health_word(0.0, 0.25), 0);
        assert_eq!(health_word(-0.1, 0.25), 0);
        assert_eq!(health_word(f32::NAN, 0.25), 0);
        // Twice the maximum keeps the low sixteen bits of 131070.
        assert_eq!(health_word(0.5, 0.25), 65534);
        assert_eq!(health_from_word(32768, 0.25).to_bits(), 0x3e00_0080);
        assert_eq!(health_from_word(65535, 0.25).to_bits(), 0x3e80_0000);
    }

    fn match_app(player_dead: bool, foe_dead: bool) -> App {
        let mut app = App::new();
        app.insert_resource(EffectClock { now_ms: 500 })
            .insert_resource(ArenaMatch::begin(1))
            .add_systems(Update, tick_arena_match);
        let mut player = Fighter::default();
        player.health.red = 0.0;
        let condition = |dead| {
            if dead {
                Condition::DEAD
            } else {
                Condition::NORMAL
            }
        };
        app.world_mut().spawn((
            Player,
            crate::player::PlayerBody::new(0.0),
            player,
            condition(player_dead),
        ));
        app.world_mut()
            .spawn((Fighter::default(), condition(foe_dead)));
        app
    }

    #[test]
    fn the_running_match_waits_a_second_then_ends_the_round() {
        let mut app = match_app(false, true);
        app.update();
        assert_eq!(app.world().resource::<ArenaMatch>().started_ms, Some(500));
        app.world_mut().resource_mut::<EffectClock>().now_ms = 1499;
        app.update();
        assert_eq!(
            app.world().resource::<ArenaMatch>().state,
            MatchState::Fighting
        );
        app.world_mut().resource_mut::<EffectClock>().now_ms = 1500;
        app.update();
        assert_eq!(app.world().resource::<ArenaMatch>().state, MatchState::Won);
        assert_eq!(app.world().resource::<ArenaMatch>().manager_screen, None);
    }

    #[test]
    fn a_dead_player_loses_the_round_and_shows_the_fatal_screen() {
        let mut app = match_app(true, false);
        app.update();
        app.world_mut().resource_mut::<EffectClock>().now_ms = 1500;
        app.update();
        let arena = app.world().resource::<ArenaMatch>();
        assert_eq!(arena.state, MatchState::Lost);
        assert_eq!(arena.manager_screen, Some(3));
        assert_eq!(arena.resolved.len(), 1);
        // The manager's death wipes the career: nothing is logged for it.
        assert!(arena.log.records.is_empty());
        assert_eq!(arena.deaths, 0);
    }

    #[test]
    fn the_running_match_exposes_the_continue_and_forfeit_actions() {
        let mut arena = ArenaMatch::begin(7);
        arena.rounds.round = 3;
        arena.state = MatchState::NextFight;
        assert_eq!(
            arena.end_action(false),
            crate::arena_session::EndAction::NextRound {
                round: 4,
                bracket: false
            }
        );
        arena.rounds.round = 4;
        arena.state = MatchState::Won;
        assert_eq!(
            arena.end_action(true),
            crate::arena_session::EndAction::ReturnToHub { tournament: true }
        );
        arena.manager_screen = Some(3);
        assert_eq!(
            arena.end_action(false),
            crate::arena_session::EndAction::MainMenu
        );
        assert_eq!(
            arena.forfeit(false),
            crate::arena_session::ForfeitResult::Ignored
        );
        arena.manager_screen = Some(2);
        assert_eq!(
            arena.forfeit(false),
            crate::arena_session::ForfeitResult::Return { tournament: false }
        );
    }

    #[test]
    fn a_forfeit_and_the_return_are_recorded() {
        let mut log = ArenaLog::default();
        assert!(!forfeit(&mut log, true, false, 3));
        assert!(forfeit(&mut log, false, true, 3));
        assert!(log.records.is_empty());
        assert!(forfeit(&mut log, false, false, 3));
        assert_eq!(
            (log.records[0].id, log.records[0].words[0]),
            (LOG_FORFEIT, 3)
        );

        let mut records = CompanyRecords::default();
        let mut log = ArenaLog::default();
        records.start(BEAST);
        records.start(1);
        assert_eq!(records.beasts_fought, 1);
        let paid = return_to_hub(&mut records, &mut log, Finish::Won, BEAST, false, 432);
        assert_eq!(paid.prize, 432);
        assert_eq!(
            (log.records[0].id, log.records[0].words[0]),
            (LOG_WON, BEAST)
        );
        assert_eq!((records.matches_won, records.beasts_defeated), (1, 1));
        let paid = return_to_hub(&mut records, &mut log, Finish::Forfeited, 1, false, 12);
        assert_eq!(paid.prize, 0);
        assert_eq!(log.records[1].id, LOG_FORFEIT);
        assert_eq!((records.matches_forfeited, records.matches_lost), (1, 1));
        // A tournament win pays nothing here and logs nothing.
        let paid = return_to_hub(&mut records, &mut log, Finish::Won, 7, true, 96);
        assert_eq!(paid.prize, 0);
        assert_eq!(log.records.len(), 2);
        assert_eq!(records.matches_won, 2);
        return_to_hub(&mut records, &mut log, Finish::Lost, 1, false, 12);
        assert_eq!(log.records[2].id, LOG_LOST);
        assert_eq!(records.matches_lost, 2);
        assert_eq!(CompanyRecords::LABELS[9], "Deaths");
        assert_eq!(records.values()[..3], [2, 2, 1]);
    }
}
