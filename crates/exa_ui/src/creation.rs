//! The character creation screen's labels and page captions, read from the
//! original's forms. The rules live in `exa_game::create`; this module only
//! names what the screen shows, so the game code and the UI cannot drift.
//!
//! Evidence: REA on the character screen's unit routines (the page captions,
//! the final button's four labels and the two continue labels).

/// The appearance page's first tab.
pub const NAME: &str = "NAME";
/// The appearance page's second tab.
pub const HAIR: &str = "HAIR";
/// The appearance page's final button.
pub const NEXT: &str = "NEXT";
/// The equipment page's apparel caption.
pub const APPAREL: &str = "APPAREL";
/// The equipment page's weapon caption.
pub const WEAPONS: &str = "WEAPONS";
/// The versus script's equipment caption.
pub const EQUIPMENT: &str = "EQUIPMENT";
/// The versus script's lobby caption.
pub const LOBBY: &str = "LOBBY";
/// The arena recruitment button.
pub const RECRUIT: &str = "RECRUIT";
/// The story start button.
pub const BEGIN: &str = "BEGIN";
/// The versus start button.
pub const READY: &str = "READY";
/// The free creator's button.
pub const CREATE: &str = "CREATE";
/// The equipment page's final button.
pub const CONTINUE: &str = "CONTINUE";
/// The versus continue list's title.
pub const VERSUS: &str = "VERSUS";
/// The skills caption.
pub const SKILLS: &str = "SKILLS";
/// The arena skills page's progress caption.
pub const CURRENT_SKILL_PROGRESS: &str = "Current skill progress";
/// The new global experience caption.
pub const NEW_GLOBAL_EXPERIENCE: &str = "New global experience";
/// The global experience caption.
pub const GLOBAL_EXPERIENCE: &str = "Global experience";
/// The continue screen's save label.
pub const CONTINUE_FROM_SAVE: &str = "Continue from save";
/// The continue screen's checkpoint label.
pub const CONTINUE_FROM_CHECKPOINT: &str = "Continue from checkpoint";
/// The name warning's title.
pub const CHARACTER_NAME: &str = "Character Name";
/// The name warning's text.
pub const PLEASE_NAME: &str = "Please name your character";
/// The error shown when continue mode has no save manager.
pub const SAVE_MANAGER_MISSING: &str = "Save manager not found.";
/// The character screen's title.
pub const CHARACTER_SCREEN: &str = "Character Screen";

/// A creator page's captions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageLabels {
    /// The first tab's caption.
    pub first: &'static str,
    /// The second tab's caption.
    pub second: &'static str,
    /// The final button's caption.
    pub final_button: &'static str,
}

/// The appearance page's captions.
pub fn appearance_labels() -> PageLabels {
    PageLabels {
        first: NAME,
        second: HAIR,
        final_button: NEXT,
    }
}

/// The equipment page's captions. A versus lobby with its script shows the
/// equipment and lobby tabs; every other mode shows apparel and weapons.
pub fn equipment_labels(game_mode: i32, screen_mode: i32, versus_script: bool) -> PageLabels {
    let versus = game_mode == 0x1010 && versus_script;
    PageLabels {
        first: if versus { EQUIPMENT } else { APPAREL },
        second: if versus { LOBBY } else { WEAPONS },
        final_button: commit_label(screen_mode, game_mode),
    }
}

/// The final button's label on the appearance page.
pub fn commit_label(screen_mode: i32, game_mode: i32) -> &'static str {
    if screen_mode == 10 {
        RECRUIT
    } else if game_mode == 4 {
        BEGIN
    } else if game_mode == 0x1010 {
        READY
    } else {
        CREATE
    }
}

/// The continue button's label for a save kind (2 is a save).
pub fn continue_label(kind: i32) -> &'static str {
    if kind == 2 {
        CONTINUE_FROM_SAVE
    } else {
        CONTINUE_FROM_CHECKPOINT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_appearance_page_names_its_tabs_and_button() {
        assert_eq!(
            appearance_labels(),
            PageLabels {
                first: "NAME",
                second: "HAIR",
                final_button: "NEXT"
            }
        );
    }

    #[test]
    fn the_equipment_page_follows_the_versus_script() {
        assert_eq!(
            equipment_labels(8, 10, false),
            PageLabels {
                first: "APPAREL",
                second: "WEAPONS",
                final_button: "RECRUIT"
            }
        );
        assert_eq!(
            equipment_labels(0x1010, 0, true),
            PageLabels {
                first: "EQUIPMENT",
                second: "LOBBY",
                final_button: "READY"
            }
        );
        assert_eq!(equipment_labels(0x1010, 0, false).first, "APPAREL");
    }

    #[test]
    fn the_final_button_is_named_for_its_mode() {
        assert_eq!(commit_label(10, 8), "RECRUIT");
        assert_eq!(commit_label(4, 4), "BEGIN");
        assert_eq!(commit_label(0, 0x1010), "READY");
        assert_eq!(commit_label(0, 0), "CREATE");
    }

    #[test]
    fn the_continue_button_names_each_save_kind() {
        assert_eq!(continue_label(2), "Continue from save");
        assert_eq!(continue_label(1), "Continue from checkpoint");
        assert_eq!(continue_label(0), "Continue from checkpoint");
    }
}
