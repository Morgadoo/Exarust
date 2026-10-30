//! The item GUI's popups: the item card, the open-container popup and the
//! parchment panel.
//!
//! The item card is 268 by 260, tagged `0xCE1C2012`; it fades in and out
//! over 256 clock units on a squared curve (the fade-in is the square of
//! the elapsed share, the fade-out the square of the remaining share), shows
//! its item icon at the top centre (the plain card centres it), and closes
//! once its fade-out ends. The container popup is tagged `0xCE1C2013`, wraps
//! an item panel whose object is 544 bytes, and saves the panel into the
//! container's record when it closes. The parchment is a borderless
//! `0xCE1C2014` panel sized to a sixth of a unit per texture pixel and
//! hit-tested through the texture's alpha.
//!
//! Evidence: the three popups' constructors, close, draw and hit handlers
//! (`docs/UI.md`, item GUI).

use crate::skin::SkinBatch;

/// The item card's size, tag, flags and digit.
pub const DESC_WIDTH: i32 = 268;
pub const DESC_HEIGHT: i32 = 260;
pub const DESC_TAG: u32 = 0xce1c_2012;
pub const DESC_FLAGS: u32 = 0x310f0;
/// Bits the constructor sets in the form's second flag word.
pub const DESC_EXTRA_FLAGS: u32 = 0x3080;
/// The form's margin field (`0xc`); the plain icon placement subtracts it.
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
    /// The clock value the current fade ends at.
    pub fade_at: u32,
    /// The clock value from which the card takes the pointer: once the
    /// clock reaches it during the fade-in (or the fade-in ends first) the
    /// GUI re-runs its pointer test with the force flag, once. A separate
    /// field from [`fade_at`](Self::fade_at); the code that sets it is
    /// outside this unit.
    pub shown_at: u32,
    /// The forced pointer test has run; the card's hit test answers only
    /// from then on ([`desc_hits`]).
    pub pointer_ready: bool,
}

/// One fade step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupFrame {
    pub alpha: f32,
    pub close: bool,
    /// Re-run the GUI's pointer test now, forced (once per showing).
    pub refresh_pointer: bool,
}

impl PopupFade {
    /// Starts the fade-in, reversing a fade-out in progress the way
    /// [`fade_out`](Self::fade_out) reverses a fade-in. The retail show routine
    /// is outside this unit, so this mirrors the fade-out start.
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

    /// The per-frame step. The clock compares are unsigned: a fade over
    /// (its end at or below `now`) ends it, and the squared curve is the
    /// remaining share (fade-out) or one less it (fade-in) squared in
    /// double, narrowed to float.
    pub fn step(&mut self, now: u32) -> PopupFrame {
        let mut refresh_pointer = false;
        match self.state {
            1 => {
                if !self.pointer_ready && self.shown_at <= now {
                    self.pointer_ready = true;
                    refresh_pointer = true;
                }
                if self.fade_at > now {
                    let left = i64::from(self.fade_at) - i64::from(now);
                    let t = 1.0 - left as f64 * FADE_RATE;
                    return PopupFrame {
                        alpha: (t * t) as f32,
                        close: false,
                        refresh_pointer,
                    };
                }
                self.state = 0;
                if !self.pointer_ready {
                    self.pointer_ready = true;
                    refresh_pointer = true;
                }
                PopupFrame {
                    alpha: 1.0,
                    close: false,
                    refresh_pointer,
                }
            }
            2 => {
                if self.fade_at > now {
                    let left = i64::from(self.fade_at) - i64::from(now);
                    let t = left as f64 * FADE_RATE;
                    PopupFrame {
                        alpha: (t * t) as f32,
                        close: false,
                        refresh_pointer,
                    }
                } else {
                    self.state = 0;
                    PopupFrame {
                        alpha: 0.0,
                        close: true,
                        refresh_pointer,
                    }
                }
            }
            _ => PopupFrame::default(),
        }
    }
}

/// One clock unit's share of the fade (`1 / 256`).
const FADE_RATE: f64 = 0.003_906_25;

impl Default for PopupFrame {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            close: false,
            refresh_pointer: false,
        }
    }
}

/// Where the card's icon goes (`card` true: the stats card's layout).
///
/// For the plain card `width` is the form's client width minus its margin
/// ([`DESC_DIGIT`] field) and `height` its icon-area height field (rescaled by
/// [`desc_rescale`]); the card layout uses `icon_height` only.
pub fn desc_icon_place(
    width: i32,
    height: i32,
    icon_height: i32,
    unit: i32,
    card: bool,
) -> (i32, i32) {
    desc_icon_place_sized(width, height, icon_height, unit, card, ICON_SIZE)
}

/// [`desc_icon_place`] for an icon of side `icon_size` in place of the
/// renderer's 160 (the running card scales its icon with the unit).
pub fn desc_icon_place_sized(
    width: i32,
    height: i32,
    icon_height: i32,
    unit: i32,
    card: bool,
    icon_size: i32,
) -> (i32, i32) {
    if !card {
        return (
            (width.wrapping_sub(icon_size)) / 2,
            (height.wrapping_sub(icon_size)) / 2 + unit * 4,
        );
    }
    let mut x = unit * 13 - (icon_size >> 1);
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

/// The keep-open bit a left press sets in the card's form flags; the leave
/// rule ([`desc_hover`]) then leaves the card up.
pub const DESC_KEEP_OPEN: u32 = 2;

/// The card's mouse rule (its press slot, which takes both a press, `0x1000`,
/// and a click, `0x2000`), applied after the base form handler (whose answer
/// the card returns unchanged): any left-button event sets the keep-open bit
/// (`2`) in the form's flags; otherwise any right-button event starts the
/// fade-out ([`PopupFade::fade_out`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescMouse {
    pub keep_open: bool,
    pub fade_out: bool,
}

pub fn desc_mouse(mask: u16) -> DescMouse {
    if mask & 1 != 0 {
        DescMouse {
            keep_open: true,
            fade_out: false,
        }
    } else {
        DescMouse {
            keep_open: false,
            fade_out: mask & 2 != 0,
        }
    }
}

/// The card's hit test only works once its forced pointer test has run
/// ([`PopupFade::pointer_ready`]).
pub fn desc_hits(shown: bool, base: i32) -> i32 {
    if shown { base } else { 0 }
}

/// The result slot value a hover answer of `-1` stores in the card.
pub const DESC_HOVER_SLOT: i32 = -3;

/// The card's hover rule, applied after the base form handler. An answer of
/// `-1` stores [`DESC_HOVER_SLOT`] in the form's result slot (returned);
/// an answer of `0` hides the card (a fade-out of 256 units from `now`) unless
/// it is hidden, flagged keep-open (`flags & 2`) or already fading out.
pub fn desc_hover(
    result: i32,
    visible: bool,
    flags: u32,
    now: u32,
    fade: &mut PopupFade,
) -> Option<i32> {
    if result == -1 {
        return Some(DESC_HOVER_SLOT);
    }
    if result == 0 && visible && flags & 2 == 0 && fade.state != 2 {
        fade.state = 2;
        fade.fade_at = now.wrapping_add(POPUP_FADE);
    }
    None
}

/// The container popup's tag and its inner panel's layout.
pub const CONTAINER_TAG: u32 = 0xce1c_2013;
pub const CONTAINER_FLAGS: u32 = 0x83_0023;
/// The value the constructor stores in the form's `0x68` field.
pub const CONTAINER_FIELD: i32 = 0x100;
/// The form's margin field (the card's [`DESC_DIGIT`] slot), eight units.
pub const CONTAINER_MARGIN: i32 = 8;
/// Bit the constructor clears in the form's second flag word.
pub const CONTAINER_EXTRA_CLEAR: u32 = 0x80;
/// The item panel object's size in bytes (its class record's first word).
pub const CONTAINER_PANEL: u32 = 544;
/// The panel's `0x20` field and `0x30` field values.
pub const CONTAINER_INSET: i32 = 0;
pub const CONTAINER_STEP: i32 = 4;

/// The container close: notify the game of the panel's record id, have the
/// panel save its contents back into the container's record (releasing its
/// icons), clear the panel's record id and run the card's close (which
/// releases the card's icon). A popup that is already closing leaves all of
/// that alone and answers true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerClose {
    pub notify_game: bool,
    /// The panel writes its entries back into the container record, with
    /// the icon release flag set.
    pub save_contents: bool,
    pub clear_id: bool,
    pub run_base: bool,
}

pub fn container_close(closing: bool) -> ContainerClose {
    if closing {
        ContainerClose {
            notify_game: false,
            save_contents: false,
            clear_id: false,
            run_base: false,
        }
    } else {
        ContainerClose {
            notify_game: true,
            save_contents: true,
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
    pub field: i32,
    pub margin: i32,
    pub extra_clear: u32,
    pub layout: u32,
    /// The inner panel's `0x20` and `0x30` field values.
    pub inset: i32,
    pub step: i32,
}

pub fn container_form() -> ContainerForm {
    ContainerForm {
        tag: CONTAINER_TAG,
        flags: CONTAINER_FLAGS,
        field: CONTAINER_FIELD,
        margin: CONTAINER_MARGIN,
        extra_clear: CONTAINER_EXTRA_CLEAR,
        layout: CONTAINER_PANEL,
        inset: CONTAINER_INSET,
        step: CONTAINER_STEP,
    }
}

/// The container's press slot. `None`: not a right-button event, run the
/// card's mouse rule. `Some(true)`: a right click (`0x2000`) closes the popup
/// and answers 1. `Some(false)`: a right press is answered 0 and ignored, so
/// the container popup does not fade on a right press as the card does.
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
/// The hit test maps a click to a texture pixel at six pixels per unit; the
/// size is the inverse, a sixth of a unit per pixel.
pub const PARCHMENT_SCALE: f32 = 6.0;
/// One texture pixel in units (`1 / 6`, a double in the original).
const PARCHMENT_PIXEL: f64 = 1.0 / 6.0;

/// The parchment's size from its texture's dimensions: each is the pixel
/// count times the unit over six (the scale is rounded to a float first).
pub fn parchment_size(width: u16, height: u16, unit: i32) -> (i32, i32) {
    let scale = (f64::from(unit) * PARCHMENT_PIXEL) as f32;
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

/// The parchment's press slot: any right-button event (press or click)
/// hides it and answers 3; any other event runs the base handler.
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

/// The result a parchment hit answers (the control id goes to the caller).
pub const PARCHMENT_HIT: i32 = -3;

/// The texture pixel under a click: the click times six over the unit,
/// rounded half to even. The factor is divided in double precision and
/// narrowed to single before the single-precision product.
pub fn parchment_texel(click: (i32, i32), unit: i32) -> (i32, i32) {
    let k = (f64::from(PARCHMENT_SCALE) / f64::from(unit)) as f32;
    (
        (click.0 as f32 * k).round_ties_even() as i32,
        (click.1 as f32 * k).round_ties_even() as i32,
    )
}

/// The parchment's hit test: a texture sample at or above 0xF0 hits and the
/// answer is [`PARCHMENT_HIT`] with the form and its control's id.
pub fn parchment_hit(sample: u8, control: i32) -> Option<i32> {
    parchment_hits(sample).then_some(control)
}

/// The parchment's draw quad as `(x, y, u, v)` from the form's position and
/// size; the texture's rows run bottom-up, so the top edge has `v = 1`.
pub fn parchment_quad(x: i32, y: i32, width: i32, height: i32) -> [[f32; 4]; 4] {
    let (x0, y0) = (x as f32, y as f32);
    let (x1, y1) = (
        (i64::from(x) + i64::from(width)) as f32,
        (i64::from(y) + i64::from(height)) as f32,
    );
    [
        [x0, y0, 0.0, 1.0],
        [x0, y1, 0.0, 0.0],
        [x1, y1, 1.0, 0.0],
        [x1, y0, 1.0, 1.0],
    ]
}

/// The parchment's draw: the textured program (mode 2), its texture bound,
/// white ink at the GUI's alpha byte (`alpha`, 255 unless a fade lowered it)
/// and the quad of [`parchment_quad`] in its vertex order.
#[derive(Debug, Clone, PartialEq)]
pub struct ParchmentDraw {
    pub batch: SkinBatch,
    /// The GUI's alpha scales go back to 1 after the draw: the form was
    /// fading and its fade had finished.
    pub restore_alpha: bool,
}

/// Builds the parchment's draw. While the form is fading (`fading`), its
/// fade step runs first (`fade_step`, which sets the GUI's alpha scales):
/// a step that answers false draws nothing; a step whose fade end field is
/// zero (`fade_done`) has the alpha scales reset after the draw.
pub fn parchment_draw(
    rect: (i32, i32, i32, i32),
    alpha: u8,
    fading: bool,
    fade_step: impl FnOnce() -> bool,
    fade_done: bool,
) -> Option<ParchmentDraw> {
    let mut restore_alpha = false;
    if fading {
        if !fade_step() {
            return None;
        }
        restore_alpha = fade_done;
    }
    let mut batch = SkinBatch::new();
    batch.select(2, 0, true);
    batch.texture_binds += 1;
    batch.set_rgba(0xff, 0xff, 0xff, alpha);
    batch.begin();
    for [x, y, u, v] in parchment_quad(rect.0, rect.1, rect.2, rect.3) {
        batch.set_uv(u, v);
        batch.vertex(x, y);
    }
    batch.end_quads();
    Some(ParchmentDraw {
        batch,
        restore_alpha,
    })
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
        f.shown_at = 1256;
        assert_eq!(f.fade_at, 1256);
        // A quarter of the way in the card is one sixteenth opaque.
        assert_eq!(f.step(1064).alpha, 0.0625);
        assert_eq!(f.step(1128).alpha, 0.25);
        // Three quarters of the way in: (3/4)^2.
        assert_eq!(f.step(1192).alpha, 0.5625);
        let step = f.step(1256);
        assert_eq!(step.alpha, 1.0);
        assert!(step.refresh_pointer);
        assert_eq!(f.state, 0);
        f.fade_out(2000);
        assert_eq!(f.fade_at, 2256);
        assert_eq!(f.step(2128).alpha, 0.25);
        assert_eq!(f.step(2064).alpha, 0.5625);
        assert!(f.step(2256).close);
    }

    #[test]
    fn the_pointer_test_is_forced_once_when_the_card_is_ready() {
        let mut f = PopupFade::default();
        f.show(1000);
        f.shown_at = 1100;
        assert!(!f.step(1099).refresh_pointer);
        assert!(f.step(1100).refresh_pointer);
        // It runs once.
        assert!(!f.step(1101).refresh_pointer);
        assert!(!f.step(1256).refresh_pointer);
        // A fade-in that ends before the shown clock forces the test when
        // it ends.
        let mut f = PopupFade::default();
        f.show(1000);
        f.shown_at = 1300;
        assert!(!f.step(1255).refresh_pointer);
        assert!(f.step(1256).refresh_pointer);
    }

    #[test]
    fn a_fade_reverses_where_it_stands() {
        let mut f = PopupFade::default();
        f.show(1000);
        // Half way in, the fade-out takes half the time to finish.
        f.fade_out(1128);
        assert_eq!(f.fade_at, 1128 * 2 + 256 - 1256);
        assert_eq!(f.fade_at, 1256);
        assert_eq!(f.step(1128).alpha, 0.25);
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
    fn the_mouse_and_hover_rules() {
        assert_eq!(
            desc_mouse(1),
            DescMouse {
                keep_open: true,
                fade_out: false
            }
        );
        assert_eq!(
            desc_mouse(2),
            DescMouse {
                keep_open: false,
                fade_out: true
            }
        );
        assert_eq!(desc_hits(true, 4), 4);
        assert_eq!(desc_hits(false, 4), 0);
        let mut f = PopupFade::default();
        // An answer of -1 only stores the slot value.
        assert_eq!(desc_hover(-1, true, 0, 900, &mut f), Some(-3));
        assert_eq!(f.state, 0);
        // An answer of 0 hides the card.
        assert_eq!(desc_hover(0, true, 0, 900, &mut f), None);
        assert_eq!((f.state, f.fade_at), (2, 900 + 256));
        // Keep-open, hidden and already fading out all leave it alone.
        let mut g = PopupFade::default();
        desc_hover(0, true, 2, 900, &mut g);
        desc_hover(0, false, 0, 900, &mut g);
        assert_eq!(g.state, 0);
        desc_hover(0, true, 0, 1000, &mut f);
        assert_eq!(f.fade_at, 900 + 256);
    }

    #[test]
    fn the_forms_and_parchment_helpers() {
        let d = desc_form();
        assert_eq!((d.tag, d.width, d.height), (DESC_TAG, 268, 260));
        assert!(desc_close(true));
        assert!(!desc_close(false));
        let c = container_form();
        assert_eq!(
            (c.tag, c.flags, c.field, c.margin),
            (CONTAINER_TAG, CONTAINER_FLAGS, 0x100, 8)
        );
        assert_eq!(
            (c.layout, c.inset, c.step, c.extra_clear),
            (544, 0, 4, 0x80)
        );
        let p = parchment_form();
        assert_eq!((p.tag, p.flags, p.visible), (PARCHMENT_TAG, 0x80000, false));
        assert_eq!(parchment_mouse(2), ParchmentMouse::Hide);
        assert_eq!(parchment_mouse(0), ParchmentMouse::Base);
        assert_eq!(parchment_hit(0xf0, 7), Some(7));
        assert_eq!(parchment_hit(0xef, 7), None);
    }

    #[test]
    fn the_parchment_quad_runs_bottom_up() {
        let q = parchment_quad(10, 20, 30, 40);
        assert_eq!(q[0], [10.0, 20.0, 0.0, 1.0]);
        assert_eq!(q[1], [10.0, 60.0, 0.0, 0.0]);
        assert_eq!(q[2], [40.0, 60.0, 1.0, 0.0]);
        assert_eq!(q[3], [40.0, 20.0, 1.0, 1.0]);
    }

    #[test]
    fn the_parchment_draws_one_textured_quad() {
        let d = parchment_draw((10, 20, 30, 40), 255, false, || unreachable!(), true).unwrap();
        assert!(!d.restore_alpha);
        assert_eq!(d.batch.mode, crate::skin::SkinMode::Textured);
        assert_eq!(d.batch.texture_binds, 1);
        assert_eq!(d.batch.primitives.len(), 1);
        let v = d.batch.vertices_of(&d.batch.primitives[0]);
        assert_eq!(v.len(), 4);
        assert_eq!(v[0].ink, [255, 255, 255, 255]);
        let quad = parchment_quad(10, 20, 30, 40);
        for (vertex, [x, y, u, w]) in v.iter().zip(quad) {
            assert_eq!((vertex.x, vertex.y, vertex.u, vertex.v), (x, y, u, w));
        }
        // A fade that answers false draws nothing; a finished one restores.
        assert!(parchment_draw((0, 0, 1, 1), 255, true, || false, true).is_none());
        let faded = parchment_draw((0, 0, 1, 1), 128, true, || true, true).unwrap();
        assert!(faded.restore_alpha);
        let v = faded.batch.vertices_of(&faded.batch.primitives[0]);
        assert_eq!(v[0].ink[3], 128);
        assert!(
            !parchment_draw((0, 0, 1, 1), 255, true, || true, false)
                .unwrap()
                .restore_alpha
        );
    }

    #[test]
    fn the_parchment_maps_clicks_to_texels() {
        // At four pixels to the unit a click is six sixths: 1.5 texels a pixel.
        assert_eq!(parchment_texel((4, 10), 4), (6, 15));
        // 1.5 rounds half to even.
        assert_eq!(parchment_texel((1, 3), 4), (2, 4));
    }

    #[test]
    fn the_container_and_parchment() {
        let c = container_close(false);
        assert!(c.notify_game && c.save_contents && c.clear_id && c.run_base);
        assert!(!container_close(true).run_base);
        assert_eq!(container_right_click(0x2002), Some(true));
        assert_eq!(container_right_click(0x0002), Some(false));
        assert_eq!(container_right_click(0x2000), None);
        // A texture pixel is a sixth of a unit, rounded half to even.
        assert_eq!(parchment_size(64, 32, 4), (43, 21));
        assert_eq!(parchment_size(60, 30, 4), (40, 20));
        assert!(parchment_hits(0xf0));
        assert!(!parchment_hits(0xef));
    }
}
