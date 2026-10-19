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
}
