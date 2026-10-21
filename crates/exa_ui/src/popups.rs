//! The item GUI's popups: the item card, the open-container popup and the
//! parchment panel.
//!
//! The item card is 268 by 260, tagged `0xCE1C2012`; it fades in and out
//! over 256 clock units on a squared curve, shows its item icon at the
//! top centre (the plain card centres it), and closes once its fade-out
//! ends. The container popup is tagged `0xCE1C2013`, holds a 544-byte
//! panel at eight units, and saves the panel into the container's record
//! when it closes. The parchment is a borderless `0xCE1C2014` panel sized
//! to its texture in six-unit steps and hit-tested through the texture's
//! alpha.
//!
//! Evidence: the three popups' constructors, close, draw and hit handlers
//! (`docs/UI.md`, item GUI).

/// The item card's size, tag, flags and digit.
pub const DESC_WIDTH: i32 = 268;
pub const DESC_HEIGHT: i32 = 260;
pub const DESC_TAG: u32 = 0xce1c_2012;
pub const DESC_FLAGS: u32 = 0x310f0;
pub const DESC_EXTRA_FLAGS: u32 = 0x3080;
pub const DESC_DIGIT: i32 = 0xc;
/// The card fades over this many clock units.
pub const POPUP_FADE: u32 = 256;

/// The item card's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescForm {
    pub tag: u32,
    pub width: i32,
    pub height: i32,
    pub flags: u32,
    pub extra_flags: u32,
    pub digit: i32,
}

/// The item card's constructor values.
pub fn desc_form() -> DescForm {
    DescForm {
        tag: DESC_TAG,
        width: DESC_WIDTH,
        height: DESC_HEIGHT,
        flags: DESC_FLAGS,
        extra_flags: DESC_EXTRA_FLAGS,
        digit: DESC_DIGIT,
    }
}

/// The item icon's size, the same as the icon renderer's.
pub const ICON_SIZE: i32 = 160;

/// The card's fade state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PopupFade {
    /// 0 idle, 1 fading in, 2 fading out.
    pub state: u8,
    pub fade_at: u32,
    pub shown: bool,
}

/// One fade step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupFrame {
    pub alpha: f32,
    pub close: bool,
    /// The card just became fully shown (its page hook runs once).
    pub shown_now: bool,
}

impl PopupFade {
    /// The show: fades in over 256 units, reversing a fade-out.
    pub fn show(&mut self, now: u32) {
        if self.state == 2 {
            self.fade_at = now
                .wrapping_mul(2)
                .wrapping_add(POPUP_FADE)
                .wrapping_sub(self.fade_at);
        } else {
            self.fade_at = now.wrapping_add(POPUP_FADE);
        }
        self.state = 1;
    }

    /// Begins a fade-out (no-op while already fading out).
    pub fn fade_out(&mut self, now: u32) {
        if self.state == 2 {
            return;
        }
        if self.state == 1 {
            self.fade_at = now
                .wrapping_mul(2)
                .wrapping_add(POPUP_FADE)
                .wrapping_sub(self.fade_at);
        } else {
            self.fade_at = now.wrapping_add(POPUP_FADE);
        }
        self.state = 2;
    }

    /// The per-frame step.
    pub fn step(&mut self, now: u32) -> PopupFrame {
        let mut shown_now = false;
        if self.state == 1 {
            if !self.shown && self.fade_at <= now {
                self.shown = true;
                shown_now = true;
            }
            if now < self.fade_at {
                let left = u64::from(self.fade_at) - u64::from(now);
                let t = left as f64 / f64::from(POPUP_FADE);
                return PopupFrame {
                    alpha: (t * t) as f32,
                    close: false,
                    shown_now,
                };
            }
            self.state = 0;
            if !self.shown {
                self.shown = true;
                shown_now = true;
            }
            PopupFrame {
                alpha: 1.0,
                close: false,
                shown_now,
            }
        } else if self.state == 2 {
            if now < self.fade_at {
                let left = u64::from(self.fade_at) - u64::from(now);
                let t = left as f64 / f64::from(POPUP_FADE);
                PopupFrame {
                    alpha: (t * t) as f32,
                    close: false,
                    shown_now,
                }
            } else {
                self.state = 0;
                PopupFrame {
                    alpha: 0.0,
                    close: true,
                    shown_now,
                }
            }
        } else {
            PopupFrame {
                alpha: 1.0,
                close: false,
                shown_now,
            }
        }
    }
}

impl Default for PopupFrame {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            close: false,
            shown_now: false,
        }
    }
}

/// Where the card's icon goes (`card` true: the stats card's layout).
pub fn desc_icon_place(
    width: i32,
    height: i32,
    icon_height: i32,
    unit: i32,
    card: bool,
) -> (i32, i32) {
    if !card {
        return (
            (width.wrapping_sub(ICON_SIZE)) / 2,
            (height.wrapping_sub(ICON_SIZE)) / 2 + unit * 4,
        );
    }
    let mut x = unit * 13 - ICON_SIZE / 2;
    let y = unit;
    if unit * 11 < icon_height {
        x = x.wrapping_add(icon_height).wrapping_sub(unit * 11);
    }
    (x, y)
}

/// The card's rescale: its height becomes the unit times its share of the
/// parent's height, rounded half to even.
pub fn desc_rescale(height: i32, parent_height: i32, unit: i32) -> i32 {
    (f64::from(unit) * (f64::from(height) / f64::from(parent_height))).round_ties_even() as i32
}

/// The card's mouse rule: a left press marks it handled; a right release
/// starts the fade-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescMouse {
    pub handled: bool,
    pub fade_out: bool,
}

pub fn desc_mouse(mask: u16, base: bool) -> DescMouse {
    if mask & 1 == 0 {
        DescMouse {
            handled: base,
            fade_out: mask & 2 != 0,
        }
    } else {
        DescMouse {
            handled: true,
            fade_out: false,
        }
    }
}

/// The card's hit test only works once it is fully shown.
pub fn desc_hits(shown: bool, base: i32) -> i32 {
    if shown { base } else { 0 }
}

/// The card's leave rule: hide it when the cursor leaves, unless the
/// child flag or the no-hide bit says otherwise.
pub fn desc_leave(
    result: i32,
    child: bool,
    visible: bool,
    flags: u32,
    state: u8,
    now: u32,
    fade: &mut PopupFade,
) -> bool {
    if result == -1 && child && visible && flags & 2 == 0 && state != 2 {
        fade.state = 2;
        fade.fade_at = now.wrapping_add(POPUP_FADE);
        return true;
    }
    false
}

/// The container popup's tag and its inner panel's layout.
pub const CONTAINER_TAG: u32 = 0xce1c_2013;
pub const CONTAINER_FLAGS: u32 = 0x83_0023;
pub const CONTAINER_DIGIT: i32 = 0x100;
pub const CONTAINER_PANEL: u32 = 544;
/// The panel's first inset and spacing.
pub const CONTAINER_INSET: i32 = 0;
pub const CONTAINER_STEP: i32 = 4;

/// The container close: notify the game, save the panel and clear the
/// container id. A popup already closing leaves all of that alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerClose {
    pub notify_game: bool,
    pub save_panel: bool,
    pub clear_id: bool,
    pub run_base: bool,
}

pub fn container_close(closing: bool) -> ContainerClose {
    if closing {
        ContainerClose {
            notify_game: false,
            save_panel: false,
            clear_id: false,
            run_base: false,
        }
    } else {
        ContainerClose {
            notify_game: true,
            save_panel: true,
            clear_id: true,
            run_base: true,
        }
    }
}

/// The item card's close: it releases the icon it held.
pub fn desc_close(has_icon: bool) -> bool {
    has_icon
}

/// The container popup's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerForm {
    pub tag: u32,
    pub flags: u32,
    pub digit: i32,
    pub layout: u32,
    /// The inner panel's first inset and step.
    pub inset: i32,
    pub step: i32,
}

pub fn container_form() -> ContainerForm {
    ContainerForm {
        tag: CONTAINER_TAG,
        flags: CONTAINER_FLAGS,
        digit: CONTAINER_DIGIT,
        layout: CONTAINER_PANEL,
        inset: CONTAINER_INSET,
        step: CONTAINER_STEP,
    }
}

/// The container's right-click: a release carrying `0x2000` closes it.
pub fn container_right_click(mask: u16) -> Option<bool> {
    if mask & 2 == 0 {
        None
    } else if mask & 0x2000 == 0x2000 {
        Some(true)
    } else {
        Some(false)
    }
}

/// The parchment's tag and its alpha-testing hit threshold.
pub const PARCHMENT_TAG: u32 = 0xce1c_2014;
pub const PARCHMENT_FLAGS: u32 = 0x80000;
/// A texture sample at or above this alpha hits.
pub const PARCHMENT_ALPHA: u8 = 0xf0;
/// The parchment's size is its texture dimension times six units.
pub const PARCHMENT_SCALE: f32 = 6.0;

/// The parchment's size from its texture's dimensions.
pub fn parchment_size(width: u16, height: u16, unit: i32) -> (i32, i32) {
    let scale = unit as f32 * PARCHMENT_SCALE;
    (
        (f32::from(width) * scale).round_ties_even() as i32,
        (f32::from(height) * scale).round_ties_even() as i32,
    )
}

/// The parchment popup's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParchmentForm {
    pub tag: u32,
    pub flags: u32,
    pub visible: bool,
}

pub fn parchment_form() -> ParchmentForm {
    ParchmentForm {
        tag: PARCHMENT_TAG,
        flags: PARCHMENT_FLAGS,
        visible: false,
    }
}

/// The parchment's mouse: a right release hides it and answers 3; any
/// other event runs the base handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParchmentMouse {
    Hide,
    Base,
}

pub fn parchment_mouse(mask: u16) -> ParchmentMouse {
    if mask & 2 != 0 {
        ParchmentMouse::Hide
    } else {
        ParchmentMouse::Base
    }
}

/// The parchment's hit test: the texture point is the click times six over
/// the unit; a sample at or above 0xF0 hits with the control's id.
pub fn parchment_hit(click: (i32, i32), unit: i32, sample: u8, control: i32) -> Option<i32> {
    let _ = (
        (click.0 as f32 * (PARCHMENT_SCALE / unit as f32)) as i32,
        (click.1 as f32 * (PARCHMENT_SCALE / unit as f32)) as i32,
    );
    parchment_hits(sample).then_some(control)
}

/// The parchment's draw quad, in its own pixels.
pub fn parchment_quad(width: i32, height: i32) -> [[f32; 2]; 4] {
    [
        [0.0, 0.0],
        [0.0, height as f32],
        [width as f32, height as f32],
        [width as f32, 0.0],
    ]
}

/// The parchment's destroy releases its image.
pub fn parchment_destroy() -> (bool, bool) {
    (true, true)
}

/// A text sample hits the parchment at or above [`PARCHMENT_ALPHA`].
pub fn parchment_hits(sample_alpha: u8) -> bool {
    sample_alpha >= PARCHMENT_ALPHA
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_fades_on_a_squared_curve() {
        let mut f = PopupFade::default();
        f.show(1000);
        assert_eq!(f.fade_at, 1256);
        assert_eq!(f.step(1128).alpha, 0.25);
        let step = f.step(1256);
        assert_eq!(step.alpha, 1.0);
        assert!(step.shown_now);
        assert_eq!(f.state, 0);
        f.fade_out(2000);
        assert_eq!(f.fade_at, 2256);
        assert_eq!(f.step(2128).alpha, 0.25);
        assert!(f.step(2256).close);
    }

    #[test]
    fn the_icon_has_two_places() {
        assert_eq!(desc_icon_place(268, 260, 160, 4, false), (54, 66));
        assert_eq!(desc_icon_place(268, 260, 160, 4, true), (88, 4));
        // A taller icon pushes the stats card's icon right.
        assert_eq!(desc_icon_place(268, 260, 200, 4, true), (128, 4));
        assert_eq!(desc_rescale(260, 600, 4), 2);
    }

    #[test]
    fn the_mouse_and_leave_rules() {
        assert_eq!(
            desc_mouse(1, false),
            DescMouse {
                handled: true,
                fade_out: false
            }
        );
        assert_eq!(
            desc_mouse(2, false),
            DescMouse {
                handled: false,
                fade_out: true
            }
        );
        assert_eq!(desc_hits(true, 4), 4);
        assert_eq!(desc_hits(false, 4), 0);
        let mut f = PopupFade::default();
        assert!(desc_leave(-1, true, true, 0, 0, 900, &mut f));
        assert_eq!((f.state, f.fade_at), (2, 900 + 256));
        assert!(!desc_leave(-1, true, true, 2, 0, 900, &mut f));
    }

    #[test]
    fn the_forms_and_parchment_helpers() {
        let d = desc_form();
        assert_eq!((d.tag, d.width, d.height), (DESC_TAG, 268, 260));
        assert!(desc_close(true));
        assert!(!desc_close(false));
        let c = container_form();
        assert_eq!(
            (c.tag, c.flags, c.digit),
            (CONTAINER_TAG, CONTAINER_FLAGS, 0x100)
        );
        assert_eq!((c.layout, c.inset, c.step), (544, 0, 4));
        let p = parchment_form();
        assert_eq!((p.tag, p.flags, p.visible), (PARCHMENT_TAG, 0x80000, false));
        assert_eq!(parchment_mouse(2), ParchmentMouse::Hide);
        assert_eq!(parchment_mouse(0), ParchmentMouse::Base);
        assert_eq!(parchment_hit((4, 4), 4, 0xf0, 7), Some(7));
        assert_eq!(parchment_hit((4, 4), 4, 0xef, 7), None);
        assert_eq!(parchment_quad(10, 20)[2], [10.0, 20.0]);
        assert_eq!(parchment_destroy(), (true, true));
    }

    #[test]
    fn the_container_and_parchment() {
        let c = container_close(false);
        assert!(c.notify_game && c.save_panel && c.clear_id && c.run_base);
        assert!(!container_close(true).run_base);
        assert_eq!(container_right_click(0x2002), Some(true));
        assert_eq!(container_right_click(0x0002), Some(false));
        assert_eq!(container_right_click(0x2000), None);
        assert_eq!(parchment_size(64, 32, 4), (1536, 768));
        assert!(parchment_hits(0xf0));
        assert!(!parchment_hits(0xef));
    }
}
