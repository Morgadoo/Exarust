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

const END_RESUME: u64 = 0x1_0018_bf80;
const END_QUIT_FORM: u64 = 0x1_0012_43c0;
const END_PAGE4_SPECIAL: u64 = 0x1_0012_4650;
const END_PAGE4_NORMAL: u64 = 0x1_0012_4820;
const END_PAGE8_FINAL: u64 = 0x1_0012_3d30;
const END_PAGE8_FORM: u64 = 0x1_0012_4800;
const END_PAGE8_A: u64 = 0x1_0013_2250;
const END_PAGE8_B: u64 = 0x1_0017_b480;
const END_PAGE8_C: u64 = 0x1_0012_c2c0;

/// One dispatched end-session action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndSessionAction {
    Call(u64),
    Show { form: u64, flag: bool },
    Registered(u64),
}

/// The end-session dispatch's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndSessionInput {
    pub page: i32,
    pub action: i32,
    pub form_page: i32,
    pub final_round: bool,
    pub handler: Option<u64>,
}

/// The dispatched actions and the cleared fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndSessionDispatch {
    pub actions: Vec<EndSessionAction>,
    pub clears_action: bool,
    pub clears_field_204: bool,
    pub result: i32,
}

/// Resolves the end-session page and action into calls or forms.
pub fn dispatch_end_session(input: EndSessionInput) -> EndSessionDispatch {
    let mut actions = Vec::new();
    match input.page {
        2 => match input.action {
            1 => actions.push(EndSessionAction::Call(END_RESUME)),
            2 => actions.push(EndSessionAction::Show {
                form: END_QUIT_FORM,
                flag: true,
            }),
            _ => {}
        },
        4 => match input.action {
            1 => {
                actions.push(EndSessionAction::Call(if input.form_page == 4 {
                    END_PAGE4_SPECIAL
                } else {
                    END_PAGE4_NORMAL
                }));
            }
            2 => actions.push(EndSessionAction::Call(END_PAGE4_NORMAL)),
            _ => {}
        },
        8 => match input.action {
            1 => {
                if input.form_page == 2 {
                    actions.push(EndSessionAction::Call(END_PAGE8_A));
                    actions.push(EndSessionAction::Call(END_PAGE8_B));
                    actions.push(EndSessionAction::Call(END_PAGE8_C));
                    actions.push(EndSessionAction::Show {
                        form: END_PAGE8_FORM,
                        flag: true,
                    });
                } else {
                    actions.push(EndSessionAction::Show {
                        form: END_PAGE8_FINAL,
                        flag: input.final_round,
                    });
                }
            }
            2 => actions.push(EndSessionAction::Show {
                form: END_QUIT_FORM,
                flag: true,
            }),
            _ => {}
        },
        _ => {
            if let Some(handler) = input.handler {
                actions.push(EndSessionAction::Registered(handler));
            }
        }
    }
    EndSessionDispatch {
        actions,
        clears_action: true,
        clears_field_204: true,
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
pub fn update_menu_hover(
    hover: i32,
    field_204: i32,
    previous: i32,
    stamp: i32,
    now: i32,
) -> MenuHover {
    let candidate = if hover == field_204 { field_204 } else { 0 };
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
    fn the_end_session_dispatches_its_pages() {
        let input = |page, action, form_page, final_round, handler| EndSessionInput {
            page,
            action,
            form_page,
            final_round,
            handler,
        };

        let resume = dispatch_end_session(input(2, 1, 0, false, None));
        assert_eq!(resume.actions, vec![EndSessionAction::Call(END_RESUME)]);
        let quit = dispatch_end_session(input(2, 2, 0, false, None));
        assert_eq!(
            quit.actions,
            vec![EndSessionAction::Show {
                form: END_QUIT_FORM,
                flag: true
            }]
        );

        assert_eq!(
            dispatch_end_session(input(4, 1, 4, false, None)).actions,
            vec![EndSessionAction::Call(END_PAGE4_SPECIAL)]
        );
        assert_eq!(
            dispatch_end_session(input(4, 1, 3, false, None)).actions,
            vec![EndSessionAction::Call(END_PAGE4_NORMAL)]
        );
        assert_eq!(
            dispatch_end_session(input(4, 2, 0, false, None)).actions,
            vec![EndSessionAction::Call(END_PAGE4_NORMAL)]
        );

        assert_eq!(
            dispatch_end_session(input(8, 1, 2, false, None)).actions,
            vec![
                EndSessionAction::Call(END_PAGE8_A),
                EndSessionAction::Call(END_PAGE8_B),
                EndSessionAction::Call(END_PAGE8_C),
                EndSessionAction::Show {
                    form: END_PAGE8_FORM,
                    flag: true
                }
            ]
        );
        assert_eq!(
            dispatch_end_session(input(8, 1, 3, true, None)).actions,
            vec![EndSessionAction::Show {
                form: END_PAGE8_FINAL,
                flag: true
            }]
        );
        assert_eq!(
            dispatch_end_session(input(8, 1, 3, false, None)).actions,
            vec![EndSessionAction::Show {
                form: END_PAGE8_FINAL,
                flag: false
            }]
        );

        assert_eq!(
            dispatch_end_session(input(3, 1, 0, false, Some(0xabc))).actions,
            vec![EndSessionAction::Registered(0xabc)]
        );
        assert!(
            dispatch_end_session(input(3, 1, 0, false, None))
                .actions
                .is_empty()
        );

        let done = dispatch_end_session(input(2, 1, 0, false, None));
        assert!(done.clears_action);
        assert!(done.clears_field_204);
        assert_eq!(done.result, 0);
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
}
