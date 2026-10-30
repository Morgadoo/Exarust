//! The main menu's clock conversion.

/// Quantises a 32-bit clock to 1/135 and scales it by 1/8.
pub fn menu_clock_scale(value: i32) -> f32 {
    let step = (f64::from(value) * 0.007407407407407408).round_ties_even();
    ((step as i64) as f64 * 0.125) as f32
}

/// The result of the journal/loot key override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuKeyClose {
    pub closed: bool,
    pub result: i32,
}

/// Closes on a mask carrying both 0x2000 and 0x2; otherwise runs the base handler.
pub fn menu_key_close(mask: u16, base: impl FnOnce() -> i32) -> MenuKeyClose {
    if (mask & 0x2002) == 0x2002 {
        MenuKeyClose {
            closed: true,
            result: 1,
        }
    } else {
        MenuKeyClose {
            closed: false,
            result: base(),
        }
    }
}

/// The class-select result: the stored index, whether its sound played, and the return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassSelect {
    pub index: i32,
    pub played: bool,
    pub result: i32,
}

/// Stores the class index and plays its sound when both it and the flag are positive.
pub fn select_menu_class(index: i32, sound_flag: i32) -> ClassSelect {
    ClassSelect {
        index,
        played: index > 0 && sound_flag > 0,
        result: 3,
    }
}

/// The skill tooltip's table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTipPlace {
    pub x: i32,
    pub y: i32,
    pub icon: u64,
    pub text: u64,
}

/// The tooltip's icon position, label rectangle and counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTipDraw {
    pub icon_at: [i32; 2],
    pub label: [i32; 4],
    pub icon: u64,
    pub text: u64,
    pub begin_calls: u32,
    pub bind_calls: u32,
    pub setup_calls: u32,
    pub draw_calls: u32,
}

/// Draws the skill tooltip's icon at two by five units and its label beneath.
pub fn draw_skill_tip(place: SkillTipPlace, unit: i32) -> SkillTipDraw {
    SkillTipDraw {
        icon_at: [unit.wrapping_mul(2), unit.wrapping_mul(5)],
        label: [
            place.x.wrapping_sub(unit.wrapping_mul(4)),
            place.y,
            unit.wrapping_mul(6),
            unit.wrapping_mul(2),
        ],
        icon: place.icon,
        text: place.text,
        begin_calls: 1,
        bind_calls: 1,
        setup_calls: 2,
        draw_calls: 2,
    }
}

/// The transition screen's fixed text.
pub const TRANSITION_TEXT: &str = "TO BE CONTINUED";

/// The transition fade's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionFadePlace {
    pub page: i32,
    pub x: i32,
    pub y: i32,
    pub colour: [u8; 3],
    pub alpha: u8,
    pub glyph_width: i32,
    pub glyph_height: i32,
}

/// The transition fade's draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionFade {
    pub active: bool,
    pub blend_mode: i32,
    pub colour: [u8; 4],
    pub text: &'static str,
    pub origin: [i32; 2],
    pub begin_calls: u32,
    pub draw_calls: u32,
    pub font_calls: u32,
    pub base_calls: u32,
}

/// Draws the transition screen's centred text on page 2.
pub fn draw_transition_fade(place: TransitionFadePlace) -> TransitionFade {
    if place.page != 2 {
        return TransitionFade {
            active: false,
            blend_mode: 0,
            colour: [0; 4],
            text: TRANSITION_TEXT,
            origin: [0, 0],
            begin_calls: 0,
            draw_calls: 0,
            font_calls: 0,
            base_calls: 1,
        };
    }
    let width = 15i64 * i64::from(place.glyph_width);
    let x = (((i64::from(place.x) - width) as u64) >> 1) as u32 as i32;
    let y = (((i64::from(place.y) - i64::from(place.glyph_height)) as u64) >> 1) as u32 as i32;
    TransitionFade {
        active: true,
        blend_mode: 0x203,
        colour: [
            place.colour[0],
            place.colour[1],
            place.colour[2],
            place.alpha,
        ],
        text: TRANSITION_TEXT,
        origin: [x, y],
        begin_calls: 1,
        draw_calls: 1,
        font_calls: 1,
        base_calls: 1,
    }
}

/// The settled skill index and the object flags after the page rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillSelection {
    pub index: i32,
    pub flags: u32,
}

/// Clamps the low 16 bits to eight and toggles bits 0x110 on page 1.
pub fn settle_skill_selection(raw: i32, page: i32, flags: u32) -> SkillSelection {
    let index = raw & 0xffff;
    let index = if index > 8 { 0 } else { index };
    let flags = if page == 1 {
        if index > 0 {
            flags | 0x110
        } else {
            flags & !0x110
        }
    } else {
        flags
    };
    SkillSelection { index, flags }
}

/// The inventory's close result and which path closed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryKeyClose {
    pub closed: bool,
    pub callback: bool,
    pub result: i32,
}

/// Closes the inventory on the 0x2002 mask through its callback or the form close.
pub fn inventory_key_close(
    mask: u16,
    has_callback: bool,
    base: impl FnOnce() -> i32,
) -> InventoryKeyClose {
    if (mask & 0x2002) == 0x2002 {
        InventoryKeyClose {
            closed: true,
            callback: has_callback,
            result: 1,
        }
    } else {
        InventoryKeyClose {
            closed: false,
            callback: false,
            result: base(),
        }
    }
}

/// The skill-selector key outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillKey {
    pub closed: bool,
    pub result: i32,
}

/// Routes a skill-selector key to the base handler or to the close with 0x40.
pub fn skill_selector_key(
    selection: i32,
    page: i32,
    word: i32,
    mode_byte: u8,
    base: impl FnOnce() -> i32,
) -> SkillKey {
    let base_path = (selection > 0 && page > 1) || ((word & 0xffff) == 9 && mode_byte == 0);
    if base_path {
        SkillKey {
            closed: false,
            result: base(),
        }
    } else {
        SkillKey {
            closed: true,
            result: 0x40,
        }
    }
}

/// A form a menu hands over to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuStep {
    /// The form a confirmed class opens.
    ClassForm,
}

/// What a click on the end-session menu runs, by the game mode first and
/// the clicked item second. The routines behind the transition screen take
/// a music-fade flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndSessionAction {
    /// Practice's victory menu, first item: the prelude closes, the
    /// experience system and the character screen are reset, then the
    /// session leaves to the menu form behind the transition (music fades).
    PracticeRestart,
    /// Practice's next fight behind the transition; the music fades only
    /// into the last fight (`last`), whose boss track replaces it.
    NextPracticeFight { last: bool },
    /// Back to the title behind the transition (music fades): practice's and
    /// the arena's second item.
    BackToMenu,
    /// The story's reconstructed menu, first item: the clicked item is
    /// cleared before the respawn runs.
    StoryRespawn,
    /// The story's other first items and its second item: the story ends.
    StoryQuit,
    /// The arena's first item: the arena unit's continue.
    ArenaContinue,
    /// Any other mode: the first registered mode handler whose mode word
    /// matches, given the clicked item.
    Registered { handler: usize, item: i32 },
}

/// The end-session menu's own constructor values, set after the main
/// menu's base constructor: the scale 0.9, two byte flags clear and one base
/// word set to 1 (the uses of the flags and the word are not traced).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndSessionForm {
    pub scale: f32,
    pub first_flag: bool,
    pub second_flag: bool,
    pub base_word: i32,
}

/// The constructor's values.
pub fn end_session_form() -> EndSessionForm {
    EndSessionForm {
        scale: 0.9,
        first_flag: false,
        second_flag: false,
        base_word: 1,
    }
}

/// The game mode words the end-session dispatch branches on.
pub const END_PRACTICE: u32 = 2;
pub const END_STORY: u32 = 4;
pub const END_ARENA: u32 = 8;
/// The menu numbers its first item tests: practice's victory menu and the
/// story's reconstructed-player menu.
pub const END_MENU_WON: i32 = 2;
pub const END_MENU_RECONSTRUCTED: i32 = 4;

/// The end-session dispatch's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndSessionInput {
    /// The game mode word.
    pub mode: u32,
    /// The clicked item, 1-based (0 or less: none).
    pub item: i32,
    /// The menu's number.
    pub menu: i32,
    /// The practice ladder's fight count and the current fight.
    pub fights: i32,
    pub fight: i32,
}

/// The dispatched action and the cleared fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndSessionDispatch {
    pub action: Option<EndSessionAction>,
    /// The clicked item and the item the pointer went down on are cleared
    /// after every click, whatever ran.
    pub clears_item: bool,
    pub clears_pressed_item: bool,
    pub result: i32,
}

/// The end-session menu's click. Practice (2), story (4) and arena (8)
/// branch on the clicked item; any other mode word (below 2 included, as the
/// first compare is unsigned) runs the first registered handler whose mode
/// word matches, `handlers` being the registered modes in order. The last
/// practice fight is the one whose index is the fight count less one, in
/// 64-bit arithmetic.
pub fn dispatch_end_session(input: EndSessionInput, handlers: &[u32]) -> EndSessionDispatch {
    let action = match input.mode {
        END_PRACTICE => match input.item {
            1 if input.menu == END_MENU_WON => Some(EndSessionAction::PracticeRestart),
            1 => Some(EndSessionAction::NextPracticeFight {
                last: i64::from(input.fights) - 1 == i64::from(input.fight),
            }),
            2 => Some(EndSessionAction::BackToMenu),
            _ => None,
        },
        END_STORY => match input.item {
            1 if input.menu == END_MENU_RECONSTRUCTED => Some(EndSessionAction::StoryRespawn),
            1 | 2 => Some(EndSessionAction::StoryQuit),
            _ => None,
        },
        END_ARENA => match input.item {
            1 => Some(EndSessionAction::ArenaContinue),
            2 => Some(EndSessionAction::BackToMenu),
            _ => None,
        },
        mode => handlers
            .iter()
            .position(|&registered| registered == mode)
            .map(|handler| EndSessionAction::Registered {
                handler,
                item: input.item,
            }),
    };
    EndSessionDispatch {
        action,
        clears_item: true,
        clears_pressed_item: true,
        result: 0,
    }
}

/// The main menu's hover state after one update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuHover {
    pub selection: i32,
    pub sound: bool,
    pub stamp: i32,
    pub result: i32,
}

/// Updates the main menu's hover and rate-limits its sound to 200 units.
/// `pressed_item` is the item the pointer went down on (0 none): only it
/// highlights while hovered.
pub fn update_menu_hover(
    hover: i32,
    pressed_item: i32,
    previous: i32,
    stamp: i32,
    now: i32,
) -> MenuHover {
    let candidate = if hover == pressed_item {
        pressed_item
    } else {
        0
    };
    if candidate == previous {
        return MenuHover {
            selection: candidate,
            sound: false,
            stamp,
            result: 0,
        };
    }
    let fresh = stamp.wrapping_add(0xc8) < now;
    MenuHover {
        selection: candidate,
        sound: fresh && candidate != 0,
        stamp: if fresh { now } else { stamp },
        result: 3,
    }
}

/// One main-menu key dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuKeyAction {
    Confirm { index: i32, arg: u64 },
    Forward,
    Back,
    Move(i32),
    None,
}

/// The main-menu key's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuKeyInput {
    pub mask: u32,
    pub selection: i32,
    pub limit: i32,
    pub confirm_arg: u64,
    pub sub_result: i32,
}

/// The main-menu key's dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuKey {
    pub action: MainMenuKeyAction,
    pub selection: i32,
    pub sound: bool,
    pub result: i32,
}

/// Routes the main menu's confirm, forward, back and direction keys.
pub fn main_menu_key(input: MainMenuKeyInput) -> MainMenuKey {
    let idle = MainMenuKey {
        action: MainMenuKeyAction::None,
        selection: input.selection,
        sound: false,
        result: 0,
    };
    if (input.mask & 0x100) != 0 {
        if input.selection > 0 {
            return MainMenuKey {
                action: MainMenuKeyAction::Confirm {
                    index: input.selection,
                    arg: input.confirm_arg,
                },
                selection: input.selection,
                sound: true,
                result: input.sub_result,
            };
        }
        return idle;
    }
    if (input.mask & 0x200) != 0 {
        return MainMenuKey {
            action: MainMenuKeyAction::Forward,
            result: input.sub_result,
            ..idle
        };
    }
    if (input.mask & 0x400) != 0 {
        return MainMenuKey {
            action: MainMenuKeyAction::Back,
            result: input.sub_result,
            ..idle
        };
    }
    if (input.mask & 3) != 0 {
        match input.mask & 0xf000 {
            0x8000 if input.selection < input.limit => {
                return MainMenuKey {
                    action: MainMenuKeyAction::Move((input.selection + 1).max(1)),
                    result: input.sub_result,
                    ..idle
                };
            }
            0x4000 if input.selection > 1 => {
                return MainMenuKey {
                    action: MainMenuKeyAction::Move(input.selection - 1),
                    result: input.sub_result,
                    ..idle
                };
            }
            _ => {}
        }
    }
    idle
}

/// The inventory detach's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryDetachInput {
    pub already_inactive: bool,
    pub has_owner: bool,
    pub has_child: bool,
    pub child_visible: bool,
    pub has_stack: bool,
    pub stack_count: i32,
    pub loot_pane: bool,
    pub base_result: bool,
}

/// The inventory detach's work and result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryDetach {
    pub inactive: bool,
    pub marked: bool,
    pub placed: bool,
    pub stack_count: i32,
    pub freed: bool,
    pub globals_copied: bool,
    pub base_calls: u32,
    pub result: bool,
}

/// Marks the owner, places the child and releases the stack before the base call.
pub fn detach_inventory(input: InventoryDetachInput) -> InventoryDetach {
    if input.already_inactive {
        return InventoryDetach {
            inactive: true,
            marked: false,
            placed: false,
            stack_count: input.stack_count,
            freed: false,
            globals_copied: false,
            base_calls: 0,
            result: true,
        };
    }
    let (count, freed) = if input.has_stack {
        if input.stack_count > 1 {
            (input.stack_count - 1, false)
        } else {
            (0, true)
        }
    } else {
        (input.stack_count, false)
    };
    InventoryDetach {
        inactive: false,
        marked: input.has_owner,
        placed: input.has_owner && input.has_child && input.child_visible,
        stack_count: count,
        freed,
        globals_copied: input.loot_pane,
        base_calls: 1,
        result: input.base_result,
    }
}

/// The inventory page override's field and result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryPage {
    pub field: i32,
    pub result: i32,
}

/// Stores page −3 when the argument is −1.
pub fn inventory_page(arg: i32, field: i32, base_result: i32) -> InventoryPage {
    InventoryPage {
        field: if arg == -1 { -3 } else { field },
        result: base_result,
    }
}

/// The special name the buffer submits.
pub const MENU_NAME_CODE: &[u8; 10] = b"barechoons";

/// The name buffer after one character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuName {
    pub buffer: [u8; 10],
    pub base: bool,
    pub matched: bool,
}

/// Shifts the name buffer and appends a lowered character.
pub fn type_menu_name(buffer: [u8; 10], ch: u8, keymask: u32) -> MenuName {
    if (keymask & 2) == 0 {
        return MenuName {
            buffer,
            base: true,
            matched: false,
        };
    }
    let mut out = [0u8; 10];
    out[..9].copy_from_slice(&buffer[1..10]);
    out[9] = if (0x41..=0x5a).contains(&ch) {
        ch.wrapping_add(0x20)
    } else {
        ch
    };
    MenuName {
        buffer: out,
        base: false,
        matched: &out == MENU_NAME_CODE,
    }
}

/// The splash screen's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplashBounds {
    pub screen: u32,
    pub second: i32,
    pub result: i32,
}

/// The splash bounds write the screen and two negative ones.
pub fn splash_bounds(screen: u32) -> SplashBounds {
    SplashBounds {
        screen,
        second: -1,
        result: -1,
    }
}

/// The splash destroy's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplashDestroyInput {
    pub free_flag: bool,
    pub first: bool,
    pub second: bool,
    pub count: i32,
}

/// The splash destroy's calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplashDestroy {
    pub release_calls: u32,
    pub global_cleared: bool,
    pub first_freed: bool,
    pub second_freed: bool,
    pub list_calls: u32,
    pub base_calls: u32,
    pub final_calls: u32,
}

/// Releases the splash screen's resources in the observed order.
pub fn destroy_splash(input: SplashDestroyInput) -> SplashDestroy {
    let free = u32::from(input.free_flag);
    SplashDestroy {
        release_calls: free,
        global_cleared: true,
        first_freed: input.first,
        second_freed: input.second,
        list_calls: u32::from(input.count > 0),
        base_calls: 1,
        final_calls: free,
    }
}

/// The skill selector's two draw modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSelectorMode {
    Title,
    Skill,
    Empty,
}

/// The skill selector's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillSelectorDrawPlace {
    pub raw: i32,
    pub page: i32,
    pub x0: i32,
    pub x1: i32,
    pub y1: i32,
    pub icon_width: i32,
}

/// The selector's title, icon and label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillSelectorDraw {
    pub mode: SkillSelectorMode,
    pub title: Option<[i32; 5]>,
    pub icon: Option<[i32; 3]>,
    pub label: Option<[i32; 5]>,
    pub begin_calls: u32,
    pub bind_calls: u32,
    pub font_calls: u32,
    pub draw_calls: u32,
}

/// Draws the skill selector's title or selected skill.
pub fn draw_skill_selector(place: SkillSelectorDrawPlace, unit: i32) -> SkillSelectorDraw {
    let width = place
        .x1
        .wrapping_sub(place.x0)
        .wrapping_sub(unit.wrapping_mul(2));
    let base = SkillSelectorDraw {
        mode: SkillSelectorMode::Empty,
        title: None,
        icon: None,
        label: None,
        begin_calls: 1,
        bind_calls: 1,
        font_calls: 0,
        draw_calls: 0,
    };
    if place.page == 2 {
        return SkillSelectorDraw {
            mode: SkillSelectorMode::Title,
            title: Some([unit.wrapping_mul(12), width, 0, unit.wrapping_mul(20), 1]),
            font_calls: 1,
            draw_calls: 1,
            ..base
        };
    }
    let index = (place.raw & 0xffff).wrapping_sub(1);
    if (0..8).contains(&index) {
        let icon_x = place
            .x1
            .wrapping_sub(place.x0)
            .wrapping_sub(place.icon_width)
            / 2;
        return SkillSelectorDraw {
            mode: SkillSelectorMode::Skill,
            icon: Some([icon_x, unit.wrapping_mul(16), place.icon_width]),
            label: Some([
                unit.wrapping_mul(18),
                width,
                0,
                place.y1.wrapping_sub(unit.wrapping_mul(20)),
                1,
            ]),
            font_calls: 2,
            draw_calls: 2,
            ..base
        };
    }
    base
}

/// The skill tooltip show's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTipShowInput {
    pub found: bool,
    pub anchor: [i32; 2],
    /// The selector's skill (1-based), the first index into the technique
    /// texts.
    pub skill_index: i32,
    /// The hovered row's technique (1-based), the second index.
    pub technique_index: i32,
    pub unit: i32,
    /// The GUI scale index (0 the smallest scale, which takes the taller
    /// box).
    pub gui_level: i32,
    /// When the tooltip's fade-in ends.
    pub fade_end: i32,
    pub stamp_base: i32,
}

/// The tooltip show's outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTipShow {
    pub shown: bool,
    pub index: Option<i32>,
    pub threshold: Option<i32>,
    pub flags_or: u32,
    pub place: [i32; 2],
    pub height: i32,
    pub visible: bool,
    pub calls: u32,
    pub stamp: i32,
}

/// Places the skill tooltip from the found row.
pub fn show_skill_tip(input: SkillTipShowInput) -> SkillTipShow {
    if !input.found {
        return SkillTipShow {
            shown: false,
            index: None,
            threshold: None,
            flags_or: 0,
            place: [0, 0],
            height: 0,
            visible: false,
            calls: 0,
            stamp: 0,
        };
    }
    SkillTipShow {
        shown: true,
        index: Some(input.skill_index - 1),
        threshold: Some(input.technique_index - 1),
        flags_or: 0x10,
        place: [
            input.anchor[0],
            input.unit.wrapping_mul(9).wrapping_add(input.anchor[1]),
        ],
        height: if input.gui_level == 0 {
            input.unit.wrapping_mul(24)
        } else {
            input.unit.wrapping_mul(18)
        },
        visible: true,
        calls: 3,
        stamp: input
            .stamp_base
            .wrapping_mul(3)
            .wrapping_add(input.fade_end),
    }
}

/// The char-loot show's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharLootShowInput {
    pub already_inactive: bool,
    pub has_owner: bool,
    pub has_child: bool,
    pub child_visible: bool,
    pub base_result: bool,
}

/// The char-loot show's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharLootShow {
    pub inactive: bool,
    pub marked: bool,
    pub placed: bool,
    pub base_calls: u32,
    pub result: bool,
}

/// Marks the owner and places a visible child when the loot screen becomes active.
pub fn show_char_loot(input: CharLootShowInput) -> CharLootShow {
    if input.already_inactive {
        return CharLootShow {
            inactive: true,
            marked: false,
            placed: false,
            base_calls: 0,
            result: true,
        };
    }
    CharLootShow {
        inactive: false,
        marked: input.has_owner,
        placed: input.has_owner && input.has_child && input.child_visible,
        base_calls: 1,
        result: input.base_result,
    }
}

/// The journal's origin, table slot and string assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalShow {
    pub origin: [i32; 2],
    pub table_offset: i32,
    pub assigned: bool,
    pub base_calls: u32,
}

/// Copies the journal's origin and assigns its page string.
pub fn show_journal(x: i32, y: i32, page: i32) -> JournalShow {
    JournalShow {
        origin: [x, y],
        table_offset: page.wrapping_mul(0x18).wrapping_add(0x10),
        assigned: true,
        base_calls: 1,
    }
}

/// The character view destroy's calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewDestroy {
    pub release_calls: u32,
    pub base_calls: u32,
    pub final_calls: u32,
}

/// Releases the view on a set free flag and always runs the base destructor.
pub fn destroy_character_view(free_flag: bool) -> ViewDestroy {
    let free = u32::from(free_flag);
    ViewDestroy {
        release_calls: free,
        base_calls: 1,
        final_calls: free,
    }
}

/// The character view clear's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewClear {
    pub had_view: bool,
    pub release_calls: u32,
    pub slot_cleared: bool,
    pub base_calls: u32,
    pub result: bool,
}

/// Releases the view and clears its owner slot before the base call.
pub fn clear_character_view(has_view: bool, base_result: bool) -> ViewClear {
    ViewClear {
        had_view: has_view,
        release_calls: u32::from(has_view),
        slot_cleared: has_view,
        base_calls: 1,
        result: base_result,
    }
}

/// The settings menu's opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsOpen {
    /// Whether the menu was created (it did not exist).
    pub created: bool,
    /// Whether an existing menu was re-shown instead.
    pub shown: bool,
    /// Whether the menu was laid out.
    pub laid_out: bool,
    /// The entry the resume flag selects.
    pub selected: Option<i32>,
    /// The word the open finishes with.
    pub finished: i32,
}

/// Opens the settings menu: creates and attaches it when none exists,
/// otherwise re-shows it, lays it out, selects entry 1 behind the resume
/// flag and finishes with -1.
pub fn open_settings_menu(exists: bool, select: bool) -> SettingsOpen {
    SettingsOpen {
        created: !exists,
        shown: exists,
        laid_out: true,
        selected: if select { Some(1) } else { None },
        finished: -1,
    }
}

/// Closes an existing settings menu (and only an existing one).
pub fn close_settings_menu(exists: bool) -> bool {
    exists
}

/// Clears an existing settings menu's flag (and only an existing one).
pub fn reset_settings_menu(exists: bool) -> bool {
    exists
}

/// A node's record: the flags and the signed word at 0x468.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewNodeRecord {
    pub flags: u32,
    pub value: i32,
}

/// A character-view chain node: its next node and its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewNode {
    pub next: Option<usize>,
    pub record: Option<ViewNodeRecord>,
}

/// Walks the chain from `start` and stops at the first node whose record is
/// missing, or whose flags carry both 0x12000000 with a negative word; any
/// other node follows its next.
pub fn first_settled_view_node(nodes: &[ViewNode], start: usize) -> Option<usize> {
    let mut current = start;
    loop {
        let node = nodes.get(current)?;
        match node.record {
            None => return Some(current),
            Some(record) => {
                if (record.flags & 0x12000000) == 0x12000000 && record.value < 0 {
                    return Some(current);
                }
                current = node.next?;
            }
        }
    }
}

/// The skill selector's applied value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSelectorApplied {
    Selected(i32),
    None,
}

/// The skill selector show's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillSelectorShow {
    pub applied: SkillSelectorApplied,
    pub flags_cleared: bool,
    pub base_calls: u32,
}

/// Clears the selector's flags and applies its selection or the special −1.
pub fn show_skill_selector(has_object: bool, selection: i32, word: i32) -> SkillSelectorShow {
    let mut applied = SkillSelectorApplied::None;
    let mut flags_cleared = false;
    if has_object {
        flags_cleared = true;
        if selection > 0 {
            applied = SkillSelectorApplied::Selected(selection);
        } else {
            let low = word & 0xffff;
            if low == 9 || low == 10 {
                applied = SkillSelectorApplied::Selected(-1);
            }
        }
    }
    SkillSelectorShow {
        applied,
        flags_cleared,
        base_calls: 1,
    }
}

/// The class-select confirm's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassConfirm {
    pub selected: Option<i32>,
    pub global_index: Option<i32>,
    pub sound: bool,
    pub page_copied: bool,
    pub form: Option<MenuStep>,
    pub form_flag: bool,
    pub result: i32,
}

/// Stores the class, plays its sound and opens the next form.
pub fn confirm_class(selection: i32, sound_ready: bool) -> ClassConfirm {
    if selection <= 0 {
        return ClassConfirm {
            selected: None,
            global_index: None,
            sound: false,
            page_copied: false,
            form: None,
            form_flag: false,
            result: 0x40,
        };
    }
    ClassConfirm {
        selected: Some(selection),
        global_index: Some(selection - 1),
        sound: sound_ready,
        page_copied: true,
        form: Some(MenuStep::ClassForm),
        form_flag: false,
        result: 7,
    }
}

/// One class-navigation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassNavAction {
    Confirm,
    Forward,
    Move(i32),
    None,
}

/// The class navigation's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassNavInput {
    pub mask: u32,
    pub selection: i32,
    pub available: u32,
    pub sound_ready: bool,
}

/// The class navigation's action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassNav {
    pub action: ClassNavAction,
    pub sound: bool,
    pub form: Option<MenuStep>,
    pub result: i32,
}

/// Confirms, forwards, backs or moves the class selection.
pub fn navigate_class(input: ClassNavInput) -> ClassNav {
    let idle = ClassNav {
        action: ClassNavAction::None,
        sound: false,
        form: None,
        result: 0,
    };
    if (input.mask & 0x100) != 0 {
        if input.selection <= 0 {
            return idle;
        }
        return ClassNav {
            action: ClassNavAction::Confirm,
            sound: input.sound_ready,
            form: Some(MenuStep::ClassForm),
            result: 7,
        };
    }
    if (input.mask & 0x200) != 0 || (input.mask & 0x400) != 0 {
        return ClassNav {
            action: ClassNavAction::Forward,
            ..idle
        };
    }
    if (input.mask & 3) == 0 {
        return idle;
    }
    let avail = (input.available << 1) | 1;
    let target = match input.mask & 0xf000 {
        0x8000 => {
            let mut target = input.selection.max(0);
            if target <= 3 {
                let mut index = target;
                loop {
                    if ((avail >> index) & 1) != 0 {
                        target = index + 1;
                        break;
                    }
                    if index >= 3 {
                        break;
                    }
                    index += 1;
                }
            }
            target
        }
        0x4000 => {
            if input.selection <= 2 {
                1
            } else {
                let mut index = input.selection - 2;
                let mut target = input.selection;
                loop {
                    if ((avail >> index) & 1) != 0 {
                        target = index + 1;
                        break;
                    }
                    if index <= 0 {
                        break;
                    }
                    index -= 1;
                }
                target
            }
        }
        _ => input.selection,
    };
    if target != input.selection {
        ClassNav {
            action: ClassNavAction::Move(target),
            ..idle
        }
    } else {
        idle
    }
}

/// The class-card hit test's inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassCardHitInput {
    pub hit: [i32; 2],
    pub available: u32,
    pub screen_w: i32,
    pub clock: i32,
    pub scale: f32,
}

/// The hit card's ordinal and the band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassCardHit {
    pub index: Option<i32>,
    pub count: i32,
    pub top: i32,
    pub height: i32,
}

/// Maps a point to one of the laid-out class cards.
pub fn hit_class_cards(input: ClassCardHitInput) -> ClassCardHit {
    let count = 1
        + i32::from((input.available & 1) != 0)
        + i32::from((input.available & 2) != 0)
        + i32::from((input.available & 4) != 0);
    let width = (input.scale * 256.0).round_ties_even() as i32;
    let step = input.scale * 288.0;
    let height = (input.scale * 512.0).round_ties_even() as i32;
    let top = ((input.clock as f32 - input.scale * 1024.0).round_ties_even() as i32) / 2
        + (input.scale * 140.0).round_ties_even() as i32;
    let miss = ClassCardHit {
        index: None,
        count,
        top,
        height,
    };
    if input.hit[1] <= top || input.hit[1] >= top + height {
        return miss;
    }
    let mut slot = 0i32;
    for ordinal in 0..4u32 {
        if ordinal > 0 && (input.available & (1 << (ordinal - 1))) == 0 {
            continue;
        }
        let offset = (slot as f32 - count as f32 * 0.5) * step;
        let left = input.screen_w / 2 + offset.round_ties_even() as i32;
        slot += 1;
        if input.hit[0] > left && input.hit[0] < left + width {
            return ClassCardHit {
                index: Some(ordinal as i32 + 1),
                count,
                top,
                height,
            };
        }
    }
    miss
}

/// The main-menu confirm's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuConfirm {
    pub selected: Option<i32>,
    pub copy_to_204: bool,
    pub sound: bool,
    pub result: i32,
}

/// Copies the selection to field 0x204 and confirms on the 0x1000 direction.
pub fn confirm_main_menu(selection: i32, mask: u32, sound_ready: bool) -> MenuConfirm {
    if selection <= 0 {
        return MenuConfirm {
            selected: None,
            copy_to_204: true,
            sound: false,
            result: 0x40,
        };
    }
    if (mask & 0xf000) != 0x1000 {
        return MenuConfirm {
            selected: None,
            copy_to_204: false,
            sound: false,
            result: 0x40,
        };
    }
    MenuConfirm {
        selected: Some(selection),
        copy_to_204: true,
        sound: sound_ready,
        result: 7,
    }
}

/// The main menu's forward action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuForward {
    /// The action the page runs: 0 on page 1 and 1 on page 8.
    pub action: Option<u8>,
    /// Whether the sound plays (the input must be ready).
    pub sound: bool,
    /// Whether the session entry follows.
    pub session: bool,
}

/// Runs the page's forward action: page 1 runs 0 and page 8 runs 1, each
/// entering the session; any other page does nothing. The sound plays only
/// when the input is ready.
pub fn forward_main_menu(page: i32, input_active: bool) -> MenuForward {
    let action = match page {
        1 => Some(0),
        8 => Some(1),
        _ => None,
    };
    MenuForward {
        action,
        sound: action.is_some() && input_active,
        session: action.is_some(),
    }
}

/// The main-menu select's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuSelect {
    pub index: i32,
    pub sound: bool,
    pub stamp: i32,
    pub result: i32,
}

/// Stores the menu index and sounds it at most once per 200 units.
pub fn select_main_menu(index: i32, stamp: i32, now: i32, sound_ready: bool) -> MenuSelect {
    let play = index > 0 && sound_ready && stamp.wrapping_add(0xc8) < now;
    MenuSelect {
        index,
        sound: play,
        stamp: if play { now } else { stamp },
        result: 3,
    }
}

/// The sound cues the main menu asks for.
pub const SELECT_SOUND: &str = "MenuSel";
pub const CLICK_SOUND: &str = "MenuClick";
/// The gain the two cues play at (a guess: the original passes a flags word
/// rather than a linear gain to its audio front end).
pub const MENU_SOUND_GAIN: f32 = 0.2;
/// The cue pitch: a random roll from 0 to 1 becomes 0.8 through 1.2, the
/// original's `roll · 0.4 + 0.8`.
pub fn menu_sound_pitch(roll: f32) -> f32 {
    roll * 0.4 + 0.8
}

/// The title art's texture (`maintitle01.rfi`).
pub const MENU_TITLE_TEXTURE: &str = "maintitle01";
/// The title art's tint.
pub const MENU_TITLE_INK: [u8; 4] = [0xe6, 0xe6, 0xe6, 0x99];

/// The title art's rectangle: the texture scaled by the row scale, centred
/// across, its top `round(row_scale · 10)` down. The original rounds each
/// float conversion to even.
pub fn title_place(screen_width: i32, row_scale: f32, texture: (u32, u32)) -> [i32; 4] {
    let round = |value: f32| crate::gui_font::round_i32(f64::from(value));
    let width = round(texture.0 as f32 * row_scale);
    let height = round(texture.1 as f32 * row_scale);
    let x = round((screen_width as f32 - width as f32) * 0.5);
    let y = round(row_scale * 10.0);
    [x, y, width, height]
}

/// The menu music's two themes and their gains.
pub const MENU_THEME: (&str, f32) = ("MenuTheme", 0.1);
pub const MENU_THEME_DNB: (&str, f32) = ("MenuTheme_DnB", 0.2);

/// The music toggle: when the flag is clear the drum-and-bass theme plays
/// at 0.2 and the flag becomes set; when set the plain theme plays at 0.1
/// and the flag clears.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MusicToggle {
    pub drum_and_bass: bool,
    pub theme: &'static str,
    pub gain: f32,
}

/// Toggles the main menu's music theme (`b -> MenuTheme_DnB` at 0.2, else
/// `MenuTheme` at 0.1).
pub fn toggle_menu_music(flag: bool) -> MusicToggle {
    if flag {
        MusicToggle {
            drum_and_bass: false,
            theme: MENU_THEME.0,
            gain: MENU_THEME.1,
        }
    } else {
        MusicToggle {
            drum_and_bass: true,
            theme: MENU_THEME_DNB.0,
            gain: MENU_THEME_DNB.1,
        }
    }
}

/// The row scale a main menu takes: `round(H/135) × 0.125`.
pub fn menu_row_scale(height: i32) -> f32 {
    (round_even(f64::from(height) / 135.0) as f32) * 0.125
}

/// Round to even through a 64-bit integer, as the original's conversion
/// does.
fn round_even(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    value.round_ties_even() as i64
}

/// What a menu transition opens once the menu has closed. Every one first
/// clears the menu's row marks, hides it, shows the title page and tears
/// the world down; all but the character screen then copy the game mode
/// into the creator's mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuShow {
    /// The character creator (a new practice, versus or story character).
    Creator,
    /// The class select (the story's BEGIN).
    ClassSelect,
    /// The emblem generator (the arena's NEW CAREER).
    Emblem,
    /// The character creator reached from a CONTINUE's character screen.
    CharacterScreen,
}

impl MenuShow {
    /// The session state the show leaves behind: the creator's 2, or the
    /// setup forms' 0x40.
    pub fn state(self) -> u32 {
        match self {
            MenuShow::Creator | MenuShow::CharacterScreen => 2,
            MenuShow::ClassSelect | MenuShow::Emblem => 0x40,
        }
    }

    /// Whether the show copies the game mode into the creator's mode.
    pub fn copies_game_mode(self) -> bool {
        !matches!(self, MenuShow::CharacterScreen)
    }
}

/// The main menu's constructor values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuDefaults {
    /// The title picture is drawn.
    pub title: bool,
    /// A point on no row is kept by the menu rather than let through.
    pub blocks: bool,
    /// The row origin, a fraction of the client height (each page sets its
    /// own when shown).
    pub origin: f32,
    /// A design length of 45 pixels, not read by the menu's own routines.
    pub unread_length: i32,
    /// The row advance in design pixels.
    pub row_advance: i32,
    /// Two of the form's style words (meanings not established).
    pub style: u32,
    pub state: u32,
}

/// The main menu's constructor values.
pub const MENU_DEFAULTS: MenuDefaults = MenuDefaults {
    title: true,
    blocks: true,
    origin: 0.37,
    unread_length: 45,
    row_advance: 70,
    style: 8,
    state: 0x1000,
};

/// The class select's show place: centred across, and down the screen by
/// `round(row_scale × 370) + round(H − row_scale × 1024) / 2`.
pub fn class_select_place(width: i32, height: i32, row_scale: f32) -> (i32, i32) {
    let x = width >> 1;
    let y = round_even(f64::from(row_scale * 370.0))
        + (round_even(f64::from(height as f32 - row_scale * 1024.0)) / 2);
    (x, y as i32)
}

/// The back key's page change: page 1 goes to the title and page 8 to the
/// start menu; any other page stays. The sound plays for both moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuBack {
    pub page: i32,
    pub sound: bool,
}

/// The back key from `page`.
pub fn back_main_menu(page: i32) -> MenuBack {
    match page {
        1 => MenuBack {
            page: 0,
            sound: true,
        },
        8 => MenuBack {
            page: 1,
            sound: true,
        },
        _ => MenuBack { page, sound: false },
    }
}

/// The menu's resume: it closes the menu so the game goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuResume {
    /// The pause count, one lower.
    pub pause: i32,
    /// The match overlay is shown again (a won or round-won session).
    pub show_overlay: bool,
}

/// The resume from the in-game or end pages: the pause count drops by
/// one, the menu's row marks clear and it hides, the overlay comes back
/// when the session state has bit 4 or is 8, the settings and the manual
/// close, and the hub form takes the focus.
pub fn resume_from_menu(pause: i32, state: u32) -> MenuResume {
    MenuResume {
        pause: pause - 1,
        show_overlay: state & 4 != 0 || state == 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_clock_quantises_to_one_135th() {
        assert_eq!(menu_clock_scale(0).to_bits(), 0.0f32.to_bits());
        assert_eq!(menu_clock_scale(135).to_bits(), 0.125f32.to_bits());
        assert_eq!(menu_clock_scale(270).to_bits(), 0.25f32.to_bits());
        assert_eq!(menu_clock_scale(67).to_bits(), 0.0f32.to_bits());
        assert_eq!(menu_clock_scale(68).to_bits(), 0.125f32.to_bits());
        assert_eq!(menu_clock_scale(202).to_bits(), 0.125f32.to_bits());
        assert_eq!(menu_clock_scale(203).to_bits(), 0.25f32.to_bits());
    }

    #[test]
    fn the_menu_close_key_needs_both_bits() {
        let mut base_ran = false;
        let closed = menu_key_close(0x2002, || {
            base_ran = true;
            7
        });
        assert!(closed.closed);
        assert_eq!(closed.result, 1);
        assert!(!base_ran);

        let base = menu_key_close(0x2000, || 7);
        assert!(!base.closed);
        assert_eq!(base.result, 7);

        let low = menu_key_close(0x2, || 7);
        assert!(!low.closed);
        assert_eq!(low.result, 7);

        let wide = menu_key_close(0xffff, || 7);
        assert!(wide.closed);
        assert_eq!(wide.result, 1);
    }

    #[test]
    fn the_class_select_guards_its_sound() {
        let picked = select_menu_class(4, 10);
        assert_eq!(picked.index, 4);
        assert!(picked.played);
        assert_eq!(picked.result, 3);

        assert!(!select_menu_class(0, 10).played);
        assert!(!select_menu_class(-1, 10).played);
        assert!(!select_menu_class(4, 0).played);
        assert_eq!(select_menu_class(0, 0).result, 3);
    }

    #[test]
    fn the_skill_tip_places_its_icon_and_label() {
        let tip = draw_skill_tip(
            SkillTipPlace {
                x: 100,
                y: 50,
                icon: 7,
                text: 9,
            },
            5,
        );
        assert_eq!(tip.icon_at, [10, 25]);
        assert_eq!(tip.label, [80, 50, 30, 10]);
        assert_eq!((tip.icon, tip.text), (7, 9));
        assert_eq!(
            (
                tip.begin_calls,
                tip.bind_calls,
                tip.setup_calls,
                tip.draw_calls
            ),
            (1, 1, 2, 2)
        );
    }

    #[test]
    fn the_transition_fade_centres_its_text() {
        let place = TransitionFadePlace {
            page: 2,
            x: 200,
            y: 50,
            colour: [10, 20, 30],
            alpha: 255,
            glyph_width: 8,
            glyph_height: 16,
        };
        let fade = draw_transition_fade(place);
        assert!(fade.active);
        assert_eq!(fade.text, "TO BE CONTINUED");
        assert_eq!(fade.origin, [40, 17]);
        assert_eq!(fade.colour, [10, 20, 30, 255]);
        assert_eq!(fade.blend_mode, 0x203);
        assert_eq!(
            (
                fade.begin_calls,
                fade.draw_calls,
                fade.font_calls,
                fade.base_calls
            ),
            (1, 1, 1, 1)
        );

        let other = draw_transition_fade(TransitionFadePlace { page: 1, ..place });
        assert!(!other.active);
        assert_eq!(other.begin_calls, 0);
        assert_eq!(other.draw_calls, 0);
        assert_eq!(other.base_calls, 1);
    }

    #[test]
    fn the_skill_selection_clamps_and_flags() {
        let picked = settle_skill_selection(4, 1, 0);
        assert_eq!(picked.index, 4);
        assert_eq!(picked.flags, 0x110);

        let cleared = settle_skill_selection(0, 1, 0xffff);
        assert_eq!(cleared.index, 0);
        assert_eq!(cleared.flags, 0xffff & !0x110);

        assert_eq!(settle_skill_selection(9, 1, 0xffff).index, 0);
        assert_eq!(settle_skill_selection(-1, 1, 0xffff).index, 0);
        assert_eq!(settle_skill_selection(8, 1, 0).index, 8);

        let other = settle_skill_selection(4, 0, 0x20);
        assert_eq!(other.index, 4);
        assert_eq!(other.flags, 0x20);
    }

    #[test]
    fn the_inventory_close_prefers_its_callback() {
        let with = inventory_key_close(0x2002, true, || 7);
        assert!(with.closed);
        assert!(with.callback);
        assert_eq!(with.result, 1);

        let without = inventory_key_close(0x2002, false, || 7);
        assert!(without.closed);
        assert!(!without.callback);
        assert_eq!(without.result, 1);

        let mut ran = false;
        let base = inventory_key_close(0x2000, true, || {
            ran = true;
            7
        });
        assert!(!base.closed);
        assert!(!base.callback);
        assert_eq!(base.result, 7);
        assert!(ran);
    }

    #[test]
    fn the_skill_selector_routes_its_key() {
        assert_eq!(skill_selector_key(3, 2, 4, 0, || 7).result, 7);
        assert!(!skill_selector_key(3, 2, 4, 0, || 7).closed);
        assert_eq!(skill_selector_key(0, 1, 9, 0, || 7).result, 7);

        let closed = skill_selector_key(0, 1, 4, 0, || 7);
        assert!(closed.closed);
        assert_eq!(closed.result, 0x40);

        let word_nine_mode_one = skill_selector_key(0, 1, 9, 1, || 7);
        assert!(word_nine_mode_one.closed);
        assert_eq!(word_nine_mode_one.result, 0x40);

        let page_one = skill_selector_key(3, 1, 4, 0, || 7);
        assert!(page_one.closed);
    }

    #[test]
    fn the_end_session_form_keeps_its_constructor_values() {
        let f = end_session_form();
        assert_eq!(f.scale, 0.9);
        assert!(!f.first_flag && !f.second_flag);
        assert_eq!(f.base_word, 1);
    }

    #[test]
    fn the_end_session_dispatches_by_mode_then_item() {
        let input = |mode, item, menu, fight| EndSessionInput {
            mode,
            item,
            menu,
            fights: 8,
            fight,
        };
        let run = |i| dispatch_end_session(i, &[3, 16, 3]).action;
        // Practice: the victory menu restarts, any other menu fights on.
        assert_eq!(
            run(input(2, 1, 2, 0)),
            Some(EndSessionAction::PracticeRestart)
        );
        assert_eq!(
            run(input(2, 1, 3, 6)),
            Some(EndSessionAction::NextPracticeFight { last: false })
        );
        assert_eq!(
            run(input(2, 1, 3, 7)),
            Some(EndSessionAction::NextPracticeFight { last: true })
        );
        assert_eq!(run(input(2, 2, 3, 0)), Some(EndSessionAction::BackToMenu));
        // Story: only the reconstructed menu respawns.
        assert_eq!(run(input(4, 1, 4, 0)), Some(EndSessionAction::StoryRespawn));
        assert_eq!(run(input(4, 1, 2, 0)), Some(EndSessionAction::StoryQuit));
        assert_eq!(run(input(4, 2, 4, 0)), Some(EndSessionAction::StoryQuit));
        // Arena.
        assert_eq!(
            run(input(8, 1, 5, 0)),
            Some(EndSessionAction::ArenaContinue)
        );
        assert_eq!(run(input(8, 2, 5, 0)), Some(EndSessionAction::BackToMenu));
        // Items past two, and none, do nothing in the three modes.
        assert_eq!(run(input(8, 3, 5, 0)), None);
        assert_eq!(run(input(2, 0, 3, 0)), None);
        // Other modes go to the first matching handler, with the item.
        assert_eq!(
            run(input(3, 0, 0, 0)),
            Some(EndSessionAction::Registered {
                handler: 0,
                item: 0
            })
        );
        assert_eq!(
            run(input(16, 2, 0, 0)),
            Some(EndSessionAction::Registered {
                handler: 1,
                item: 2
            })
        );
        assert_eq!(run(input(1, 1, 0, 0)), None);
        let d = dispatch_end_session(input(4, 1, 4, 0), &[]);
        assert!(d.clears_item && d.clears_pressed_item);
        assert_eq!(d.result, 0);
    }

    #[test]
    fn the_menu_hover_rate_limits_its_sound() {
        let same = update_menu_hover(5, 5, 5, 0, 300);
        assert_eq!(same.selection, 5);
        assert!(!same.sound);
        assert_eq!(same.result, 0);
        assert_eq!(same.stamp, 0);

        let changed = update_menu_hover(5, 5, 0, 0, 300);
        assert_eq!(changed.selection, 5);
        assert!(changed.sound);
        assert_eq!(changed.stamp, 300);
        assert_eq!(changed.result, 3);

        let zero = update_menu_hover(3, 5, 4, 0, 300);
        assert_eq!(zero.selection, 0);
        assert!(!zero.sound);
        assert_eq!(zero.stamp, 300);
        assert_eq!(zero.result, 3);

        let limited = update_menu_hover(5, 5, 0, 200, 300);
        assert!(!limited.sound);
        assert_eq!(limited.stamp, 200);
        assert_eq!(limited.result, 3);
    }

    #[test]
    fn the_main_menu_key_routes_its_masks() {
        let input = |mask, selection, limit| MainMenuKeyInput {
            mask,
            selection,
            limit,
            confirm_arg: 0xabc,
            sub_result: 7,
        };

        let confirm = main_menu_key(input(0x100, 5, 10));
        assert_eq!(
            confirm.action,
            MainMenuKeyAction::Confirm {
                index: 5,
                arg: 0xabc
            }
        );
        assert!(confirm.sound);
        assert_eq!(confirm.result, 7);

        let zero = main_menu_key(input(0x100, 0, 10));
        assert_eq!(zero.action, MainMenuKeyAction::None);
        assert!(!zero.sound);
        assert_eq!(zero.result, 0);

        assert_eq!(
            main_menu_key(input(0x200, 5, 10)).action,
            MainMenuKeyAction::Forward
        );
        assert_eq!(
            main_menu_key(input(0x400, 5, 10)).action,
            MainMenuKeyAction::Back
        );

        assert_eq!(
            main_menu_key(input(0x8001, 2, 4)).action,
            MainMenuKeyAction::Move(3)
        );
        assert_eq!(
            main_menu_key(input(0x8001, 4, 4)).action,
            MainMenuKeyAction::None
        );
        assert_eq!(
            main_menu_key(input(0x8001, 0, 4)).action,
            MainMenuKeyAction::Move(1)
        );
        assert_eq!(
            main_menu_key(input(0x4001, 3, 10)).action,
            MainMenuKeyAction::Move(2)
        );
        assert_eq!(
            main_menu_key(input(0x4001, 1, 10)).action,
            MainMenuKeyAction::None
        );
    }

    #[test]
    fn the_inventory_detach_releases_its_stack() {
        let input = |already_inactive, has_owner, has_child, child_visible, has_stack, count| {
            InventoryDetachInput {
                already_inactive,
                has_owner,
                has_child,
                child_visible,
                has_stack,
                stack_count: count,
                loot_pane: false,
                base_result: true,
            }
        };

        let inactive = detach_inventory(input(true, true, true, true, true, 2));
        assert!(inactive.inactive);
        assert!(!inactive.marked);
        assert_eq!(inactive.base_calls, 0);
        assert!(inactive.result);

        let active = detach_inventory(input(false, true, true, true, true, 2));
        assert!(active.marked);
        assert!(active.placed);
        assert_eq!(active.stack_count, 1);
        assert!(!active.freed);
        assert_eq!(active.base_calls, 1);
        assert!(active.result);

        let last = detach_inventory(input(false, true, false, false, true, 1));
        assert!(!last.placed);
        assert_eq!(last.stack_count, 0);
        assert!(last.freed);

        let none = detach_inventory(input(false, false, false, false, false, 3));
        assert!(!none.marked);
        assert_eq!(none.stack_count, 3);

        let loot = detach_inventory(InventoryDetachInput {
            loot_pane: true,
            ..input(false, true, true, true, false, 0)
        });
        assert!(loot.globals_copied);
    }

    #[test]
    fn the_inventory_page_stores_its_sentinel() {
        assert_eq!(
            inventory_page(-1, 5, 9),
            InventoryPage {
                field: -3,
                result: 9
            }
        );
        assert_eq!(
            inventory_page(4, 5, 9),
            InventoryPage {
                field: 5,
                result: 9
            }
        );
    }

    #[test]
    fn the_menu_name_shifts_and_lowers() {
        let buffer = *b"?barechoon";
        let typed = type_menu_name(buffer, b'S', 2);
        assert!(!typed.base);
        assert_eq!(&typed.buffer, b"barechoons");
        assert!(typed.matched);

        let lowered = type_menu_name([0; 10], b'A', 2);
        assert_eq!(lowered.buffer[9], b'a');
        assert!(!lowered.matched);

        let kept = type_menu_name([0; 10], b'0', 2);
        assert_eq!(kept.buffer[9], b'0');

        let base = type_menu_name(buffer, b'X', 0);
        assert!(base.base);
        assert_eq!(base.buffer, buffer);
        assert!(!base.matched);
    }

    #[test]
    fn the_splash_bounds_are_negative() {
        let bounds = splash_bounds(7);
        assert_eq!(bounds.screen, 7);
        assert_eq!(bounds.second, -1);
        assert_eq!(bounds.result, -1);
    }

    #[test]
    fn the_splash_destroy_frees_in_order() {
        let freeing = destroy_splash(SplashDestroyInput {
            free_flag: true,
            first: true,
            second: true,
            count: 2,
        });
        assert_eq!(freeing.release_calls, 1);
        assert!(freeing.global_cleared);
        assert!(freeing.first_freed);
        assert!(freeing.second_freed);
        assert_eq!(freeing.list_calls, 1);
        assert_eq!(freeing.base_calls, 1);
        assert_eq!(freeing.final_calls, 1);

        let quiet = destroy_splash(SplashDestroyInput {
            free_flag: false,
            first: false,
            second: false,
            count: 0,
        });
        assert_eq!(quiet.release_calls, 0);
        assert_eq!(quiet.list_calls, 0);
        assert_eq!(quiet.base_calls, 1);
        assert_eq!(quiet.final_calls, 0);
    }

    #[test]
    fn the_skill_selector_draws_its_two_modes() {
        let place = |raw, page| SkillSelectorDrawPlace {
            raw,
            page,
            x0: 10,
            x1: 110,
            y1: 180,
            icon_width: 20,
        };

        let title = draw_skill_selector(place(4, 2), 5);
        assert_eq!(title.mode, SkillSelectorMode::Title);
        assert_eq!(title.title, Some([60, 90, 0, 100, 1]));
        assert_eq!((title.font_calls, title.draw_calls), (1, 1));

        let skill = draw_skill_selector(place(4, 1), 5);
        assert_eq!(skill.mode, SkillSelectorMode::Skill);
        assert_eq!(skill.icon, Some([40, 80, 20]));
        assert_eq!(skill.label, Some([90, 90, 0, 80, 1]));
        assert_eq!((skill.font_calls, skill.draw_calls), (2, 2));

        let empty = draw_skill_selector(place(9, 1), 5);
        assert_eq!(empty.mode, SkillSelectorMode::Empty);
        assert_eq!((empty.font_calls, empty.draw_calls), (0, 0));
        assert_eq!(
            draw_skill_selector(place(0, 1), 5).mode,
            SkillSelectorMode::Empty
        );
    }

    #[test]
    fn the_skill_tip_show_places_and_stamps() {
        let input = SkillTipShowInput {
            found: true,
            anchor: [7, 8],
            skill_index: 5,
            technique_index: 4,
            unit: 5,
            gui_level: 0,
            fade_end: 20,
            stamp_base: 10,
        };
        let shown = show_skill_tip(input);
        assert!(shown.shown);
        assert_eq!(shown.index, Some(4));
        assert_eq!(shown.threshold, Some(3));
        assert_eq!(shown.flags_or, 0x10);
        assert_eq!(shown.place, [7, 53]);
        assert_eq!(shown.height, 120);
        assert!(shown.visible);
        assert_eq!(shown.calls, 3);
        assert_eq!(shown.stamp, 50);

        let short = show_skill_tip(SkillTipShowInput {
            gui_level: 1,
            ..input
        });
        assert_eq!(short.height, 90);

        let missing = show_skill_tip(SkillTipShowInput {
            found: false,
            ..input
        });
        assert!(!missing.shown);
        assert_eq!(missing.calls, 0);
        assert_eq!(missing.index, None);
    }

    #[test]
    fn the_loot_show_marks_and_places() {
        let input = |already_inactive| CharLootShowInput {
            already_inactive,
            has_owner: true,
            has_child: true,
            child_visible: true,
            base_result: true,
        };
        let active = show_char_loot(input(false));
        assert!(!active.inactive);
        assert!(active.marked);
        assert!(active.placed);
        assert_eq!(active.base_calls, 1);
        assert!(active.result);

        let inactive = show_char_loot(input(true));
        assert!(inactive.inactive);
        assert!(!inactive.marked);
        assert_eq!(inactive.base_calls, 0);

        let hidden = show_char_loot(CharLootShowInput {
            child_visible: false,
            ..input(false)
        });
        assert!(!hidden.placed);
    }

    #[test]
    fn the_journal_show_copies_and_assigns() {
        let shown = show_journal(12, 34, 3);
        assert_eq!(shown.origin, [12, 34]);
        assert_eq!(shown.table_offset, 3 * 0x18 + 0x10);
        assert!(shown.assigned);
        assert_eq!(shown.base_calls, 1);
    }

    #[test]
    fn the_character_view_clears_and_destroys() {
        let freeing = destroy_character_view(true);
        assert_eq!(
            (
                freeing.release_calls,
                freeing.base_calls,
                freeing.final_calls
            ),
            (1, 1, 1)
        );
        let quiet = destroy_character_view(false);
        assert_eq!(
            (quiet.release_calls, quiet.base_calls, quiet.final_calls),
            (0, 1, 0)
        );

        let clear = clear_character_view(true, true);
        assert!(clear.had_view);
        assert_eq!(clear.release_calls, 1);
        assert!(clear.slot_cleared);
        assert_eq!(clear.base_calls, 1);
        assert!(clear.result);

        let empty = clear_character_view(false, false);
        assert!(!empty.had_view);
        assert_eq!(empty.release_calls, 0);
        assert_eq!(empty.base_calls, 1);
        assert!(!empty.result);
    }

    #[test]
    fn the_skill_selector_show_applies_its_word() {
        let picked = show_skill_selector(true, 3, 4);
        assert_eq!(picked.applied, SkillSelectorApplied::Selected(3));
        assert!(picked.flags_cleared);
        assert_eq!(picked.base_calls, 1);

        assert_eq!(
            show_skill_selector(true, 0, 9).applied,
            SkillSelectorApplied::Selected(-1)
        );
        assert_eq!(
            show_skill_selector(true, 0, 10).applied,
            SkillSelectorApplied::Selected(-1)
        );
        assert_eq!(
            show_skill_selector(true, 0, 4).applied,
            SkillSelectorApplied::None
        );

        let absent = show_skill_selector(false, 3, 4);
        assert_eq!(absent.applied, SkillSelectorApplied::None);
        assert!(!absent.flags_cleared);
        assert_eq!(absent.base_calls, 1);
    }

    #[test]
    fn the_class_confirm_stores_and_opens() {
        let picked = confirm_class(3, true);
        assert_eq!(picked.selected, Some(3));
        assert_eq!(picked.global_index, Some(2));
        assert!(picked.sound);
        assert!(picked.page_copied);
        assert_eq!(picked.form, Some(MenuStep::ClassForm));
        assert!(!picked.form_flag);
        assert_eq!(picked.result, 7);

        let quiet = confirm_class(3, false);
        assert!(!quiet.sound);

        let none = confirm_class(0, true);
        assert_eq!(none.result, 0x40);
        assert_eq!(none.selected, None);
        assert_eq!(none.form, None);
        assert!(!confirm_class(-1, true).page_copied);
    }

    #[test]
    fn the_class_navigation_scans_available_bits() {
        let input = |mask, selection| ClassNavInput {
            mask,
            selection,
            available: 0b1010,
            sound_ready: true,
        };

        let confirm = navigate_class(input(0x100, 3));
        assert_eq!(confirm.action, ClassNavAction::Confirm);
        assert_eq!(confirm.result, 7);
        assert_eq!(confirm.form, Some(MenuStep::ClassForm));
        assert!(confirm.sound);
        assert_eq!(navigate_class(input(0x100, 0)).result, 0);

        assert_eq!(
            navigate_class(input(0x200, 1)).action,
            ClassNavAction::Forward
        );
        assert_eq!(
            navigate_class(input(0x400, 1)).action,
            ClassNavAction::Forward
        );

        assert_eq!(
            navigate_class(input(0x8001, 1)).action,
            ClassNavAction::Move(3)
        );
        assert_eq!(
            navigate_class(input(0x4001, 4)).action,
            ClassNavAction::Move(3)
        );
        assert_eq!(
            navigate_class(input(0x4001, 2)).action,
            ClassNavAction::Move(1)
        );
        assert_eq!(
            navigate_class(input(0x8001, 4)).action,
            ClassNavAction::None
        );
        assert_eq!(
            navigate_class(input(0x8001, 0)).action,
            ClassNavAction::Move(1)
        );
    }

    #[test]
    fn the_class_cards_are_hit_inside_their_bands() {
        let input = |hit| ClassCardHitInput {
            hit,
            available: 1,
            screen_w: 1000,
            clock: 2048,
            scale: 1.0,
        };
        let first = hit_class_cards(input([300, 700]));
        assert_eq!(first.index, Some(1));
        assert_eq!(first.count, 2);
        assert_eq!(first.top, 652);
        assert_eq!(first.height, 512);

        assert_eq!(hit_class_cards(input([501, 700])).index, Some(2));
        assert_eq!(hit_class_cards(input([468, 700])).index, None);
        assert_eq!(hit_class_cards(input([212, 700])).index, None);
        assert_eq!(hit_class_cards(input([300, 652])).index, None);
        assert_eq!(hit_class_cards(input([300, 1164])).index, None);
        assert_eq!(hit_class_cards(input([300, 653])).index, Some(1));

        let absent = hit_class_cards(ClassCardHitInput {
            available: 0,
            ..input([400, 700])
        });
        assert_eq!(absent.index, Some(1));
        assert_eq!(absent.count, 1);
    }

    #[test]
    fn the_main_menu_confirm_and_select() {
        let confirmed = confirm_main_menu(3, 0x1000, true);
        assert_eq!(confirmed.selected, Some(3));
        assert!(confirmed.copy_to_204);
        assert!(confirmed.sound);
        assert_eq!(confirmed.result, 7);

        let other = confirm_main_menu(3, 0x2000, true);
        assert!(!other.copy_to_204);
        assert_eq!(other.result, 0x40);

        let none = confirm_main_menu(0, 0x1000, true);
        assert!(none.copy_to_204);
        assert_eq!(none.selected, None);
        assert_eq!(none.result, 0x40);

        let picked = select_main_menu(3, 0, 300, true);
        assert!(picked.sound);
        assert_eq!(picked.stamp, 300);
        assert_eq!(picked.result, 3);

        let zero = select_main_menu(0, 0, 300, true);
        assert!(!zero.sound);
        assert_eq!(zero.stamp, 0);

        let limited = select_main_menu(3, 200, 300, true);
        assert!(!limited.sound);
        assert_eq!(limited.stamp, 200);

        assert!(!select_main_menu(3, 0, 300, false).sound);
    }

    #[test]
    fn the_main_menu_forward_runs_the_page_action() {
        let resume = forward_main_menu(1, true);
        assert_eq!(resume.action, Some(0));
        assert!(resume.sound);
        assert!(resume.session);

        let other = forward_main_menu(8, false);
        assert_eq!(other.action, Some(1));
        assert!(!other.sound);
        assert!(other.session);

        let none = forward_main_menu(2, true);
        assert_eq!(none.action, None);
        assert!(!none.sound);
        assert!(!none.session);
    }

    #[test]
    fn the_character_view_stops_its_chain_walk() {
        let settled = [
            ViewNode {
                next: Some(1),
                record: Some(ViewNodeRecord { flags: 0, value: 0 }),
            },
            ViewNode {
                next: None,
                record: Some(ViewNodeRecord {
                    flags: 0x12000000,
                    value: -1,
                }),
            },
        ];
        assert_eq!(first_settled_view_node(&settled, 0), Some(1));

        let missing = [
            ViewNode {
                next: Some(1),
                record: Some(ViewNodeRecord { flags: 0, value: 0 }),
            },
            ViewNode {
                next: Some(2),
                record: Some(ViewNodeRecord {
                    flags: 0x12000000,
                    value: 0,
                }),
            },
            ViewNode {
                next: None,
                record: None,
            },
        ];
        assert_eq!(first_settled_view_node(&missing, 0), Some(2));

        let open = [ViewNode {
            next: None,
            record: Some(ViewNodeRecord {
                flags: 0x02000000,
                value: -1,
            }),
        }];
        assert_eq!(first_settled_view_node(&open, 0), None);
    }

    #[test]
    fn the_settings_menu_opens_closes_and_resets() {
        let created = open_settings_menu(false, false);
        assert!(created.created);
        assert!(!created.shown);
        assert!(created.laid_out);
        assert_eq!(created.selected, None);
        assert_eq!(created.finished, -1);

        let shown = open_settings_menu(true, true);
        assert!(!shown.created);
        assert!(shown.shown);
        assert_eq!(shown.selected, Some(1));

        assert!(close_settings_menu(true));
        assert!(!close_settings_menu(false));
        assert!(reset_settings_menu(true));
        assert!(!reset_settings_menu(false));
    }

    #[test]
    fn the_menu_music_swaps_themes() {
        let first = toggle_menu_music(false);
        assert_eq!(
            (first.drum_and_bass, first.theme, first.gain),
            (true, "MenuTheme_DnB", 0.2)
        );
        let second = toggle_menu_music(true);
        assert_eq!(
            (second.drum_and_bass, second.theme, second.gain),
            (false, "MenuTheme", 0.1)
        );
    }

    #[test]
    fn the_row_scale_quantises_by_135ths() {
        assert_eq!(menu_row_scale(1080), 1.0);
        assert_eq!(menu_row_scale(900), 0.875);
        assert_eq!(menu_row_scale(453), 0.375);
        assert_eq!(menu_row_scale(135), 0.125);
        assert_eq!(menu_row_scale(67), 0.0);
    }

    #[test]
    fn the_class_select_sits_where_the_original_puts_it() {
        assert_eq!(class_select_place(1600, 1014, 1.0), (800, 365));
        assert_eq!(class_select_place(1600, 1080, 1.0), (800, 398));
        assert_eq!(class_select_place(800, 600, 0.5), (400, 229));
    }

    #[test]
    fn the_back_key_swaps_two_pages() {
        assert_eq!(
            back_main_menu(1),
            MenuBack {
                page: 0,
                sound: true
            }
        );
        assert_eq!(
            back_main_menu(8),
            MenuBack {
                page: 1,
                sound: true
            }
        );
        assert_eq!(
            back_main_menu(2),
            MenuBack {
                page: 2,
                sound: false
            }
        );
    }

    #[test]
    fn the_resume_drops_the_pause_and_shows_the_overlay() {
        assert_eq!(
            resume_from_menu(1, 3),
            MenuResume {
                pause: 0,
                show_overlay: false
            }
        );
        assert!(resume_from_menu(1, 4).show_overlay);
        assert!(resume_from_menu(1, 5).show_overlay);
        assert!(resume_from_menu(1, 8).show_overlay);
        assert!(!resume_from_menu(1, 0x10).show_overlay);
    }

    #[test]
    fn the_constructor_defaults() {
        assert_eq!(
            (
                MENU_DEFAULTS.title,
                MENU_DEFAULTS.blocks,
                MENU_DEFAULTS.unread_length,
                MENU_DEFAULTS.row_advance,
                MENU_DEFAULTS.style,
                MENU_DEFAULTS.state
            ),
            (true, true, 45, 70, 8, 0x1000)
        );
        assert_eq!(MENU_DEFAULTS.origin.to_bits(), 0x3ebd_70a4);
        assert_eq!(MenuShow::Creator.state(), 2);
        assert_eq!(MenuShow::ClassSelect.state(), 0x40);
        assert_eq!(MenuShow::Emblem.state(), 0x40);
        assert!(MenuShow::Emblem.copies_game_mode());
        assert!(!MenuShow::CharacterScreen.copies_game_mode());
    }

    #[test]
    fn the_title_art_is_scaled_and_centred() {
        // 1024 by 256 at row scale 1: centred at (288, 10).
        let place = title_place(1600, 1.0, (1024, 256));
        assert_eq!(place, [288, 10, 1024, 256]);
        // Half scale: 512 by 128, x 544, y 5.
        assert_eq!(title_place(1600, 0.5, (1024, 256)), [544, 5, 512, 128]);
        // A texture odd at the scale rounds to even: 101 × 0.25 = 25.25
        // (25), 102 × 0.25 = 25.5 (26); the x 487.5 rounds to the even 488
        // and the y 2.5 to the even 2.
        assert_eq!(title_place(1000, 0.25, (101, 102)), [488, 2, 25, 26]);
        assert_eq!(MENU_TITLE_INK, [0xe6, 0xe6, 0xe6, 0x99]);
        assert_eq!(MENU_TITLE_TEXTURE, "maintitle01");
    }

    #[test]
    fn the_menu_cues_pitch_from_half_to_one_and_a_fifth() {
        assert!((menu_sound_pitch(0.0) - 0.8).abs() < 1e-7);
        assert!((menu_sound_pitch(0.5) - 1.0).abs() < 1e-7);
        assert!((menu_sound_pitch(1.0) - 1.2).abs() < 1e-7);
        assert_eq!((SELECT_SOUND, CLICK_SOUND), ("MenuSel", "MenuClick"));
    }
}
