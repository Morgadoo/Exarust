//! The character creator as a running screen: the story's background
//! picker and the creation form, opened from the main menu, drawn through
//! the GUI draw path with the shown palette and the line skin, taking the
//! pointer and keys, and starting the game with the record it builds.
//!
//! The menu opens it as the original's menu shows route it: the title's
//! BEGIN shows the background picker (`TClassSelect`), whose confirm opens
//! the creator for that background (screen mode 4, story game mode 4); the
//! practice page's NOVICE and EXPERT and the in-game NEW GAME open the
//! free creator in practice mode (2); the arena's NEW CAREER opens it in
//! arena mode (8) and VERSUS in versus mode (0x1010). The arena's recruit
//! (screen mode 10) is reached only with `--open-creator recruit`: its
//! caller, the arena's company form, is not wired.
//!
//! The form ([`Creator`]) fills the window: the creation form's seven
//! records ([`exa_ui::creation_forms::CREATION_RECORDS`]), the preview
//! viewport ([`crate::create::preview_frame`], drawn by
//! [`crate::creator_view`]), the body editor at 4, 60 (the physique picker
//! at 8, 64, the tone picker at 8, 328 and its twelve records) and the hair
//! editor at the form's width less 176, 32 (the hair-colour picker at 4,
//! 24 with its attached trackbar, the style list under it and its seven
//! records). The records are laid out at unit 4 and scaled to the live GUI
//! unit as the original's rescale scales them (`v / 4 · unit`). The
//! controls' answers run the original's callbacks
//! ([`crate::create`]): the sex radios change the profession word, the
//! Height trackbar settles `(0.5 − v) · 2`, Age copies its value, Pitch is
//! `v · 0.17 + 0.9`, the voice buttons step the voice and the facial-hair
//! buttons the variant, the radios pick the hair set, the pickers write
//! the physique point, the skin words and the hair bytes. NEXT asks for a
//! name ([`crate::create::submit_creation`]) and shows the equipment page;
//! its final button commits: the record is written
//! ([`crate::create::write_created_record`]), kept as [`CreatedCharacter`]
//! for the player's spawn, and the story or practice starts through the
//! flow. BACK goes back a page, or leaves to the menu.
//!
//! `--open-creator <mode>` opens it once the load ends ([`OpenCreatorOnLoad`]).
//!
//! Guesses (labelled where they are made): the creation form fills the
//! window (its 0x800000 attribute is read so); the compact layout is taken
//! when the design height is under 768; the picker markers are a small
//! square; the image buttons' icons are the creator sheet's cells, the
//! mirrored style flipping them; the hair list shows each style's entry
//! name where the original renders an icon; the equipment page's kit lists
//! and skills panel are drawn as empty panels; the arena career, versus
//! lobby and recruit commits, the power tree and the voice previews are not
//! wired.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_formats::characters::{CharacterRecord, Pools};
use exa_ui::creation_forms::{
    self, BACKGROUND_DESCRIPTIONS, BACKGROUND_NAMES, BASE_COMPACT_HEIGHT, BASE_RECORDS, BASE_SIZE,
    CREATION_RECORDS, HAIR_COMPACT_HEIGHT, HAIR_RECORDS, HAIR_SIZE, base, creation, hair,
};
use exa_ui::gui_controls::{self, Control, ControlEvent, Message, message, state};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTexture, GuiTextures, LayerTexture, text_batch};
use exa_ui::gui_font::GuiFont;
use exa_ui::gui_text::draw_line;
use exa_ui::line_skin::shown_palette;
use exa_ui::menu::MenuShow;
use exa_ui::picker_fields::{Picker, physique_cross};
use exa_ui::pickers::{FieldVertex, HairColPicker, PickPoint, TonePicker};
use exa_ui::skin::{SkinBatch, SkinPalette};

use crate::create::{
    self, BODY_EDITOR_AT, CreationGear, CreationScreenPage, GearSlot, HAIR_EDITOR_RIGHT,
    HAIR_EDITOR_STYLE, HAIR_EDITOR_TOP, NameWarning, PageDestination, SubmitInput,
};
use crate::flow::{Background, Flow, FlowSteps, Phase};
use crate::gui_paint::{self, Frame, Painted};
use crate::menu_pages::MenuCommand;
use crate::menu_screen::{MenuChoice, MenuScreenSystems};
use crate::practice::Ladder;

/// The story background picker's portrait strip.
pub const STRIP: &str = exa_ui::class_select::STRIP;
/// The creator's icon sheet.
pub const ICONS: &str = create::CREATION_ICON_SHEET;
/// The creation form's attribute word: a client fill and 0x800000.
pub const CREATION_ATTRIBUTES: u32 = 0x80_0020;
/// The editors' attribute word: the base form constructor's.
pub const EDITOR_ATTRIBUTES: u32 = gui_paint::FORM_ATTRIBUTES;
/// The physique and tone pickers' places in the body editor (the tone
/// picker's height is 112 in the compact layout).
pub const PHYSIQUE_AT: (i32, i32) = (8, 0x40);
pub const TONE_AT: (i32, i32) = (8, 0x148);
/// The hair-colour picker's place in the hair editor.
pub const HAIR_PICKER_AT: (i32, i32) = (4, 0x18);
/// The design height under which the compact layout is taken [guess: the
/// original's short-layout flag is set elsewhere].
pub const COMPACT_BELOW: i32 = 768;
/// The marker square's half side, in pixels [guess].
const MARKER_HALF: f32 = 3.0;

/// What the creator is opened for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatorEntry {
    /// The screen mode: 4 story, 10 recruit, else the free creator.
    pub screen_mode: i32,
    /// The game mode: 2 practice, 4 story, 8 arena, 0x1010 versus.
    pub game_mode: i32,
    /// The story background (0 unknown, 1 knight, 2 proctor, 3 villager).
    pub background: i32,
    /// The practice ladder a practice commit starts.
    pub ladder: Option<Ladder>,
}

impl CreatorEntry {
    /// The free creator in practice mode on `ladder`.
    pub fn practice(ladder: Ladder) -> Self {
        Self {
            screen_mode: 0,
            game_mode: 2,
            background: 0,
            ladder: Some(ladder),
        }
    }

    /// The story creator for a background.
    pub fn story(background: i32) -> Self {
        Self {
            screen_mode: 4,
            game_mode: 4,
            background,
            ladder: None,
        }
    }

    /// The free creator in the arena's mode (the new career's manager).
    pub fn arena() -> Self {
        Self {
            screen_mode: 0,
            game_mode: 8,
            background: 0,
            ladder: None,
        }
    }

    /// The arena's recruit.
    pub fn recruit() -> Self {
        Self {
            screen_mode: 10,
            game_mode: 8,
            background: 0,
            ladder: None,
        }
    }

    /// The free creator in versus mode.
    pub fn versus() -> Self {
        Self {
            screen_mode: 0,
            game_mode: 0x1010,
            background: 0,
            ladder: None,
        }
    }

    /// The entry a `--open-creator` word names: `story` (or `story:N` for
    /// background N), `practice`, `expert`, `arena`, `recruit` or `versus`.
    pub fn parse(word: &str) -> Option<Self> {
        let (name, background) = match word.split_once(':') {
            Some((name, index)) => (name, index.parse::<i32>().ok()?),
            None => (word, 0),
        };
        Some(match name {
            "story" if (0..4).contains(&background) => Self::story(background),
            "practice" | "novice" => Self::practice(Ladder::Novice),
            "expert" => Self::practice(Ladder::Expert),
            "arena" | "career" => Self::arena(),
            "recruit" => Self::recruit(),
            "versus" => Self::versus(),
            _ => return None,
        })
    }
}

/// Open the creator.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenCreator(pub CreatorEntry);

/// Open the story's background picker.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenClassSelect;

/// The record the last committed character was made from, which the
/// player's spawn takes its body, skin, hair and name from.
#[derive(Resource, Debug, Clone, Default)]
pub struct CreatedCharacter(pub Option<CharacterRecord>);

/// `--open-creator <mode>`: open the creator (or the background picker for
/// `story` without a background) once the load has ended.
#[derive(Resource, Debug, Clone, Default)]
pub struct OpenCreatorOnLoad {
    pub word: Option<String>,
    pub done: bool,
}

/// The background picker's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassSelect {
    /// The hovered card number (1-based; 0 none).
    pub hover: i32,
    /// The unlock word's background bits.
    pub available: u32,
}

/// Which form a record belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Main,
    Base,
    Hair,
}

/// What the pointer is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Control(Part, usize),
    Physique,
    Tone,
    HairColour,
    HairList(usize),
}

/// The creation form's state.
#[derive(Debug, Clone)]
pub struct Creator {
    pub entry: CreatorEntry,
    pub page: CreationScreenPage,
    pub record: CharacterRecord,
    pub screen: (i32, i32),
    pub unit: i32,
    pub compact: bool,
    /// The records in window pixels, relative to their form's corner.
    pub main: Vec<Control>,
    pub base: Vec<Control>,
    pub hair: Vec<Control>,
    pub name: exa_ui::controls::TextField,
    pub name_focused: bool,
    pub physique: Picker,
    pub tone: TonePicker,
    pub hair_colour: HairColPicker,
    /// The hair styles the list offers (the `CharExt` hair entries).
    pub hair_styles: Vec<String>,
    /// The voice selection (0-based) and its preview clock.
    pub voice: i32,
    pub voice_clock: i32,
    pub held: Option<Target>,
    pub hovered: Option<Target>,
    /// The name warning box is up.
    pub warning: Option<NameWarning>,
    /// The record changed since the preview was built.
    dirty: bool,
    /// The regeneration clock (frames), its last stamp and whether the
    /// preview was built once.
    clock: u32,
    regenerated: u32,
    built: bool,
    /// The Height and Age trackbars' settle stamp.
    settle_last: i32,
}

/// Scales a design length at unit 4 to `unit`, as the original's rescale:
/// divided by the old unit, then times the new.
pub fn px(v: i32, unit: i32) -> i32 {
    v / 4 * unit
}

/// The design size at unit 4 a window of `screen` pixels stands for.
pub fn design(screen: (i32, i32), unit: i32) -> (i32, i32) {
    let unit = unit.max(1);
    (screen.0 * 4 / unit, screen.1 * 4 / unit)
}

fn scaled(controls: &[Control], unit: i32) -> Vec<Control> {
    controls
        .iter()
        .map(|c| {
            let mut c = c.clone();
            c.x = px(c.x, unit);
            c.y = px(c.y, unit);
            c.w = px(c.w, unit);
            c.h = px(c.h, unit);
            if c.class == 4 {
                // A rescale re-sizes a knob bar's knob to 4·unit + 2.
                c.knob = unit * 4 + 2;
            }
            c
        })
        .collect()
}

fn hidden(c: &Control) -> bool {
    c.state & 0x110 == 0x110
}

fn takes_hits(c: &Control) -> bool {
    !hidden(c)
        && c.state & state::DISABLED == 0
        && !(c.class == 5 && !c.reports)
        && c.class != exa_ui::controls::SEPARATOR
}

impl Creator {
    /// Opens the creator for `entry` with `record`, at `screen` pixels and
    /// GUI `unit`.
    pub fn open(
        entry: CreatorEntry,
        record: CharacterRecord,
        screen: (i32, i32),
        unit: i32,
        hair_styles: Vec<String>,
    ) -> Self {
        let mut creator = Self {
            entry,
            page: CreationScreenPage::Appearance,
            name: exa_ui::controls::TextField::new(
                record.name.as_bytes(),
                CREATION_RECORDS[creation::NAME].max_len,
                CREATION_RECORDS[creation::NAME].style,
            ),
            record,
            screen,
            unit: unit.max(1),
            compact: false,
            main: Vec::new(),
            base: Vec::new(),
            hair: Vec::new(),
            name_focused: false,
            physique: Picker::default(),
            tone: TonePicker::new(),
            hair_colour: HairColPicker::new(HAIR_PICKER_AT),
            hair_styles,
            voice: 0,
            voice_clock: 0,
            held: None,
            hovered: None,
            warning: None,
            dirty: true,
            clock: 0,
            regenerated: 0,
            built: false,
            settle_last: 0,
        };
        creator.tone.callback = true;
        creator.hair_colour.callback = true;
        creator.hair_colour.material = true;
        creator.hair_colour.attach();
        creator.voice = creator.record.voice_number as i32;
        creator.lay_out();
        creator.refresh_page();
        creator
    }

    /// The design size of the form.
    pub fn design(&self) -> (i32, i32) {
        design(self.screen, self.unit)
    }

    /// Lays the three tables out for the current size and unit.
    pub fn lay_out(&mut self) {
        let d = self.design();
        self.compact = d.1 < COMPACT_BELOW;
        let main = creation_forms::lay_out(&CREATION_RECORDS, d);
        let mut base_records = BASE_RECORDS;
        // The constructor writes the VOICE label's row first: 0x71 in the
        // compact layout, else 0x7f.
        base_records[base::VOICE].y = i32::from(create::base_form(self.compact).table_word);
        let base = creation_forms::lay_out(&base_records, self.base_size());
        let mut hair = creation_forms::lay_out(&HAIR_RECORDS, self.hair_size());
        // The hair picker's attach moves its trackbar 120 under the picker,
        // as wide as it and 40 tall.
        if let Some(attached) = self.hair_colour.attached {
            let (x, y, w, h) = attached.place;
            let gloss = &mut hair[hair::GLOSS];
            gloss.x = x;
            gloss.y = y;
            gloss.w = w;
            gloss.h = h;
        }
        let keep = |old: &[Control], new: &mut [Control]| {
            for (old, new) in old.iter().zip(new.iter_mut()) {
                new.state = old.state;
                new.value = old.value;
                new.text = old.text.clone();
            }
        };
        let mut main = scaled(&main, self.unit);
        let mut base = scaled(&base, self.unit);
        let mut hair = scaled(&hair, self.unit);
        keep(&self.main, &mut main);
        keep(&self.base, &mut base);
        keep(&self.hair, &mut hair);
        self.main = main;
        self.base = base;
        self.hair = hair;
    }

    /// The window was resized or the unit changed.
    pub fn resize(&mut self, screen: (i32, i32), unit: i32) {
        if (screen, unit.max(1)) == (self.screen, self.unit) {
            return;
        }
        self.screen = screen;
        self.unit = unit.max(1);
        self.lay_out();
    }

    fn base_size(&self) -> (i32, i32) {
        (
            BASE_SIZE.0,
            if self.compact {
                BASE_COMPACT_HEIGHT
            } else {
                BASE_SIZE.1
            },
        )
    }

    fn hair_size(&self) -> (i32, i32) {
        (
            HAIR_SIZE.0,
            if self.compact {
                HAIR_COMPACT_HEIGHT
            } else {
                HAIR_SIZE.1
            },
        )
    }

    fn tone_height(&self) -> i32 {
        create::base_form(self.compact).tone[3]
    }

    /// The body editor's rectangle in window pixels.
    pub fn base_rect(&self) -> GuiRect {
        let (w, h) = self.base_size();
        GuiRect::new(
            px(BODY_EDITOR_AT[0], self.unit),
            px(BODY_EDITOR_AT[1], self.unit),
            px(w, self.unit),
            px(h, self.unit),
        )
    }

    /// The hair editor's rectangle in window pixels: the form's width less
    /// 176, 32 down.
    pub fn hair_rect(&self) -> GuiRect {
        let (w, h) = self.hair_size();
        GuiRect::new(
            px(self.design().0 - HAIR_EDITOR_RIGHT, self.unit),
            px(HAIR_EDITOR_TOP, self.unit),
            px(w, self.unit),
            px(h, self.unit),
        )
    }

    /// The preview viewport in window pixels.
    pub fn preview_px(&self) -> GuiRect {
        let f = create::preview_frame(self.design());
        GuiRect::new(
            px(f.x, self.unit),
            px(f.y, self.unit),
            px(f.width, self.unit),
            px(f.height, self.unit),
        )
    }

    fn field(&self, form: GuiRect, at: (i32, i32), size: (i32, i32)) -> GuiRect {
        GuiRect::new(
            form.left + px(at.0, self.unit),
            form.top + px(at.1, self.unit),
            px(size.0, self.unit),
            px(size.1, self.unit),
        )
    }

    /// The physique picker's field in window pixels.
    pub fn physique_rect(&self) -> GuiRect {
        self.field(self.base_rect(), PHYSIQUE_AT, (0xa0, 0xa0))
    }

    /// The tone picker's field in window pixels.
    pub fn tone_rect(&self) -> GuiRect {
        self.field(self.base_rect(), TONE_AT, (0xa0, self.tone_height()))
    }

    /// The hair-colour picker's field in window pixels.
    pub fn hair_colour_rect(&self) -> GuiRect {
        self.field(self.hair_rect(), HAIR_PICKER_AT, self.hair_colour.size)
    }

    /// The hair style list's rectangle and row step in window pixels.
    pub fn hair_list_rect(&self) -> (GuiRect, i32) {
        let p = self.hair_colour.size;
        let list = create::hair_list(
            [HAIR_PICKER_AT.0, HAIR_PICKER_AT.1, p.0, p.1],
            self.hair_size().1,
            4,
        );
        let [x, y, w, h] = list.rect;
        (
            self.field(self.hair_rect(), (x, y), (w, h)),
            px(list.step, self.unit),
        )
    }

    /// The editors show on the appearance page.
    pub fn editors_shown(&self) -> bool {
        self.page == CreationScreenPage::Appearance
    }

    /// Puts the record into the controls (the edit page's update).
    pub fn refresh_page(&mut self) {
        let page = create::edit_page(&create::PageSource {
            physique: self.record.physique,
            height: self.record.height,
            age: self.record.age,
            skin: self.record.skin,
            hair_colour: [
                self.record.hair_colour[0],
                self.record.hair_colour[1],
                self.record.hair_colour[2],
            ],
            slider: 0.0,
            number: 0,
            voice: self.voice.max(0) as u32,
            pitch: self.record.voice_pitch,
            word: self.record.profession(),
            hair_picker: true,
        });
        self.physique.point = page.physique_point;
        self.tone.point = PickPoint {
            u: page.skin_point[0],
            v: page.skin_point[1],
        };
        self.tone.refresh();
        if let Some([u, v, value]) = page.hair_colour {
            self.hair_colour.set(PickPoint { u, v }, value);
        }
        let set_on = |c: &mut Control, on: bool| {
            if on {
                c.state |= state::ON | state::LIT;
            } else {
                c.state &= !(state::ON | state::LIT);
            }
        };
        set_on(&mut self.base[base::MALE], page.male_on);
        set_on(&mut self.base[base::FEMALE], page.female_on);
        self.base[base::HEIGHT].value = page.height_slider;
        self.base[base::AGE].value = page.age_slider;
        self.base[base::PITCH].value = page.pitch_slider;
        self.base[base::VOICE_LETTER].text = vec![page.voice_letter];
        set_on(&mut self.hair[hair::STRAIGHT], page.hair_off_on);
        set_on(&mut self.hair[hair::WAVY], page.hair_on);
        self.hair[hair::GLOSS].value = self.hair_colour.attached.map_or(0.25, |a| a.value);
        self.hair[hair::FACIAL_LETTER].text = vec![page.variant_letter.unwrap_or(b'-')];
        // A woman's record disables the facial-hair controls (the look's
        // state 16 on its four records).
        let look = create::update_creation_look(self.record.female, 0);
        for i in [
            hair::FACIAL,
            hair::FACIAL_PREVIOUS,
            hair::FACIAL_LETTER,
            hair::FACIAL_NEXT,
        ] {
            let c = &mut self.hair[i];
            c.state = (c.state & !state::DISABLED) | (look.values[0] as u32 & state::DISABLED);
        }
        // The page's labels and the editors' visibility.
        let header = match self.page {
            CreationScreenPage::Appearance => create::appearance_page_header(),
            CreationScreenPage::Equipment => {
                create::equipment_page_header(self.entry.game_mode, self.entry.screen_mode, false)
            }
        };
        self.main[creation::NAME_LABEL].text = header.first.as_bytes().to_vec();
        self.main[creation::HAIR_LABEL].text = header.second.as_bytes().to_vec();
        self.main[creation::NEXT].text = header.final_button.as_bytes().to_vec();
        if self.page == CreationScreenPage::Equipment {
            self.main[creation::NEXT].text =
                exa_ui::creation::commit_label(self.entry.screen_mode, self.entry.game_mode)
                    .as_bytes()
                    .to_vec();
        }
        let name = &mut self.main[creation::NAME];
        if self.page == CreationScreenPage::Appearance {
            name.state &= !0x110;
        } else {
            // [guess] the name field is hidden behind the equipment page.
            name.state |= 0x110;
        }
    }

    /// The record changed: the preview is rebuilt once the rule lets it.
    fn changed(&mut self) {
        self.dirty = true;
    }

    /// Advances the regeneration clock a frame.
    pub fn tick(&mut self) {
        self.clock = self.clock.wrapping_add(1);
    }

    /// Whether the preview is rebuilt now: a change waits while the clock
    /// is within two units of the last rebuild (the body editor's rule,
    /// [`create::sync_physique`]; the clock counts frames [guess]).
    pub fn take_regeneration(&mut self) -> bool {
        if !self.dirty {
            return false;
        }
        if self.built && u64::from(self.regenerated) + 2 > u64::from(self.clock) {
            return false;
        }
        self.regenerated = self.clock;
        self.built = true;
        self.dirty = false;
        true
    }

    fn controls(&self, part: Part) -> &[Control] {
        match part {
            Part::Main => &self.main,
            Part::Base => &self.base,
            Part::Hair => &self.hair,
        }
    }

    fn controls_mut(&mut self, part: Part) -> &mut Vec<Control> {
        match part {
            Part::Main => &mut self.main,
            Part::Base => &mut self.base,
            Part::Hair => &mut self.hair,
        }
    }

    fn origin(&self, part: Part) -> (i32, i32) {
        match part {
            Part::Main => (0, 0),
            Part::Base => (self.base_rect().left, self.base_rect().top),
            Part::Hair => (self.hair_rect().left, self.hair_rect().top),
        }
    }

    /// What is under the window point `(x, y)`.
    pub fn hit(&self, x: i32, y: i32) -> Option<Target> {
        if self.editors_shown() {
            if self.physique_rect().contains(x, y) {
                return Some(Target::Physique);
            }
            if self.tone_rect().contains(x, y) {
                return Some(Target::Tone);
            }
            if self.hair_colour_rect().contains(x, y) {
                return Some(Target::HairColour);
            }
            let (list, step) = self.hair_list_rect();
            if list.contains(x, y) && step > 0 {
                let row = ((y - list.top) / step) as usize;
                if row <= self.hair_styles.len() {
                    return Some(Target::HairList(row));
                }
            }
        }
        let parts: &[Part] = if self.editors_shown() {
            &[Part::Base, Part::Hair, Part::Main]
        } else {
            &[Part::Main]
        };
        for &part in parts {
            let (ox, oy) = self.origin(part);
            let (lx, ly) = (x - ox, y - oy);
            for slop in [false, true] {
                for (i, c) in self.controls(part).iter().enumerate() {
                    if takes_hits(c)
                        && exa_ui::form::control_hit(
                            lx,
                            ly,
                            (c.x, c.y, c.x + c.w, c.y + c.h),
                            slop,
                            self.unit,
                        )
                    {
                        return Some(Target::Control(part, i));
                    }
                }
            }
        }
        None
    }

    fn send(&mut self, part: Part, i: usize, m: Message) -> Vec<usize> {
        let unit = self.unit;
        let mut events = Vec::new();
        gui_controls::dispatch(self.controls_mut(part), i, m, unit, &mut events);
        events
            .into_iter()
            .filter_map(|e| match e {
                ControlEvent::Changed(i) => Some(i),
                ControlEvent::AltPress(_) => None,
            })
            .collect()
    }

    /// The pointer moved to `(x, y)`: a held picker or trackbar follows it,
    /// the hover moves. Returns what the creator asks of the game.
    pub fn pointer(&mut self, x: i32, y: i32) -> Vec<CreatorAction> {
        let hit = self.hit(x, y);
        if let Some(held) = self.held {
            return match held {
                Target::Control(part, i) => {
                    let (ox, _) = self.origin(part);
                    let class = self.controls(part)[i].class;
                    let param = if class == 4 { (x - ox) as u32 } else { 0 };
                    let changed = self.send(
                        part,
                        i,
                        Message {
                            message: message::HOVER,
                            param,
                            over: hit == Some(held),
                            ..Message::default()
                        },
                    );
                    self.answer(part, &changed)
                }
                Target::Physique | Target::Tone | Target::HairColour => {
                    self.pick(held, x, y);
                    Vec::new()
                }
                Target::HairList(_) => Vec::new(),
            };
        }
        if hit != self.hovered {
            for (target, over) in [(self.hovered, false), (hit, true)] {
                if let Some(Target::Control(part, i)) = target
                    && let Some(c) = self.controls_mut(part).get_mut(i)
                {
                    if over {
                        c.state = (c.state & !0xf_0000) | 0x1_0000;
                    } else {
                        c.state &= !0xf_0000;
                    }
                    if c.class != 4 {
                        self.send(
                            part,
                            i,
                            Message {
                                message: message::HOVER,
                                over,
                                ..Message::default()
                            },
                        );
                    }
                }
            }
            self.hovered = hit;
        }
        Vec::new()
    }

    /// A left press at `(x, y)`.
    pub fn press(&mut self, x: i32, y: i32) -> Vec<CreatorAction> {
        if self.warning.is_some() {
            return Vec::new();
        }
        let hit = self.hit(x, y);
        self.name_focused = hit == Some(Target::Control(Part::Main, creation::NAME));
        let Some(target) = hit else {
            return Vec::new();
        };
        self.held = Some(target);
        match target {
            Target::Control(part, i) => {
                let c = &self.controls(part)[i];
                if c.class == exa_ui::controls::TEXT_FIELD {
                    self.name.focus(true);
                    return Vec::new();
                }
                let (ox, _) = self.origin(part);
                let param = if c.class == 5 {
                    gui_controls::LABEL_CLICK
                } else {
                    (x - ox) as u32
                };
                let changed = self.send(
                    part,
                    i,
                    Message {
                        message: message::PRESS,
                        param,
                        extra: 1,
                        over: true,
                        ..Message::default()
                    },
                );
                self.answer(part, &changed)
            }
            Target::Physique | Target::Tone | Target::HairColour => {
                self.pick(target, x, y);
                Vec::new()
            }
            Target::HairList(row) => {
                // The list's selection is 1-based: row r is selection r + 1.
                if create::hair_style_changed(&mut self.record.hair_style, row as i32 + 1) {
                    self.changed();
                }
                Vec::new()
            }
        }
    }

    /// A left release at `(x, y)`.
    pub fn release(&mut self, x: i32, y: i32) -> Vec<CreatorAction> {
        if self.warning.is_some() {
            // The warning's box closes on a click.
            self.warning = None;
            return Vec::new();
        }
        let Some(held) = self.held.take() else {
            return Vec::new();
        };
        match held {
            Target::Control(part, i) => {
                let over = self.hit(x, y) == Some(held);
                let changed = self.send(
                    part,
                    i,
                    Message {
                        message: message::RELEASE,
                        param: gui_controls::RELEASE_LANDS,
                        extra: 1,
                        over,
                        ..Message::default()
                    },
                );
                self.answer(part, &changed)
            }
            _ => Vec::new(),
        }
    }

    /// A picker took the point `(x, y)`.
    fn pick(&mut self, target: Target, x: i32, y: i32) {
        match target {
            Target::Physique => {
                let r = self.physique_rect();
                let local = (x - r.left, y - r.top);
                if self.physique.drag(local, (r.width(), r.height())) {
                    // The point goes into the record at once.
                    self.record.physique = self.physique.point;
                    self.changed();
                }
            }
            Target::Tone => {
                let r = self.tone_rect();
                self.tone.size = (r.width(), r.height());
                let before = self.tone.point;
                self.tone.press(Some((x - r.left, y - r.top)), 0);
                if self.tone.point != before {
                    let mut source = self.page_source();
                    create::skin_changed(&mut source, [self.tone.point.u, self.tone.point.v]);
                    self.record.skin = source.skin;
                    self.changed();
                }
            }
            Target::HairColour => {
                let r = self.hair_colour_rect();
                let size = self.hair_colour.size;
                self.hair_colour.size = (r.width(), r.height());
                let before = self.hair_colour.point;
                self.hair_colour.press(Some((x - r.left, y - r.top)), 0);
                self.hair_colour.size = size;
                if self.hair_colour.point != before {
                    self.hair_colour_changed();
                }
            }
            _ => {}
        }
    }

    fn hair_colour_changed(&mut self) {
        let value = self.hair_colour.attached.map_or(0.25, |a| a.value);
        let mut bytes = Vec::new();
        create::hair_colour_changed(
            &mut bytes,
            [self.hair_colour.point.u, self.hair_colour.point.v, value],
        );
        for (slot, byte) in self.record.hair_colour.iter_mut().zip(bytes) {
            *slot = byte;
        }
        self.changed();
    }

    fn page_source(&self) -> create::PageSource {
        create::PageSource {
            physique: self.record.physique,
            height: self.record.height,
            age: self.record.age,
            skin: self.record.skin,
            hair_colour: [
                self.record.hair_colour[0],
                self.record.hair_colour[1],
                self.record.hair_colour[2],
            ],
            slider: 0.0,
            number: 0,
            voice: self.voice.max(0) as u32,
            pitch: self.record.voice_pitch,
            word: self.record.profession(),
            hair_picker: true,
        }
    }

    /// Runs the callbacks of the records that reported a change.
    fn answer(&mut self, part: Part, changed: &[usize]) -> Vec<CreatorAction> {
        let mut out = Vec::new();
        for &i in changed {
            match (part, i) {
                (Part::Main, creation::BACK) => out.push(self.back()),
                (Part::Main, creation::NEXT) => out.extend(self.advance()),
                (Part::Base, base::MALE | base::FEMALE) => {
                    let c = &self.base[i];
                    if c.state & state::ON != 0 {
                        let radio = if i == base::FEMALE { 2 } else { 1 };
                        if let Some(change) = create::sex_radio(
                            self.record.profession() as i32,
                            radio,
                            true,
                            true,
                            true,
                        ) {
                            self.record.set_profession(change.word as u32);
                            // Both voice selections go back to the first.
                            self.voice = 0;
                            self.record.voice_number = 0;
                            self.record.voice = exa_formats::characters::voice_name(0);
                            self.refresh_page();
                            self.changed();
                        }
                    }
                }
                (Part::Base, base::HEIGHT) => {
                    let settle = create::settle_appearance(
                        self.base[i].value,
                        self.record.height,
                        self.settle_last,
                        self.clock as i32,
                    );
                    if let Some(value) = settle.stored {
                        self.record.height = value;
                        self.settle_last = settle.last;
                        self.changed();
                    }
                }
                (Part::Base, base::AGE) => {
                    let settle = create::copy_appearance(
                        self.base[i].value,
                        self.record.age,
                        self.settle_last,
                        self.clock as i32,
                    );
                    if let Some(value) = settle.stored {
                        self.record.age = value;
                        self.settle_last = settle.last;
                        self.changed();
                    }
                }
                (Part::Base, base::PITCH) => {
                    self.record.voice_pitch = create::voice_pitch(self.base[i].value);
                }
                (Part::Base, base::VOICE_PREVIOUS) => {
                    if let Some(voice) = create::previous_voice(self.voice) {
                        self.select_voice(voice);
                    }
                }
                (Part::Base, base::VOICE_NEXT) => {
                    self.select_voice(create::next_voice(self.voice));
                }
                (Part::Base, base::VOICE_PLAY) => {
                    out.push(CreatorAction::PlayVoice(self.voice));
                }
                (Part::Hair, hair::STRAIGHT | hair::WAVY) => {
                    let wavy = i == hair::WAVY && self.hair[i].state & state::ON != 0;
                    let word =
                        create::set_hair_visible(self.record.profession() as i32, wavy) as u32;
                    self.record.set_profession(word);
                    self.changed();
                }
                (Part::Hair, hair::GLOSS) => {
                    let value = self.hair[i].value;
                    self.hair_colour.child_refresh(value);
                    self.hair_colour_changed();
                }
                (Part::Hair, hair::FACIAL_PREVIOUS) => {
                    let step = create::step_creation_variant(self.record.profession() as i32, true);
                    self.variant_stepped(step);
                }
                (Part::Hair, hair::FACIAL_NEXT) => {
                    let step =
                        create::advance_creation_variant(self.record.profession() as i32, true);
                    self.variant_stepped(step);
                }
                _ => {}
            }
        }
        out
    }

    fn variant_stepped(&mut self, step: create::CreationStep) {
        if !step.changed {
            return;
        }
        self.record.set_profession(step.word as u32);
        self.hair[hair::FACIAL_LETTER].text = vec![step.caption.unwrap_or(b'-')];
        self.changed();
    }

    fn select_voice(&mut self, voice: i32) {
        self.voice = voice;
        self.record.voice_number = voice.max(0) as u32;
        self.record.voice = exa_formats::characters::voice_name(voice.max(0) as u32);
        self.base[base::VOICE_LETTER].text = vec![(voice as u8).wrapping_add(b'A')];
    }

    /// The submit state the final button reads.
    fn submit_input(&self) -> SubmitInput {
        SubmitInput {
            page: self.page,
            name_len: self.name.text.len(),
            // [guess] the power tree is not wired: the record's powers are
            // not offered to it.
            known_powers: 0,
            game_mode: self.entry.game_mode,
            screen_mode: self.entry.screen_mode,
            continue_mode: false,
            equipment_hook: false,
        }
    }

    /// NEXT (and the final button on the equipment page): the page advances
    /// or the record commits.
    pub fn advance(&mut self) -> Option<CreatorAction> {
        let submit = create::submit_creation(self.submit_input());
        if let Some(warning) = submit.warning {
            self.warning = Some(warning);
            return None;
        }
        if submit.build_equipment {
            self.page = CreationScreenPage::Equipment;
            self.refresh_page();
            return None;
        }
        submit.commit.map(|kind| {
            let record = self.committed_record();
            CreatorAction::Commit {
                kind,
                record: Box::new(record),
            }
        })
    }

    /// The record the commit writes: the creator's name and gear, its
    /// skills, full health.
    pub fn committed_record(&self) -> CharacterRecord {
        let mut record = self.record.clone();
        let gear = CreationGear {
            hands: record.hands.map(|h| GearSlot {
                item: h.id,
                flags: h.flags,
            }),
            worn: record
                .worn
                .iter()
                .map(|w| GearSlot {
                    item: w.id,
                    flags: w.flags,
                })
                .collect(),
            carried: record
                .inventory
                .iter()
                .map(|c| GearSlot {
                    item: c.item,
                    flags: c.flags,
                })
                .collect(),
        };
        let name = create::copy_creation_name(&String::from_utf8_lossy(&self.name.text));
        let skills = record.skills;
        create::write_created_record(&mut record, &name, &gear, &skills, true);
        record
    }

    /// BACK: the equipment page goes back to the appearance page; the
    /// appearance page leaves the creator.
    pub fn back(&mut self) -> CreatorAction {
        let switch = create::switch_creation_page(
            self.page,
            self.entry.screen_mode,
            create::BuiltPanels::default(),
        );
        if switch.destination == PageDestination::Appearance {
            self.page = CreationScreenPage::Appearance;
            self.refresh_page();
            return CreatorAction::None;
        }
        CreatorAction::Leave(switch.destination)
    }

    /// A key typed into the name field: printable bytes up to the limit,
    /// Backspace removes the last.
    pub fn type_name(&mut self, typed: &str) {
        if !self.name_focused || self.page != CreationScreenPage::Appearance {
            return;
        }
        for ch in typed.chars() {
            if !(' '..='~').contains(&ch) {
                continue;
            }
            if self.name.text.len() as i32 >= self.name.max_len {
                break;
            }
            self.name.text.push(ch as u8);
        }
        self.name.edit.reset(self.name.text.len());
    }

    /// Backspace in the name field.
    pub fn backspace(&mut self) {
        if self.name_focused && self.page == CreationScreenPage::Appearance {
            self.name.text.pop();
            self.name.edit.reset(self.name.text.len());
        }
    }
}

/// What the creator asks of the game.
#[derive(Debug, Clone, PartialEq)]
pub enum CreatorAction {
    None,
    /// Leave the creator for the menu (or the arena's recruit form).
    Leave(PageDestination),
    /// The final button committed the record.
    Commit {
        kind: create::CommitKind,
        record: Box<CharacterRecord>,
    },
    /// The voice's preview plays.
    PlayVoice(i32),
}

/// The open screens.
#[derive(Resource, Debug, Default)]
pub struct CreatorScreen {
    pub class_select: Option<ClassSelect>,
    pub creator: Option<Creator>,
}

impl CreatorScreen {
    pub fn is_open(&self) -> bool {
        self.class_select.is_some() || self.creator.is_some()
    }
}

/// The creator's input and paint, after the menu's.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CreatorSystems;

pub struct CreatorScreenPlugin;

impl Plugin for CreatorScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreatorScreen>()
            .init_resource::<CreatedCharacter>()
            .init_resource::<OpenCreatorOnLoad>()
            .init_resource::<GuiFrames>()
            .add_message::<MenuChoice>()
            .add_message::<OpenCreator>()
            .add_message::<OpenClassSelect>()
            .add_message::<KeyboardInput>()
            .add_systems(
                Update,
                (open_from_menu, open_screens, input, paint)
                    .chain()
                    .in_set(CreatorSystems)
                    .after(MenuScreenSystems),
            );
    }
}

fn screen_and_unit(window: &Window) -> ((i32, i32), i32) {
    let screen = (
        window.physical_width() as i32,
        window.physical_height() as i32,
    );
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    (screen, unit)
}

/// The menu's rows that open the creator or the background picker, and
/// `--open-creator` once the load has ended.
fn open_from_menu(
    mut choices: MessageReader<MenuChoice>,
    mut on_load: ResMut<OpenCreatorOnLoad>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    flow: Option<Res<Flow>>,
    mut creators: MessageWriter<OpenCreator>,
    mut pickers: MessageWriter<OpenClassSelect>,
) {
    for MenuChoice(command) in choices.read() {
        match *command {
            MenuCommand::BeginStory => {
                pickers.write(OpenClassSelect);
            }
            MenuCommand::Practice { expert } => {
                let ladder = if expert {
                    Ladder::Expert
                } else {
                    Ladder::Novice
                };
                creators.write(OpenCreator(CreatorEntry::practice(ladder)));
            }
            MenuCommand::NewPractice => {
                let ladder = flow.as_ref().map_or(Ladder::Novice, |f| f.practice.ladder);
                creators.write(OpenCreator(CreatorEntry::practice(ladder)));
            }
            MenuCommand::NewCareer => {
                creators.write(OpenCreator(CreatorEntry::arena()));
            }
            MenuCommand::Versus => {
                creators.write(OpenCreator(CreatorEntry::versus()));
            }
            _ => {}
        }
    }
    if on_load.done || loading.is_some() {
        return;
    }
    let Some(word) = on_load.word.clone() else {
        return;
    };
    on_load.done = true;
    if word == "story" {
        pickers.write(OpenClassSelect);
    } else if let Some(entry) = CreatorEntry::parse(&word) {
        creators.write(OpenCreator(entry));
    } else {
        warn!("--open-creator: unknown mode {word:?}");
    }
}

/// The record a creator opens with: the entry's plan, a background's
/// template where the story names one, full health, the mode's gear filter
/// and the underwear.
pub fn entry_record(entry: CreatorEntry, template: Option<CharacterRecord>) -> CharacterRecord {
    let plan = create::plan_creation_entry(
        entry.screen_mode,
        entry.game_mode,
        entry.background,
        false,
        true,
        false,
        false,
    );
    let mut record = CharacterRecord::reset();
    if let Ok(plan) = plan {
        if let create::EntrySource::Background(background) = plan.source
            && let Some(bg) = create::plan_background(background, false)
            && bg.source == create::PresetSource::Template
            && let Some(template) = template
        {
            record = template;
        }
        if plan.full_health {
            record.pools = Pools::FULL;
        }
        if plan.clear_experience {
            record.experience = 0;
        }
        if plan.filter {
            let mut gear = CreationGear {
                hands: record.hands.map(|h| GearSlot {
                    item: h.id,
                    flags: h.flags,
                }),
                worn: record
                    .worn
                    .iter()
                    .map(|w| GearSlot {
                        item: w.id,
                        flags: w.flags,
                    })
                    .collect(),
                carried: Vec::new(),
            };
            create::filter_creation_gear(&mut gear, entry.game_mode, entry.background, &[]);
            record.hands = gear.hands.map(GearSlot::item_ref);
            record.worn = gear.worn.iter().map(|s| s.item_ref()).collect();
            record.inventory.clear();
        }
    }
    if !record
        .worn
        .iter()
        .any(|w| w.index() == exa_formats::characters::UNDERWEAR_ITEM)
        && record.race == 0
    {
        record.worn.push(exa_formats::characters::ItemRef::new(
            exa_formats::characters::UNDERWEAR_ITEM,
            0,
        ));
    }
    record
}

/// Loads a GUI texture the screens need once.
fn ensure_texture(textures: &mut GuiTextures, game: Option<&exa_assets::Game>, name: &str) {
    if textures.textures.contains_key(name) || textures.missing.iter().any(|n| n == name) {
        return;
    }
    let Some(game) = game else {
        return;
    };
    match game.load_rgba(name) {
        Ok(image) => {
            if let Some(texture) =
                GuiTexture::from_bottom_up(image.width, image.height, &image.pixels)
            {
                textures.textures.insert(name.to_string(), texture);
            }
        }
        Err(error) => {
            warn!("creator: GUI texture {name:?}: {error}");
            textures.missing.push(name.to_string());
        }
    }
}

/// The hair styles the list offers: the `CharExt` hair entries.
fn hair_styles(game: Option<&exa_assets::Game>) -> Vec<String> {
    game.and_then(|g| g.0.pack(exa_assets::PackKind::CharExt))
        .map(|p| {
            p.entries()
                .iter()
                .filter(|e| e.name.starts_with("hair "))
                .map(|e| e.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Opens the picker or the creator, hiding the menu: the session phase
/// becomes the class select's or the creator's.
#[allow(clippy::too_many_arguments)]
fn open_screens(
    mut pickers: MessageReader<OpenClassSelect>,
    mut creators: MessageReader<OpenCreator>,
    mut screen: ResMut<CreatorScreen>,
    mut flow: Option<ResMut<Flow>>,
    mut steps: Option<ResMut<FlowSteps>>,
    mut menu: Option<ResMut<crate::menu_screen::MenuScreen>>,
    game: Option<Res<exa_assets::Game>>,
    mut textures: Option<ResMut<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let picked = pickers.read().count() > 0;
    let entry = creators.read().last().copied();
    if !picked && entry.is_none() {
        return;
    }
    if let Some(menu) = menu.as_mut()
        && menu.open
    {
        menu.close();
    }
    if let Some(steps) = steps.as_mut() {
        // The menu's show: the menu hides and the world is torn down; the
        // player's controls are let go while a setup form is up.
        steps.steps.push(crate::flow::Step::HideMenu);
        steps.steps.push(crate::flow::Step::TeardownWorld);
        steps.steps.push(crate::flow::Step::Controls(false));
    }
    if let Some(textures) = textures.as_mut() {
        ensure_texture(textures, game.as_deref(), STRIP);
        ensure_texture(textures, game.as_deref(), ICONS);
    }
    if picked {
        let available = flow.as_ref().map_or(0, |f| f.unlocks.backgrounds);
        info!("creator: the background picker opens (unlock word {available:#x})");
        screen.class_select = Some(ClassSelect {
            hover: 0,
            available,
        });
        if let Some(flow) = flow.as_mut() {
            flow.phase = Phase::from_word(MenuShow::ClassSelect.state());
            flow.menu_open = false;
        }
    }
    if let Some(OpenCreator(entry)) = entry {
        let Ok(window) = windows.single() else {
            return;
        };
        let (size, unit) = screen_and_unit(window);
        let template = match create::background_preset(entry.background) {
            Some(create::BackgroundPreset {
                name: Some(name), ..
            }) if entry.screen_mode == 4 => game
                .as_deref()
                .and_then(|g| crate::prelude::ActorCache::new().template(g, name, false)),
            _ => None,
        };
        let record = entry_record(entry, template);
        info!(
            "creator: opens for screen mode {}, game mode {:#x}, background {} at {size:?}, unit {unit}",
            entry.screen_mode, entry.game_mode, entry.background
        );
        screen.class_select = None;
        screen.creator = Some(Creator::open(
            entry,
            record,
            size,
            unit,
            hair_styles(game.as_deref()),
        ));
        if let Some(flow) = flow.as_mut() {
            // The show leaves the creator's session state and copies the
            // game mode into the creator's.
            flow.phase = Phase::from_word(MenuShow::Creator.state());
            if MenuShow::Creator.copies_game_mode() {
                flow.mode = crate::flow::Mode::from_word(entry.game_mode as u32);
            }
            flow.menu_open = false;
        }
    }
}

/// The pointer and keys while a screen is open.
#[allow(clippy::too_many_arguments)]
fn input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut typed: MessageReader<KeyboardInput>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    time: Res<Time<Real>>,
    mut screen: ResMut<CreatorScreen>,
    mut flow: Option<ResMut<Flow>>,
    mut steps: Option<ResMut<FlowSteps>>,
    mut created: ResMut<CreatedCharacter>,
    mut creators: MessageWriter<OpenCreator>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut commands: Commands,
    players: Query<Entity, With<crate::player::Player>>,
) {
    let typed: Vec<KeyboardInput> = typed.read().cloned().collect();
    let Ok(window) = windows.single() else {
        return;
    };
    let pointer = match scripted.as_deref() {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    }
    .map(|at| {
        let scale = window.scale_factor();
        ((at.x * scale) as i32, (at.y * scale) as i32)
    });
    let (size, unit) = screen_and_unit(window);
    if let Some(picker) = screen.class_select.as_mut() {
        let wide = (size.0.max(0) as u32, size.1);
        let mut mask = 0u32;
        if let Some((x, y)) = pointer {
            let hover = exa_ui::class_select::hit(wide, picker.available, (x, y)).unwrap_or(0);
            if hover > 0 {
                picker.hover = exa_ui::class_select::class_hover(hover, false).index;
            }
        }
        if keys.just_pressed(KeyCode::Enter) || mouse.just_released(MouseButton::Left) {
            mask |= 0x100;
        }
        if keys.just_pressed(KeyCode::Escape) || mouse.just_released(MouseButton::Right) {
            mask |= 0x200;
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            mask |= 0x2001;
        }
        if keys.just_pressed(KeyCode::ArrowLeft) {
            mask |= 0x1001;
        }
        match exa_ui::class_select::class_key(mask, picker.hover, picker.available) {
            exa_ui::class_select::ClassKey::Confirm => {
                let choose = exa_ui::class_select::class_choose(picker.hover);
                if let Some(background) = choose.background {
                    info!("creator: background {background} chosen");
                    screen.class_select = None;
                    creators.write(OpenCreator(CreatorEntry::story(background)));
                }
            }
            exa_ui::class_select::ClassKey::Back => {
                screen.class_select = None;
                if let (Some(flow), Some(steps)) = (flow.as_mut(), steps.as_mut()) {
                    steps.steps.extend(flow.class_cancel_direct());
                }
            }
            exa_ui::class_select::ClassKey::Move(card) => picker.hover = card,
            exa_ui::class_select::ClassKey::Nothing => {}
        }
        return;
    }
    let Some(creator) = screen.creator.as_mut() else {
        return;
    };
    creator.tick();
    creator.resize(size, unit);
    let mut actions = Vec::new();
    for event in &typed {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Character(text) => creator.type_name(text.as_str()),
            Key::Space => creator.type_name(" "),
            _ => {}
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        creator.backspace();
    }
    if keys.just_pressed(KeyCode::Escape) {
        if creator.warning.take().is_none() {
            // The cancel key leaves as the appearance page's BACK does.
            actions.push(CreatorAction::Leave(create::leave_creator(
                creator.entry.screen_mode,
            )));
        }
    } else if keys.just_pressed(KeyCode::Enter) && creator.warning.take().is_none() {
        actions.extend(creator.advance());
    }
    if let Some((x, y)) = pointer {
        actions.extend(creator.pointer(x, y));
        if mouse.just_pressed(MouseButton::Left) {
            actions.extend(creator.press(x, y));
        }
        if mouse.just_released(MouseButton::Left) {
            actions.extend(creator.release(x, y));
        }
    }
    let now = (time.elapsed().as_millis() as u64 & 0x7fff_ffff) as i32;
    for action in actions {
        match action {
            CreatorAction::None => {}
            CreatorAction::PlayVoice(voice) => {
                let throttle = create::throttle_voice(now, creator.voice_clock);
                if throttle.played {
                    creator.voice_clock = throttle.last;
                    let prefix = create::voice_prefix(create::creation_label(
                        creator.record.profession() as i32,
                    ));
                    let preview =
                        create::voice_preview(&prefix, voice, creator.base[base::PITCH].value);
                    info!("creator: voice preview {} (not played)", preview.name);
                }
            }
            CreatorAction::Leave(destination) => {
                info!("creator: leaves to {destination:?}");
                screen.creator = None;
                if destination == PageDestination::Menu
                    && let (Some(flow), Some(steps)) = (flow.as_mut(), steps.as_mut())
                {
                    steps.steps.extend(flow.quit_to_menu());
                }
                return;
            }
            CreatorAction::Commit { kind, record } => {
                let entry = creator.entry;
                info!(
                    "creator: commit {kind:?} of {:?} (game mode {:#x})",
                    record.name, entry.game_mode
                );
                screen.creator = None;
                created.0 = Some((*record).clone());
                // The new character replaces the player that survived the
                // last session: the level load spawns it from the record.
                for player in &players {
                    commands.entity(player).despawn();
                }
                let (Some(flow), Some(steps)) = (flow.as_mut(), steps.as_mut()) else {
                    return;
                };
                let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
                let viewport = flow.viewport;
                match (kind, entry.game_mode) {
                    (create::CommitKind::Begin, _) => {
                        let background = Background::from_index(entry.background.max(0) as u32);
                        // [guess] the measured body height is the average
                        // the flow's spawn arithmetic expects.
                        let start = flow.start_story(background, 1.0, viewport, now);
                        steps.steps.extend(flow.begin_session(start));
                    }
                    (create::CommitKind::Create, 2) => {
                        let ladder = entry.ladder.unwrap_or(Ladder::Novice);
                        let standing = flow.stand_height;
                        let start = flow.start_practice(ladder, standing, viewport, now);
                        steps.steps.extend(flow.begin_session(start));
                    }
                    _ => {
                        warn!(
                            "creator: the {kind:?} commit of game mode {:#x} is not wired; back to the menu",
                            entry.game_mode
                        );
                        steps.steps.extend(flow.quit_to_menu());
                    }
                }
                return;
            }
        }
    }
}

/// A solid quad batch layer with one ink per vertex.
fn gradient_layer(quads: &[[FieldVertex; 4]], origin: (f32, f32), blend: Option<u16>) -> GuiLayer {
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    if let Some(blend) = blend {
        batch.blend = blend;
    }
    batch.begin();
    for quad in quads {
        for v in quad {
            batch.set_ink([v.colour[0], v.colour[1], v.colour[2], 255]);
            batch.vertex(origin.0 + v.at.0, origin.1 + v.at.1);
        }
    }
    batch.end_quads();
    GuiLayer::new(batch, LayerTexture::None)
}

/// A one-ink outline or fill of rectangles.
fn rect_layer(rects: &[(f32, f32, f32, f32)], ink: [u8; 4]) -> GuiLayer {
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink(ink);
    batch.begin();
    for &(x, y, w, h) in rects {
        batch.vertex(x, y);
        batch.vertex(x, y + h);
        batch.vertex(x + w, y + h);
        batch.vertex(x + w, y);
    }
    batch.end_quads();
    GuiLayer::new(batch, LayerTexture::None)
}

/// The marker square at a field's point [guess at the skin's marker].
fn marker_layer(field: GuiRect, point: [f32; 2], ink: [u8; 4]) -> GuiLayer {
    let picker = Picker {
        point,
        ..Picker::default()
    };
    let (mx, my) = picker.marker_at((field.width(), field.height()));
    let (x, y) = ((field.left + mx) as f32, (field.top + my) as f32);
    let h = MARKER_HALF;
    rect_layer(
        &[
            (x - h, y - h, h * 2.0, 1.0),
            (x - h, y + h - 1.0, h * 2.0, 1.0),
            (x - h, y - h, 1.0, h * 2.0),
            (x + h - 1.0, y - h, 1.0, h * 2.0),
        ],
        ink,
    )
}

fn text_layer(
    font: &GuiFont,
    size: Option<(u16, u16)>,
    text: &[u8],
    at: (i32, i32),
    ink: [u8; 4],
) -> Option<GuiLayer> {
    let size = size?;
    let quads = draw_line(font, text, at.0, at.1);
    Some(GuiLayer::new(
        text_batch(font, &quads, ink, size),
        LayerTexture::Named(font.texture),
    ))
}

/// Paints one record at `origin`; the text field and the image buttons are
/// drawn here, the other classes by the Settings form's painter.
#[allow(clippy::too_many_arguments)]
fn paint_record(
    c: &Control,
    origin: (i32, i32),
    unit: i32,
    palette: &SkinPalette,
    text: &crate::settings_form::Text,
    focused: bool,
    textures: &GuiTextures,
    painted: &mut Painted,
) {
    if hidden(c) {
        return;
    }
    let rect = GuiRect::new(origin.0 + c.x, origin.1 + c.y, c.w, c.h);
    match c.class {
        exa_ui::controls::TEXT_FIELD => {
            let mut batch = SkinBatch::new();
            exa_ui::skin::draw_style_frames(
                &mut batch,
                exa_ui::skin::GuiRect::sized(
                    rect.left as f32,
                    rect.top as f32,
                    rect.width() as f32,
                    rect.height() as f32,
                ),
                c.style | 0x20,
                palette,
            );
            painted.push(GuiLayer::new(
                batch,
                LayerTexture::Named(exa_ui::gui_draw::SKIN_TEXTURE),
            ));
            let pad = unit;
            let baseline = rect.top + ((rect.height() - text.font.line_height) >> 1);
            let mut shown = c.text.clone();
            if focused {
                shown.push(b'_');
            }
            if let Some(layer) = text_layer(
                text.font,
                text.size,
                &shown,
                (rect.left + pad, baseline),
                palette.text[6],
            ) {
                painted.push(layer);
            }
        }
        3 => {
            let button = gui_paint::paint_line_button(
                rect,
                gui_paint::ButtonWords {
                    style: c.style & 0x00ff_ffff,
                    state: c.state,
                    group: c.group,
                },
                unit,
                palette,
                1.0,
                text.font,
                text.size,
                "",
            );
            painted.layers.extend(button.layers);
            if c.icon > 0
                && let Some(offset) = create::CREATION_ICON_OFFSETS.get(c.icon as usize - 1)
                && textures.get(ICONS).is_some()
            {
                // The sheet is 96 × 384 with its cells cut bottom up; the
                // mirrored style flips the cell across [guess].
                let v0 = *offset as f32 / 384.0;
                let v1 = (*offset + create::CREATION_ICON_SIZE) as f32 / 384.0;
                let (u0, u1) = if c.style & 0x4000_0000 != 0 {
                    (1.0, 0.0)
                } else {
                    (0.0, 1.0)
                };
                let side = rect.width().min(rect.height()) as f32;
                let x = rect.left as f32 + (rect.width() as f32 - side) * 0.5;
                let y = rect.top as f32 + (rect.height() as f32 - side) * 0.5;
                let mut batch = SkinBatch::new();
                batch.select(3, 0, false);
                batch.set_ink(palette.text[6]);
                batch.begin();
                batch.set_uv(u0, v1);
                batch.vertex(x, y);
                batch.set_uv(u0, v0);
                batch.vertex(x, y + side);
                batch.set_uv(u1, v0);
                batch.vertex(x + side, y + side);
                batch.set_uv(u1, v1);
                batch.vertex(x + side, y);
                batch.end_quads();
                painted.push(GuiLayer::new(batch, LayerTexture::Named(ICONS)));
            }
        }
        _ => crate::settings_form::paint_control(c, origin, unit, palette, text, None, painted),
    }
}

/// The creator's paint: the form, the viewport cleared for the preview,
/// the records, and on the appearance page the two editors with their
/// pickers and the hair list; on the equipment page its panels.
pub fn paint_creator(creator: &Creator, textures: &GuiTextures) -> Painted {
    let palette = shown_palette(1.0);
    let unit = creator.unit;
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let text = crate::settings_form::Text { font: &font, size };
    let mut frame = Frame::new(
        GuiRect::new(0, 0, creator.screen.0, creator.screen.1),
        unit,
        "",
    );
    frame.attributes = CREATION_ATTRIBUTES;
    frame.style = 0;
    frame.spacing = 0;
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, size);
    // The viewport: its own frame (border 2, the preview's words), then the
    // canvas cleared inside so the 3D view under it shows.
    let view = creator.preview_px();
    let preview = create::preview_view();
    let panel = gui_paint::paint_child(
        view,
        unit,
        &palette,
        preview.style(EDITOR_ATTRIBUTES),
        preview.form_flags,
        preview.border,
        None,
    );
    painted.layers.extend(panel.layers);
    painted.push(crate::gui_screens::clear_layer(GuiRect::new(
        view.left + preview.border,
        view.top + preview.border,
        view.width() - preview.border * 2,
        view.height() - preview.border * 2,
    )));
    for (i, c) in creator.main.iter().enumerate() {
        let focused = i == creation::NAME && creator.name_focused;
        let mut c = c.clone();
        if i == creation::NAME {
            c.text = creator.name.text.clone();
        }
        paint_record(
            &c,
            (0, 0),
            unit,
            &palette,
            &text,
            focused,
            textures,
            &mut painted,
        );
    }
    if creator.editors_shown() {
        paint_editors(creator, &palette, &text, textures, &mut painted);
    } else {
        paint_equipment(creator, &palette, &mut painted);
    }
    if let Some(warning) = creator.warning {
        paint_warning(creator, warning, &palette, &font, size, &mut painted);
    }
    painted
}

fn paint_editors(
    creator: &Creator,
    palette: &SkinPalette,
    text: &crate::settings_form::Text,
    textures: &GuiTextures,
    painted: &mut Painted,
) {
    let unit = creator.unit;
    let base = creator.base_rect();
    let child = gui_paint::paint_child(base, unit, palette, EDITOR_ATTRIBUTES, 0, 0, None);
    painted.layers.extend(child.layers);
    for c in &creator.base {
        paint_record(
            c,
            (base.left, base.top),
            unit,
            palette,
            text,
            false,
            textures,
            painted,
        );
    }
    // The physique field: the cross in the field's grey, then the marker.
    let field = creator.physique_rect();
    let cross = physique_cross((field.width(), field.height()));
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink(exa_ui::picker_fields::CROSS_COLOUR);
    batch.begin();
    for [from, to] in cross {
        batch.vertex(field.left as f32 + from[0], field.top as f32 + from[1]);
        batch.vertex(field.left as f32 + to[0], field.top as f32 + to[1]);
    }
    batch.end_lines();
    painted.push(GuiLayer::new(batch, LayerTexture::None));
    let mut free = palette.highlight[7];
    free[3] = exa_ui::picker_fields::FREE_MARKER_ALPHA;
    painted.push(marker_layer(field, creator.physique.point, free));
    // The tone field: its checkerboard of four quads, then the marker.
    let field = creator.tone_rect();
    let mut tone = creator.tone.clone();
    tone.size = (field.width(), field.height());
    let quads: Vec<[FieldVertex; 4]> = tone
        .quads()
        .iter()
        .flat_map(|q| [[q[0], q[1], q[2], q[2]], [q[3], q[4], q[5], q[5]]])
        .collect();
    painted.push(gradient_layer(
        &quads,
        (field.left as f32, field.top as f32),
        None,
    ));
    painted.push(marker_layer(
        field,
        [creator.tone.point.u, creator.tone.point.v],
        free,
    ));
    // The hair editor.
    let hair = creator.hair_rect();
    let child = gui_paint::paint_child(
        hair,
        unit,
        palette,
        EDITOR_ATTRIBUTES,
        HAIR_EDITOR_STYLE,
        0,
        None,
    );
    painted.layers.extend(child.layers);
    for c in &creator.hair {
        paint_record(
            c,
            (hair.left, hair.top),
            unit,
            palette,
            text,
            false,
            textures,
            painted,
        );
    }
    let field = creator.hair_colour_rect();
    let mut picker = creator.hair_colour.clone();
    picker.size = (field.width(), field.height());
    let origin = (field.left as f32, field.top as f32);
    painted.push(gradient_layer(&picker.cells(), origin, None));
    // The shade multiplies the columns, black at the top to white at 1.2
    // times the height [guess at the blend].
    let mut shade = gradient_layer(
        &[picker.shade_quad()],
        origin,
        Some(exa_ui::gui_draw::blend_word(
            exa_ui::gui_draw::BlendFactor::DestinationColour,
            exa_ui::gui_draw::BlendFactor::Zero,
        )),
    );
    shade.clip = Some([field.left, field.top, field.width(), field.height()]);
    painted.push(shade);
    let m = creator.hair_colour.marker;
    painted.push(marker_layer(
        field,
        [creator.hair_colour.point.u, creator.hair_colour.point.v],
        [m[0], m[1], m[2], 255],
    ));
    // The style list: one row per style, bald first, the record's framed.
    let (list, step) = creator.hair_list_rect();
    let child = gui_paint::paint_child(list, unit, palette, EDITOR_ATTRIBUTES, 0x80, 0, None);
    painted.layers.extend(child.layers);
    let clip = Some([list.left, list.top, list.width(), list.height()]);
    let rows = std::iter::once("-".to_string()).chain(
        creator
            .hair_styles
            .iter()
            .map(|name| name.trim_start_matches("hair ").to_string()),
    );
    for (row, name) in rows.enumerate() {
        let top = list.top + row as i32 * step;
        if top >= list.bottom {
            break;
        }
        if row as u32 == creator.record.hair_style {
            let mut layer = rect_layer(
                &[(
                    (list.left + 2) as f32,
                    top as f32,
                    (list.width() - 4) as f32,
                    step as f32,
                )],
                palette.highlight[2],
            );
            layer.clip = clip;
            painted.push(layer);
        }
        let baseline = top + ((step - text.font.line_height) >> 1);
        if let Some(mut layer) = text_layer(
            text.font,
            text.size,
            name.as_bytes(),
            (list.left + unit * 2, baseline),
            palette.text[6],
        ) {
            layer.clip = clip;
            painted.push(layer);
        }
    }
}

/// The equipment page's panels for the mode, empty [guess: the kit lists
/// and the skills panel are not drawn yet].
fn paint_equipment(creator: &Creator, palette: &SkinPalette, painted: &mut Painted) {
    let unit = creator.unit;
    let d = creator.design();
    let tall = !creator.compact;
    let panels = create::equipment_panels(creator.entry.game_mode, false);
    let layout = create::creation_form_layout(d, creator.entry.screen_mode, tall);
    let mut places = Vec::new();
    if panels.apparel {
        places.push(layout.apparel);
    }
    if panels.weapons {
        places.push(layout.weapons);
    }
    if panels.skills {
        places.push(layout.skills);
    }
    for p in places {
        let rect = GuiRect::new(px(p.x, unit), px(p.y, unit), px(p.w, unit), px(p.h, unit));
        let child = gui_paint::paint_child(rect, unit, palette, EDITOR_ATTRIBUTES, 0x80, 0, None);
        painted.layers.extend(child.layers);
    }
}

/// The name warning's box, centred [guess at its geometry: the message
/// box's own form is not ported].
fn paint_warning(
    creator: &Creator,
    warning: NameWarning,
    palette: &SkinPalette,
    font: &GuiFont,
    size: Option<(u16, u16)>,
    painted: &mut Painted,
) {
    let unit = creator.unit;
    let (w, h) = (px(320, unit), px(96, unit));
    let rect = GuiRect::new((creator.screen.0 - w) / 2, (creator.screen.1 - h) / 2, w, h);
    let mut frame = Frame::new(rect, unit, warning.title);
    frame.attributes = gui_paint::FORM_ATTRIBUTES | 1;
    let box_paint = gui_paint::paint_frame(&frame, palette, font, size);
    painted.layers.extend(box_paint.layers);
    let width = font.text_width(warning.text.as_bytes());
    if let Some(layer) = text_layer(
        font,
        size,
        warning.text.as_bytes(),
        (rect.left + (w - width) / 2, rect.top + h / 2),
        palette.text[6],
    ) {
        painted.push(layer);
    }
}

/// The background picker's paint: the backdrop, the four cards from the
/// portrait strip, the chosen background's name and its description
/// wrapped in the 145 × 60 unit box.
pub fn paint_class_select(
    picker: &ClassSelect,
    screen: (i32, i32),
    unit: i32,
    textures: &GuiTextures,
) -> Painted {
    let palette = shown_palette(1.0);
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let mut painted = Painted::default();
    painted.push(rect_layer(
        &[(0.0, 0.0, screen.0 as f32, screen.1 as f32)],
        [0, 0, 0, 255],
    ));
    let draw = exa_ui::class_select::class_select_draw(
        (screen.0.max(0) as u32, screen.1),
        picker.available,
        picker.hover,
        unit,
    );
    if textures.get(STRIP).is_some() {
        let mut batch = SkinBatch::new();
        batch.select(3, 0, false);
        for card in &draw.cards {
            let [x, y, w, h] = card.rect.map(|v| v as f32);
            // Bottom-left, top-left, top-right, bottom-right.
            let corners = [(x, y + h), (x, y), (x + w, y), (x + w, y + h)];
            batch.set_ink(card.colour);
            batch.begin();
            for (corner, uv) in corners.iter().zip(card.uv) {
                batch.set_uv(uv[0], uv[1]);
                batch.vertex(corner.0, corner.1);
            }
            batch.end_quads();
        }
        painted.push(GuiLayer::new(batch, LayerTexture::Named(STRIP)));
    }
    let chosen = (picker.hover - 1).clamp(0, 3) as usize;
    if let Some(top) = draw.name_top {
        let name = BACKGROUND_NAMES[chosen].as_bytes();
        let left = exa_ui::class_select::name_left(screen.0.max(0) as u32, font.text_width(name));
        if let Some(layer) = text_layer(&font, size, name, (left, top), palette.text[6]) {
            painted.push(layer);
        }
    }
    if let Some([x, y, w, h]) = draw.description {
        // [guess] a greedy word wrap stands for the picker's text box.
        let mut line = String::new();
        let mut top = y;
        for word in BACKGROUND_DESCRIPTIONS[chosen].split(' ') {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if font.text_width(candidate.as_bytes()) > w && !line.is_empty() {
                if let Some(layer) =
                    text_layer(&font, size, line.as_bytes(), (x, top), palette.text[6])
                {
                    painted.push(layer);
                }
                top += font.line_height;
                line = word.to_string();
            } else {
                line = candidate;
            }
            if top > y + h + font.line_height * 4 {
                break;
            }
        }
        if !line.is_empty()
            && let Some(layer) = text_layer(&font, size, line.as_bytes(), (x, top), palette.text[6])
        {
            painted.push(layer);
        }
    }
    painted
}

fn paint(
    screen: Res<CreatorScreen>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    if !screen.is_open() {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    if let Some(picker) = &screen.class_select
        && let Ok(window) = windows.single()
    {
        let (size, unit) = screen_and_unit(window);
        paint_class_select(picker, size, unit, textures).submit(&mut frames);
    }
    if let Some(creator) = &screen.creator {
        paint_creator(creator, textures).submit(&mut frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creator() -> Creator {
        Creator::open(
            CreatorEntry::story(0),
            entry_record(CreatorEntry::story(0), None),
            (1024, 768),
            4,
            vec!["hair a".into(), "hair b".into()],
        )
    }

    fn click(c: &mut Creator, at: (i32, i32)) -> Vec<CreatorAction> {
        let mut out = c.pointer(at.0, at.1);
        out.extend(c.press(at.0, at.1));
        out.extend(c.release(at.0, at.1));
        out
    }

    fn control_centre(c: &Creator, part: Part, i: usize) -> (i32, i32) {
        let (ox, oy) = c.origin(part);
        let r = &c.controls(part)[i];
        (ox + r.x + r.w / 2, oy + r.y + r.h / 2)
    }

    #[test]
    fn the_words_open_the_modes_the_menu_opens() {
        assert_eq!(CreatorEntry::parse("story"), Some(CreatorEntry::story(0)));
        assert_eq!(CreatorEntry::parse("story:3"), Some(CreatorEntry::story(3)));
        assert_eq!(CreatorEntry::parse("story:4"), None);
        assert_eq!(
            CreatorEntry::parse("expert").and_then(|e| e.ladder),
            Some(Ladder::Expert)
        );
        assert_eq!(CreatorEntry::parse("recruit").unwrap().screen_mode, 10);
        assert_eq!(CreatorEntry::parse("versus").unwrap().game_mode, 0x1010);
        assert_eq!(CreatorEntry::parse("nope"), None);
    }

    #[test]
    fn the_form_places_its_editors_and_viewport_at_unit_four() {
        let c = creator();
        assert_eq!(c.design(), (1024, 768));
        assert!(!c.compact);
        assert_eq!(c.base_rect(), GuiRect::new(4, 60, 172, 592));
        assert_eq!(c.hair_rect(), GuiRect::new(1024 - 176, 32, 172, 612));
        assert_eq!(c.preview_px(), GuiRect::new(180, 24, 1024 - 360, 768 - 28));
        assert_eq!(c.physique_rect(), GuiRect::new(12, 124, 160, 160));
        assert_eq!(c.tone_rect(), GuiRect::new(12, 388, 160, 160));
        assert_eq!(c.hair_colour_rect(), GuiRect::new(852, 56, 160, 112));
        let (list, step) = c.hair_list_rect();
        assert_eq!(list, GuiRect::new(852, 32 + 224, 152, 612 - 216));
        assert_eq!(step, 56);
        // The gloss trackbar sits 120 under the picker.
        assert_eq!((c.hair[hair::GLOSS].x, c.hair[hair::GLOSS].y), (4, 144));
    }

    #[test]
    fn the_layout_scales_to_the_live_unit() {
        let c = Creator::open(
            CreatorEntry::practice(Ladder::Novice),
            CharacterRecord::reset(),
            (1920, 1080),
            5,
            Vec::new(),
        );
        assert_eq!(c.design(), (1536, 864));
        // The body editor: 4 / 4 · 5, 60 / 4 · 5 and 172 / 4 · 5.
        assert_eq!(c.base_rect(), GuiRect::new(5, 75, 215, 740));
        assert_eq!(c.base[base::PITCH].knob, 22);
    }

    #[test]
    fn the_editors_write_the_record() {
        let mut c = creator();
        assert!(c.take_regeneration());
        // The female radio sets bit 0 of the profession word.
        let at = control_centre(&c, Part::Base, base::FEMALE);
        click(&mut c, at);
        assert!(c.record.female);
        assert_eq!(c.base[base::MALE].state & state::ON, 0);
        // The tone picker's corner writes the skin words.
        let r = c.tone_rect();
        click(&mut c, (r.left, r.top));
        assert_eq!(c.record.skin, [0, 0]);
        // The physique picker's point goes into the record.
        let r = c.physique_rect();
        click(&mut c, (r.left + 40, r.top + 120));
        assert_eq!(c.record.physique, [0.25, 0.75]);
        // A hair style row (row 2 is the second entry).
        let (list, step) = c.hair_list_rect();
        click(&mut c, (list.left + 5, list.top + step * 2 + 1));
        assert_eq!(c.record.hair_style, 2);
        // The record changed: the preview waits two frames, then rebuilds.
        assert!(!c.take_regeneration());
        c.tick();
        c.tick();
        assert!(c.take_regeneration());
        assert!(!c.take_regeneration());
    }

    #[test]
    fn the_voice_buttons_step_the_voice_and_its_letter() {
        let mut c = creator();
        let next = control_centre(&c, Part::Base, base::VOICE_NEXT);
        click(&mut c, next);
        assert_eq!((c.voice, c.record.voice_number), (1, 1));
        assert_eq!(c.base[base::VOICE_LETTER].text, b"B");
        let previous = control_centre(&c, Part::Base, base::VOICE_PREVIOUS);
        click(&mut c, previous);
        click(&mut c, previous);
        assert_eq!(c.voice, 0);
        let play = control_centre(&c, Part::Base, base::VOICE_PLAY);
        assert_eq!(click(&mut c, play), vec![CreatorAction::PlayVoice(0)]);
    }

    #[test]
    fn next_asks_for_a_name_then_shows_the_equipment_page_and_commits() {
        let mut c = creator();
        let next = control_centre(&c, Part::Main, creation::NEXT);
        assert!(click(&mut c, next).is_empty());
        assert_eq!(c.warning, Some(NameWarning::missing()));
        // A click closes the warning.
        c.release(0, 0);
        assert!(c.warning.is_none());
        let name = control_centre(&c, Part::Main, creation::NAME);
        click(&mut c, name);
        assert!(c.name_focused);
        c.type_name("Derrin\u{e9}");
        assert_eq!(c.name.text, b"Derrin");
        click(&mut c, next);
        assert_eq!(c.page, CreationScreenPage::Equipment);
        assert_eq!(c.main[creation::NEXT].text, b"BEGIN");
        assert_eq!(c.main[creation::NAME_LABEL].text, b"APPAREL");
        let actions = click(&mut c, next);
        let [CreatorAction::Commit { kind, record }] = actions.as_slice() else {
            panic!("{actions:?}");
        };
        assert_eq!(*kind, create::CommitKind::Begin);
        assert_eq!(record.name, "Derrin");
        assert_eq!(record.pools, Pools::FULL);
    }

    #[test]
    fn back_returns_a_page_then_leaves_to_the_menu() {
        let mut c = creator();
        c.name.text = b"A".to_vec();
        c.advance();
        assert_eq!(c.page, CreationScreenPage::Equipment);
        assert_eq!(c.back(), CreatorAction::None);
        assert_eq!(c.page, CreationScreenPage::Appearance);
        assert_eq!(c.back(), CreatorAction::Leave(PageDestination::Menu));
        let mut r = Creator::open(
            CreatorEntry::recruit(),
            CharacterRecord::reset(),
            (1024, 768),
            4,
            Vec::new(),
        );
        assert_eq!(r.back(), CreatorAction::Leave(PageDestination::Arena));
    }

    #[test]
    fn the_story_record_keeps_only_the_braies_and_full_health() {
        let template = CharacterRecord {
            worn: vec![
                exa_formats::characters::ItemRef::new(8, 0),
                exa_formats::characters::ItemRef::new(0x40, 0),
            ],
            ..CharacterRecord::reset()
        };
        // The unknown background takes a new record, not the template.
        let unknown = entry_record(CreatorEntry::story(0), Some(template.clone()));
        assert_eq!(unknown.pools, Pools::FULL);
        assert!(unknown.worn.iter().all(|w| w.index() == 8));
        // A named background without a preset takes its template.
        let knight = entry_record(CreatorEntry::story(1), Some(template));
        assert_eq!(knight.worn.len(), 2);
    }

    #[test]
    fn the_paint_draws_the_form_and_clears_the_viewport() {
        let c = creator();
        let painted = paint_creator(&c, &GuiTextures::default());
        assert!(painted.layers.len() > 20);
        let clear = painted
            .layers
            .iter()
            .any(|l| l.batch.blend == exa_ui::gui_draw::BLEND_REPLACE);
        assert!(clear);
        let picker = ClassSelect {
            hover: 2,
            available: 7,
        };
        let painted = paint_class_select(&picker, (1920, 1080), 5, &GuiTextures::default());
        assert!(!painted.layers.is_empty());
    }

    #[test]
    fn the_menu_rows_open_the_screens_in_an_app() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_plugins(crate::menu_screen::MenuScreenPlugin)
            .add_plugins(CreatorScreenPlugin);
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::BeginStory));
        app.update();
        app.update();
        let screen = app.world().resource::<CreatorScreen>();
        assert!(screen.class_select.is_some());
        // Without a window the creator cannot size itself and waits.
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::Practice { expert: true }));
        app.update();
        app.update();
        assert!(app.world().resource::<CreatorScreen>().creator.is_none());
    }
}
