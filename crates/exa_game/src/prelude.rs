//! The story and practice helpers the original's prelude unit holds: the
//! rotating game-tips popup, the actor templates loaded from `actors.rpk`,
//! and the practice opponent's fixed traits, team and reaction delay.
//!
//! The popup holds two tables: eighteen rotating tips and six control
//! hints. Index 0 asks for a rotating tip, and only with the `ShowGameTips`
//! setting on; index `n` from 1 shows hint `n − 1` whatever the setting.
//! A rotating tip is drawn among the least-shown ones with the engine's
//! generator: the draw is the high word of the tie count times the stepped
//! state. The original then uses the draw itself as the tip's index (the
//! loop that walks the ties stores its tie counter, not the table
//! position), so the tip shown, and counted, is the draw-th of the table
//! whether or not it is one of the least shown. Shown counts start at one;
//! a count of zero goes to two, any other counts up. The lines are the
//! banner's ([`exa_ui::game_tips::TIPS`] and
//! [`exa_ui::game_tips::HINTS`]); [`Shown::text`] picks one. The
//! game shows them through `Step::Tip` (the knock-out and death lines, the
//! practice ladder's round and win lines); a new story shows none.
//!
//! The cached template reader looks for a cached record whose own name
//! matches (sixteen bytes, ASCII letters case-blind), reads the template
//! from the archive otherwise, gives a nameless one the asked-for name and
//! caches a copy. The same reader without caching (the practice spawn)
//! only reads and names. The unit's second reader, which builds a template
//! together with its object chunks, is not reproduced. Evidence: the
//! prelude unit's routines (SG_Prelude_Main) in the assembly.
//!
//! A template's item references with the local bit name the items of the
//! database that follows its record in the entry (its own items), the
//! others the game's: [`template_item`]. The character builder routes them
//! so (the local bit to the character's own database, the world bit to the
//! world's, the rest to the game's), so the practice opponent wears its own
//! trousers, shirt and sandals rather than the game items of those numbers.

use exa_assets::{Archive, Game, PackKind};
use exa_formats::characters::{CharacterRecord, ID_MASK, LOCAL_ITEM};
use exa_formats::objdb::Item;
use exa_ui::game_tips as tip_banner;

use bevy::prelude::{Res, ResMut};

use crate::hw_config::HardwareConfig;

/// How many tips the popup rotates through.
pub const TIPS: usize = 18;
/// How many control hints the popup shows by index.
pub const HINTS: usize = 6;
/// The actor library inside the resource pack.
pub const ACTOR_PACK: &str = "actors.rpk";
/// The role the practice opponent is put on (the arena's Unaligned role).
pub const PRACTICE_TEAM: u32 = 0x10;
/// The practice opponent's reaction delay after the fight starts, in
/// milliseconds.
pub const PRACTICE_REACTION_MS: u32 = 3000;
/// Where the original's practice spawn places the opponent, in its level
/// coordinates (metres), with no turn. Not wired: Exarust's duel placement
/// stands in for it.
pub const PRACTICE_SPOT: [f32; 3] = [0.0, 5.0, 107.0];

/// What the popup shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// A rotating tip, by its table position.
    Tip(usize),
    /// A control hint, by its table position.
    Hint(usize),
}

impl Shown {
    /// The banner's line for what the popup shows.
    pub fn text(self) -> Option<&'static str> {
        banner_text(self)
    }
}

/// The original's generator step.
fn step(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    *state
}

/// The tips popup's state: the window exists once shown, the setting, the
/// shown counts and the generator.
#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq, Eq)]
pub struct Tips {
    /// `ShowGameTips`; automatic tips need it.
    pub show_game_tips: bool,
    /// Whether the popup has been created (its counts were initialised).
    pub created: bool,
    /// One count per tip, all one when the popup is created.
    pub shown: [u32; TIPS],
    /// The generator state.
    pub roll: u32,
    /// What is on screen, if anything.
    pub current: Option<Shown>,
}

impl Default for Tips {
    fn default() -> Self {
        Self {
            show_game_tips: true,
            created: false,
            shown: [0; TIPS],
            roll: 0,
            current: None,
        }
    }
}

impl Tips {
    /// Creates the popup: its counts all start at one.
    fn create(&mut self) {
        self.shown = [1; TIPS];
        self.created = true;
    }

    /// Shows a tip as the original's show routine does: index 0 is the
    /// automatic rotating tip, gated by the setting; index `n` from 1 is
    /// control hint `n − 1`, whatever the setting (an index past the hints
    /// shows nothing here; the original reads past its table).
    pub fn show(&mut self, index: usize) -> Option<Shown> {
        if index == 0 && !self.show_game_tips {
            return None;
        }
        if !self.created {
            self.create();
        }
        if index != 0 {
            if index > HINTS {
                return None;
            }
            let shown = Shown::Hint(index - 1);
            self.current = Some(shown);
            return Some(shown);
        }
        let least = *self.shown.iter().min()?;
        let ties = self.shown.iter().filter(|&&count| count == least).count() as u64;
        // The draw is the index: the original keeps its tie counter.
        let chosen = ((u64::from(step(&mut self.roll)) * ties) >> 32) as usize;
        if self.shown[chosen] == 0 {
            self.shown[chosen] = 2;
        } else {
            self.shown[chosen] += 1;
        }
        let shown = Shown::Tip(chosen);
        self.current = Some(shown);
        Some(shown)
    }

    /// Shows the least-shown tip: [`Tips::show`] with index 0.
    pub fn show_automatic(&mut self) -> Option<Shown> {
        self.show(0)
    }

    /// Hides the popup, leaving the counts as they are.
    pub fn hide(&mut self) {
        self.current = None;
    }
}

/// Keeps the popup's `ShowGameTips` setting in step with the options, as
/// the original reads the setting each time it shows a tip. A missing
/// options resource leaves the popup as it is.
pub fn sync_tips_setting(config: Option<Res<HardwareConfig>>, mut tips: ResMut<Tips>) {
    let Some(config) = config else {
        return;
    };
    tips.show_game_tips = config.live.show_tips;
}

/// The text the popup shows for a choice: the tip from [`TIPS`] or the
/// indexed message from [`HINTS`]. An index past a table shows nothing
/// (the original reads past it).
#[must_use]
pub fn banner_text(shown: Shown) -> Option<&'static str> {
    match shown {
        Shown::Tip(index) => tip_banner::tip_text(index),
        Shown::Hint(index) => tip_banner::hint_text(index),
    }
}

/// The text the running popup shows. The flow's [`crate::flow::Step::Tip`]
/// writes it; the draw system paints it.
#[derive(bevy::prelude::Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct TipBannerText(pub String);

/// Draws the popup's banner through the GUI draw path: the translucent box
/// [`tip_banner::GameTips::draw`] places and the text in the interface's base
/// font. Colour of the box is not traced (guess: black at the drawn alpha).
pub struct TipsScreenPlugin;

impl bevy::prelude::Plugin for TipsScreenPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<TipBannerText>()
            .add_systems(bevy::prelude::Update, draw_tips);
    }
}

fn draw_tips(
    banner: bevy::prelude::Res<TipBannerText>,
    textures: Option<bevy::prelude::Res<exa_ui::gui_draw::GuiTextures>>,
    windows: bevy::prelude::Query<
        &bevy::prelude::Window,
        bevy::prelude::With<bevy::window::PrimaryWindow>,
    >,
    mut frames: bevy::prelude::ResMut<exa_ui::gui_draw::GuiFrames>,
) {
    if banner.0.is_empty() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as i32;
    let height = window.physical_height() as i32;
    if width <= 0 || height <= 0 {
        return;
    }
    let unit = exa_ui::scale::gui_scale(height.max(0) as u32, 0) as i32;
    let font = &exa_ui::gui_font::FONTBASE_24;
    let text_width = font.text_width(banner.0.as_bytes());
    let place = tip_banner::GameTips {
        text: banner.0.clone(),
    }
    .draw(width, unit, text_width);
    let Some(textures) = textures else {
        return;
    };
    let Some(texture) = textures.get(font.texture) else {
        return;
    };
    use exa_ui::gui_draw::{GuiFrame, GuiLayer, LayerTexture, text_batch};
    use exa_ui::skin::SkinBatch;
    let mut frame = GuiFrame::new();
    let [x, y] = place.box_at;
    let [w, h] = place.box_size;
    let mut box_batch = SkinBatch::new();
    box_batch.select(1, 0, false);
    box_batch.set_ink([0, 0, 0, place.alpha]);
    box_batch.begin();
    box_batch.vertex(x as f32, y as f32);
    box_batch.vertex(x as f32, (y + h) as f32);
    box_batch.vertex((x + w) as f32, (y + h) as f32);
    box_batch.vertex((x + w) as f32, y as f32);
    box_batch.end_quads();
    frame.push(GuiLayer::new(box_batch, LayerTexture::None));
    let baseline = place.text_at[1] + font.ascent;
    let quads = exa_ui::gui_text::draw_line(font, banner.0.as_bytes(), place.text_at[0], baseline);
    let size = (texture.width as u16, texture.height as u16);
    frame.push(GuiLayer::new(
        text_batch(font, &quads, [255, 255, 255, 255], size),
        LayerTexture::Named(font.texture),
    ));
    frames.push(frame);
}

/// Every named actor template in an archive, with the items its own
/// database holds.
#[derive(Debug, Clone, Default)]
pub struct ActorTemplates {
    entries: Vec<(String, CharacterRecord, Vec<Item>)>,
}

/// The item a template's reference names: one with [`LOCAL_ITEM`] among
/// the template's own items (`own`), any other among the game's
/// (`global`), each by the id without its database bits.
pub fn template_item<'a>(own: &'a [Item], global: &'a [Item], id: u32) -> Option<&'a Item> {
    let items = if id & LOCAL_ITEM != 0 { own } else { global };
    items.iter().find(|item| item.id == id & ID_MASK)
}

impl ActorTemplates {
    /// Reads every entry's record; an entry that does not parse is left
    /// out. No actors, or no archive, is an empty library.
    pub fn load(game: &Game) -> Self {
        let Some(resource) = game.0.pack(PackKind::Resource) else {
            return Self::default();
        };
        let Some(bytes) = resource.get(ACTOR_PACK) else {
            return Self::default();
        };
        let Ok(archive) = Archive::parse(bytes) else {
            return Self::default();
        };
        let entries = archive
            .entries()
            .iter()
            .filter_map(|entry| {
                let bytes = archive.bytes(entry);
                let record = CharacterRecord::parse(0, bytes).ok()?;
                // The database after the record: what its local ids name.
                let items = exa_formats::characters::actor_items(bytes).unwrap_or_default();
                Some((entry.name.clone(), record, items))
            })
            .collect();
        Self { entries }
    }

    /// An empty library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a library from records, for callers that already hold them.
    pub fn from_records(entries: impl IntoIterator<Item = (String, CharacterRecord)>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(name, record)| (name, record, Vec::new()))
                .collect(),
        }
    }

    /// Builds a library from records with their own items.
    pub fn from_records_with_items(
        entries: impl IntoIterator<Item = (String, CharacterRecord, Vec<Item>)>,
    ) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    /// The template named `name`, exactly.
    pub fn get(&self, name: &str) -> Option<&CharacterRecord> {
        self.entries
            .iter()
            .find(|(entry, _, _)| entry == name)
            .map(|(_, record, _)| record)
    }

    /// The items of the template named `name`'s own database (empty when
    /// it has none or there is no such template).
    pub fn own_items(&self, name: &str) -> &[Item] {
        self.entries
            .iter()
            .find(|(entry, _, _)| entry == name)
            .map_or(&[], |(_, _, items)| items.as_slice())
    }

    /// How many templates were read.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the library is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The template reader with the original's two modes: the cached one
/// first looks among the records it handed out, the plain one always reads.
#[derive(Debug, Clone, Default)]
pub struct ActorCache {
    /// The library, read on the first lookup.
    library: Option<ActorTemplates>,
    /// Copies of the records the cached reader handed out, in order.
    cached: Vec<CharacterRecord>,
}

/// The original's name compare: the first sixteen bytes, zero padded, with
/// `a`…`z` folded to upper case.
fn same_name(a: &str, b: &str) -> bool {
    let pad = |s: &str| {
        let mut out = [0u8; 16];
        let bytes = s.as_bytes();
        let n = bytes.len().min(16);
        out[..n].copy_from_slice(&bytes[..n]);
        out.map(|c| c.to_ascii_uppercase())
    };
    pad(a) == pad(b)
}

impl ActorCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// A template by name. With `cached`, a cached record whose own name
    /// matches is returned, and a record read is cached. Either way a
    /// template whose own name is empty takes the asked-for name. The
    /// library is read once.
    pub fn template(&mut self, game: &Game, name: &str, cached: bool) -> Option<CharacterRecord> {
        self.template_with(name, cached, || ActorTemplates::load(game))
    }

    /// The own items of the template entry `name` ([`ActorTemplates::own_items`]),
    /// reading the library once.
    pub fn own_items(&mut self, game: &Game, name: &str) -> Vec<Item> {
        self.library
            .get_or_insert_with(|| ActorTemplates::load(game))
            .own_items(name)
            .to_vec()
    }

    /// [`ActorCache::template`] with the library read by `load` when the
    /// cache has none yet.
    pub fn template_with(
        &mut self,
        name: &str,
        cached: bool,
        load: impl FnOnce() -> ActorTemplates,
    ) -> Option<CharacterRecord> {
        if cached
            && let Some(record) = self
                .cached
                .iter()
                .find(|record| same_name(&record.name, name))
        {
            return Some(record.clone());
        }
        let mut record = self.library.get_or_insert_with(load).get(name).cloned()?;
        if record.name.is_empty() {
            record.name = name.to_string();
        }
        if cached {
            self.cached.push(record.clone());
        }
        Some(record)
    }
}

/// What the practice spawn changes in a template and gives the motile.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeOpponent {
    /// The template with the practice traits set.
    pub record: CharacterRecord,
    /// Its role id: the arena's Unaligned, which is team `0x10`.
    pub team: u32,
    /// Its reaction delay after the fight starts, in milliseconds.
    pub reaction_ms: u32,
}

/// The practice opponent from `record` (read without the cache): its first
/// three personality traits are set to −100, 100 and 100, it goes on team
/// `0x10` and it waits three seconds after the fight starts.
pub fn practice_opponent(mut record: CharacterRecord) -> PracticeOpponent {
    record.traits[0] = -100.0;
    record.traits[1] = 100.0;
    record.traits[2] = 100.0;
    PracticeOpponent {
        record,
        team: PRACTICE_TEAM,
        reaction_ms: PRACTICE_REACTION_MS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hint_is_one_based_and_does_not_count() {
        let mut tips = Tips::default();
        assert_eq!(tips.shown, [0; TIPS]);
        assert_eq!(tips.show(3), Some(Shown::Hint(2)));
        assert!(tips.created);
        assert_eq!(tips.current, Some(Shown::Hint(2)));
        assert_eq!(tips.shown, [1; TIPS], "hints do not count");
        assert_eq!(tips.show(HINTS), Some(Shown::Hint(HINTS - 1)));
        tips.hide();
        assert_eq!(tips.current, None);
        assert_eq!(tips.shown, [1; TIPS]);
    }

    #[test]
    fn the_rotating_tip_is_the_draw_itself() {
        let mut tips = Tips::default();
        // Eighteen ties at one: state 1 after one step from 0, so the draw
        // is (1 * 18) >> 32 = 0.
        assert_eq!(tips.show_automatic(), Some(Shown::Tip(0)));
        assert_eq!(tips.shown[0], 2);
        // Seventeen ties left; the state 0x0808_8406 times 17 is
        // 0x8890_C466, high word 0: tip 0 again, although it is no longer
        // among the least shown.
        assert_eq!(tips.roll, 1);
        assert_eq!(tips.show_automatic(), Some(Shown::Tip(0)));
        assert_eq!(tips.shown[0], 3);
    }

    #[test]
    fn the_drawn_index_need_not_be_a_least_shown_tip() {
        let mut tips = Tips {
            created: true,
            shown: [3; TIPS],
            ..Tips::default()
        };
        tips.shown[5] = 1;
        tips.shown[9] = 1;
        // Two ties: draw 0 or 1, and that is the index counted.
        let mut state = tips.roll;
        let draw = ((u64::from(step(&mut state)) * 2) >> 32) as usize;
        assert_eq!(tips.show_automatic(), Some(Shown::Tip(draw)));
        assert_eq!(tips.shown[draw], 4);
        assert_eq!((tips.shown[5], tips.shown[9]), (1, 1));
        // A zero count goes to two.
        let mut fresh = Tips {
            created: true,
            ..Tips::default()
        };
        assert_eq!(fresh.show_automatic(), Some(Shown::Tip(0)));
        assert_eq!(fresh.shown[0], 2);
    }

    #[test]
    fn the_setting_gates_only_automatic_tips() {
        let mut tips = Tips {
            show_game_tips: false,
            ..Tips::default()
        };
        assert_eq!(tips.show_automatic(), None);
        assert!(!tips.created, "a refused automatic tip creates nothing");
        assert_eq!(tips.show(1), Some(Shown::Hint(0)), "a hint still shows");
    }

    #[test]
    fn the_options_show_tips_setting_reaches_the_popup() {
        use bevy::prelude::App;
        let mut app = App::new();
        let mut config = HardwareConfig::default();
        config.live.show_tips = false;
        app.insert_resource(config).init_resource::<Tips>();
        app.add_systems(bevy::prelude::Update, sync_tips_setting);
        app.update();
        assert!(!app.world().resource::<Tips>().show_game_tips);
        app.world_mut()
            .resource_mut::<HardwareConfig>()
            .live
            .show_tips = true;
        app.update();
        assert!(app.world().resource::<Tips>().show_game_tips);
    }

    #[test]
    fn an_index_past_the_hints_shows_nothing() {
        let mut tips = Tips::default();
        assert_eq!(tips.show(HINTS + 1), None);
        assert_eq!(tips.current, None);
    }

    /// The knock-out shows hint 2 and the death hint 3, whatever the
    /// setting; the first automatic tip is the table's draw-th line.
    #[test]
    fn the_shown_tip_names_the_banner_line() {
        let mut tips = Tips {
            show_game_tips: false,
            ..Tips::default()
        };
        assert_eq!(
            tips.show(2).and_then(Shown::text),
            Some("You have been defeated.")
        );
        assert_eq!(tips.show(3).and_then(Shown::text), Some("You have died."));
        assert_eq!(tips.show(0), None, "the setting gates the rotating tip");
        tips.show_game_tips = true;
        let shown = tips.show(0).unwrap();
        assert!(matches!(shown, Shown::Tip(_)));
        assert!(shown.text().is_some());
        assert_eq!(Shown::Tip(18).text(), None);
    }

    #[test]
    fn names_compare_on_sixteen_bytes_without_letter_case() {
        assert!(same_name("Opponent01", "opponent01"));
        assert!(!same_name("opponent01", "opponent02"));
        assert!(same_name("abcdefghijklmnopXYZ", "ABCDEFGHIJKLMNOP"));
        assert!(!same_name("a[", "A{"), "only letters fold");
    }

    #[test]
    fn an_actor_library_finds_entries_by_their_exact_name() {
        let mut record = CharacterRecord::reset();
        record.name = "Opponent".into();
        let library = ActorTemplates::from_records([("opponent01".to_string(), record.clone())]);
        assert_eq!(library.len(), 1);
        assert_eq!(
            library.get("opponent01").map(|r| r.name.as_str()),
            Some("Opponent")
        );
        assert!(library.get("Opponent").is_none(), "the entry name is exact");
        assert!(library.get("missing").is_none());
    }

    #[test]
    fn the_cache_matches_a_record_by_its_own_name() {
        let mut named = CharacterRecord::reset();
        named.name = "Opponent".into();
        let nameless = CharacterRecord::reset();
        let mut cache = ActorCache {
            library: Some(ActorTemplates::from_records([
                ("opponent01".to_string(), named),
                ("opponent02".to_string(), nameless),
            ])),
            cached: Vec::new(),
        };
        let none = ActorTemplates::new;
        // A record with its own name is cached under it, so asking by the
        // entry name reads and caches it again.
        cache.template_with("opponent01", true, none).unwrap();
        cache.template_with("opponent01", true, none).unwrap();
        assert_eq!(cache.cached.len(), 2);
        assert!(cache.template_with("OPPONENT", true, none).is_some());
        assert_eq!(cache.cached.len(), 2, "found by the record's name");
        // A nameless one takes the asked-for name and is found by it.
        let got = cache.template_with("opponent02", true, none).unwrap();
        assert_eq!(got.name, "opponent02");
        cache.template_with("opponent02", true, none).unwrap();
        assert_eq!(cache.cached.len(), 3);
        // Without the cache nothing is kept.
        assert_eq!(
            cache.template_with("opponent02", false, none).unwrap().name,
            "opponent02"
        );
        assert_eq!(cache.cached.len(), 3);
    }

    #[test]
    fn the_practice_opponent_gets_its_traits_team_and_delay() {
        let record = CharacterRecord::reset();
        let opponent = practice_opponent(record);
        assert_eq!(opponent.record.traits[..3], [-100.0, 100.0, 100.0]);
        assert_eq!(opponent.team, 0x10);
        assert_eq!(opponent.reaction_ms, 3000);
    }

    #[test]
    fn a_shown_tip_or_hint_names_its_banner_text() {
        assert_eq!(
            banner_text(Shown::Tip(0)),
            Some(exa_ui::game_tips::FIRST_TIP)
        );
        assert_eq!(banner_text(Shown::Tip(1)), Some(exa_ui::game_tips::TIPS[1]));
        assert_eq!(banner_text(Shown::Tip(18)), None);
        assert_eq!(
            banner_text(Shown::Hint(0)),
            Some(exa_ui::game_tips::HINTS[0])
        );
        assert_eq!(
            banner_text(Shown::Hint(5)),
            Some(exa_ui::game_tips::HINTS[5])
        );
        assert_eq!(banner_text(Shown::Hint(6)), None);
        // An automatic draw picks a tip and gives it text.
        let mut tips = Tips::default();
        let shown = tips.show_automatic().unwrap();
        assert!(banner_text(shown).is_some());
    }

    #[test]
    fn the_shipped_actor_library_has_opponents() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_rpk::GameData::open(&dir) else {
            return;
        };
        let game = Game(std::sync::Arc::new(data));
        let library = ActorTemplates::load(&game);
        assert_eq!(library.len(), 21);
        assert!(library.get("opponent01").is_some());
        let mut cache = ActorCache::new();
        let first = cache.template(&game, "opponent01", true).unwrap();
        let again = cache.template(&game, "opponent01", true).unwrap();
        assert_eq!(first, again);
        assert!(cache.template(&game, "missing", true).is_none());
    }

    fn named_item(id: u32, worn: &str) -> Item {
        use exa_formats::objdb;
        Item {
            id,
            name: worn.into(),
            parent: 0,
            model: String::new(),
            model_flags: 0,
            worn_model: worn.into(),
            appearance: None,
            slot_layers: [0; objdb::SLOTS],
            slot_words: [0; objdb::SLOTS],
            occupancy: [0; objdb::OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
            volume: None,
            volume_script: None,
            rank_word: 0,
            body_rank: 0,
            loadout_rank: 0,
            armour_weight: 0.0,
            armour_type: 0,
            container: None,
            random_table: None,
            has_body: false,
            has_weapon: false,
            has_shield: false,
        }
    }

    #[test]
    fn a_local_reference_names_the_template_s_own_item() {
        let own = [named_item(2, "Trouser_A"), named_item(3, "Shirty_A")];
        let global = [named_item(2, "Gloves"), named_item(3, "Helm")];
        let worn = |id| template_item(&own, &global, id).map(|i| i.worn_model.as_str());
        assert_eq!(worn(LOCAL_ITEM | 2), Some("Trouser_A"));
        assert_eq!(worn(LOCAL_ITEM | 3), Some("Shirty_A"));
        assert_eq!(worn(2), Some("Gloves"), "a plain id is the game's item");
        assert_eq!(worn(LOCAL_ITEM | 4), None, "no fallback to the game's");
        let library = ActorTemplates::from_records_with_items([(
            "opponent01".to_string(),
            CharacterRecord::reset(),
            own.to_vec(),
        )]);
        assert_eq!(library.own_items("opponent01").len(), 2);
        assert!(library.own_items("missing").is_empty());
    }

    /// The first practice opponent wears its own trousers (which cover the
    /// groin, so it takes no underwear), long-sleeved shirt and sandals:
    /// local items 2, 3 and 4 of its entry, not the game's items 2 to 4
    /// (gloves, a helm and boots).
    #[test]
    fn the_first_practice_opponent_wears_its_own_trousers() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_rpk::GameData::open(&dir) else {
            return;
        };
        let game = Game(std::sync::Arc::new(data));
        let global = game
            .0
            .pack(PackKind::Resource)
            .and_then(|p| p.get("objdb.rdb"))
            .and_then(|b| exa_formats::rdb::Database::parse(b).ok())
            .map(|db| exa_formats::objdb::items(&db))
            .unwrap_or_default();
        let mut cache = ActorCache::new();
        let record = cache.template(&game, "opponent01", false).unwrap();
        let own = cache.own_items(&game, "opponent01");
        let ids: Vec<u32> = record.worn.iter().map(|w| w.id).collect();
        assert_eq!(ids, [LOCAL_ITEM | 2, LOCAL_ITEM | 3, LOCAL_ITEM | 4]);
        let worn: Vec<&str> = ids
            .iter()
            .filter_map(|&id| template_item(&own, &global, id))
            .map(|item| item.worn_model.as_str())
            .collect();
        assert_eq!(worn, ["Trouser_A", "Shirty_A", "Sandals_B"]);
        assert!(
            ids.iter()
                .filter_map(|&id| template_item(&own, &global, id))
                .any(|item| item.covers_groin())
        );
    }
}
