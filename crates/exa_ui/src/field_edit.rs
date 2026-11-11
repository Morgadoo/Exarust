//! The text field's and combo box's own editing (`RF_GUI_Controls_Ex`):
//! their key message (0xa0) through the text edit unit
//! ([`crate::text_edit`]), the caret a press or drag places, the focus
//! message's selection and the colour swatch's drop.
//!
//! Read from the two controls' message handlers. A key reaches the
//! control the form has focused (the record under the pointer): the
//! message word is the key's byte, its second argument's bit 2 says the
//! byte was typed, and the edit unit reads Ctrl (0x40) and Shift (0x20)
//! from the key-state word. Rules: `docs/UI.md` (Arena option forms).

use crate::controls::{LabelAt, align_label, combo_text_span, text_field_span, text_style};
use crate::gui_font::GuiFont;
use crate::text_edit::{self, EditAction, EditView};

/// One key message: the key's byte, whether it was typed (the message's
/// bit 2) and the key-state word's Ctrl and Shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldKey {
    pub byte: u8,
    pub typed: bool,
    pub ctrl: bool,
    pub shift: bool,
}

/// The Enter byte: with an owner callback it submits instead of editing.
pub const ENTER: u8 = 0x0d;

/// The state bits a text field's Enter sets while its owner runs (the
/// emblem name's preset tests them).
pub const SUBMITTED: u32 = 0x10_0000;
/// The state bits a combo box's Enter sets while its owner runs.
pub const COMBO_SUBMITTED: u32 = 0x30_0000;
/// The state bit a combo box's edit sets while its owner runs.
pub const COMBO_EDITED: u32 = 0x20_0000;

/// What a key did, for the control's owner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyEffect {
    /// The owner's callback ran with these state bits set for the call
    /// (0: a plain notify).
    pub call: Option<u32>,
    /// Enter gave the focus up (the text field's release style).
    pub release_focus: bool,
    /// The edit unit's clipboard request.
    pub clipboard: Option<EditAction>,
    /// The control redraws (every handled key but a dropped byte).
    pub redraw: bool,
}

/// The fields a key edits: the text, the four edit bytes, the style, the
/// length limit (the word at 0x58) and whether the owner gave a callback.
pub struct FieldWords<'a> {
    pub text: &'a mut Vec<u8>,
    pub view: &'a mut EditView,
    pub style: u32,
    pub max_len: i32,
    pub has_callback: bool,
}

/// A text field's key message. Enter with a callback runs it with
/// [`SUBMITTED`] set and, with the release style, gives the focus up. An
/// editable field (style bits 0xd) drops a typed non-digit when it takes
/// digits only (no redraw); otherwise the edit unit takes the key with
/// the field's limit and span and, with the notify style and a callback,
/// the owner is called after every key. A read-only field passes the key
/// to the edit unit's moves. `clipboard` is read only for Ctrl+V.
pub fn text_field_key(
    font: &GuiFont,
    words: FieldWords<'_>,
    w: i32,
    pad: i32,
    unit: i32,
    key: FieldKey,
    clipboard: Option<&[u8]>,
) -> KeyEffect {
    let span = text_field_span(w, pad, unit);
    let mut effect = KeyEffect {
        redraw: true,
        ..KeyEffect::default()
    };
    if key.byte == ENTER && words.has_callback {
        effect.call = Some(SUBMITTED);
        effect.release_focus = words.style & text_style::RELEASE != 0;
        return effect;
    }
    if words.style & text_style::EDITABLE == text_style::EDITABLE {
        if words.style & text_style::DIGITS == text_style::DIGITS
            && key.typed
            && !key.byte.is_ascii_digit()
        {
            return KeyEffect::default();
        }
        let action = text_edit::apply_key(
            font,
            words.text,
            words.view,
            key.byte,
            key.typed,
            key.ctrl,
            key.shift,
            words.max_len,
            span,
            clipboard,
        );
        if action != EditAction::None {
            effect.clipboard = Some(action);
        }
        if words.style & text_style::NOTIFY != 0 && words.has_callback {
            effect.call = Some(0);
        }
    } else if let Some(copied) = text_edit::navigate_key(
        font, words.text, words.view, key.byte, key.typed, key.ctrl, key.shift, span,
    ) {
        effect.clipboard = Some(EditAction::Copy(copied));
    }
    effect
}

/// A combo box's key message. Enter sets [`COMBO_SUBMITTED`] and runs the
/// owner's callback when it has one; any other key goes to the edit unit
/// with the box's limit and span, whatever the box's editable bits, and
/// with the notify style the owner runs with [`COMBO_EDITED`] set.
pub fn combo_key(
    font: &GuiFont,
    words: FieldWords<'_>,
    w: i32,
    pad: i32,
    unit: i32,
    key: FieldKey,
    clipboard: Option<&[u8]>,
) -> KeyEffect {
    let mut effect = KeyEffect {
        redraw: true,
        ..KeyEffect::default()
    };
    if key.byte == ENTER {
        if words.has_callback {
            effect.call = Some(COMBO_SUBMITTED);
        }
        return effect;
    }
    let action = text_edit::apply_key(
        font,
        words.text,
        words.view,
        key.byte,
        key.typed,
        key.ctrl,
        key.shift,
        words.max_len,
        combo_text_span(w, pad, unit),
        clipboard,
    );
    if action != EditAction::None {
        effect.clipboard = Some(action);
    }
    if words.style & text_style::NOTIFY != 0 && words.has_callback {
        effect.call = Some(COMBO_EDITED);
    }
    effect
}

/// The edit bytes a control is created with (its reset message): the run
/// from the start to the text's end, the anchor and caret at the end.
pub fn created_view(text: &[u8]) -> EditView {
    let len = text.len().min(u8::MAX as usize) as u8;
    EditView {
        anchor: len,
        caret: len,
        length: len,
        first: 0,
    }
}

/// The focus message (0xb0) with word 0 on a control whose state has the
/// focus bit: the anchor goes to 0 and the caret to the text's end, so
/// the whole text is selected.
pub fn focus_gained(text: &[u8], view: &mut EditView) {
    view.anchor = 0;
    view.caret = text.len().min(u8::MAX as usize) as u8;
}

/// The text field's text origin: the label alignment over the field's
/// span with the drawn run's width, the main font's ascent and glyph
/// height (the field's own copy of the alignment).
pub fn text_field_origin(
    rect: (i32, i32, i32, i32),
    pad: i32,
    unit: i32,
    style: u32,
    font: &GuiFont,
    text: &[u8],
    view: &EditView,
) -> LabelAt {
    let (x, y, w, h) = rect;
    let run = crate::controls::run_width(text, view.first, view.length, &font.advances);
    let span = text_field_span(w, pad, unit);
    align_label(
        x,
        y,
        h,
        pad,
        unit,
        run,
        span,
        font.ascent,
        font.glyph.1,
        style,
    )
}

/// The caret under a pointer `x` (screen space): the drawn run walked from
/// the text origin ([`text_edit::caret_from_x`]). A drag (0x30) places the
/// caret only, so the selection grows from the anchor; then the run
/// scrolls to keep the caret in view.
pub fn drag_caret(
    font: &GuiFont,
    text: &[u8],
    view: &mut EditView,
    origin_x: i32,
    x: i32,
    span: i32,
) -> u8 {
    let caret = text_edit::caret_from_x(font, text, view, x - origin_x);
    text_edit::scroll_view(font, text, view, span);
    caret
}

/// A left press (0x40) on a field in its hovered state (state bits
/// 0xf0000 equal to 0x10000): the caret goes under the pointer and the
/// anchor joins it, then the run scrolls. Answers the press's 0x15.
pub fn press_caret(
    font: &GuiFont,
    text: &[u8],
    view: &mut EditView,
    origin_x: i32,
    x: i32,
    span: i32,
) -> u32 {
    text_edit::caret_from_x(font, text, view, x - origin_x);
    view.anchor = view.caret;
    text_edit::scroll_view(font, text, view, span);
    0x15
}

/// Whether a press reaches the text field's caret: the left button and
/// the hovered state word.
pub fn press_places_caret(button_word: u32, state: u32) -> bool {
    button_word & 7 == 1 && state & 0xf_0000 == 0x1_0000
}

/// A colour swatch's drop: the dragged colour word becomes the swatch's
/// own (its word at 0x38) and its drop callback (the word at 0x40) runs.
/// Answers the drop's 3.
pub fn swatch_drop(colour: &mut u32, dropped: u32) -> u32 {
    *colour = dropped;
    3
}

/// The emblem swatches' drop callback: the form's colour slot (the
/// swatch's word at 0x48) takes the whole dropped word, palette index in
/// the alpha byte included; the preview's copy takes it too. No refresh
/// runs.
pub fn emblem_drop(colours: &mut [u32; 4], slot: usize, dropped: u32) {
    if let Some(colour) = colours.get_mut(slot) {
        *colour = dropped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> GuiFont {
        let mut advances = [0i16; 256];
        for a in advances.iter_mut() {
            *a = 8;
        }
        GuiFont {
            texture: "t",
            glyph: (8, 12),
            ascent: 10,
            slant: 0,
            gap: 2,
            line_height: 12,
            advances,
        }
    }

    fn key(byte: u8, typed: bool) -> FieldKey {
        FieldKey {
            byte,
            typed,
            ..FieldKey::default()
        }
    }

    /// The emblem name field's style: editable, notify and release.
    const NAME: u32 = 0x3c0_3b2d;
    /// The picture number fields': editable, digits only.
    const NUMBER: u32 = 0x400_330d;

    #[test]
    fn typing_inserts_at_the_caret_and_replaces_a_selection() {
        let f = font();
        let mut text = b"Ravns".to_vec();
        let mut view = created_view(&text);
        // The focus selects all; a typed byte replaces it.
        focus_gained(&text, &mut view);
        assert_eq!((view.anchor, view.caret), (0, 5));
        let e = text_field_key(
            &f,
            FieldWords {
                text: &mut text,
                view: &mut view,
                style: NAME,
                max_len: 20,
                has_callback: true,
            },
            160,
            4,
            4,
            key(b'R', true),
            None,
        );
        assert_eq!(text, b"R");
        // The name's notify style calls the owner on every key, without
        // the submitted bit.
        assert_eq!(e.call, Some(0));
        for (byte, typed) in [(b'v', true), (text_edit::key::LEFT, false), (b'a', true)] {
            text_field_key(
                &f,
                FieldWords {
                    text: &mut text,
                    view: &mut view,
                    style: NAME,
                    max_len: 20,
                    has_callback: true,
                },
                160,
                4,
                4,
                key(byte, typed),
                None,
            );
        }
        assert_eq!(text, b"Rav");
        assert_eq!(view.caret, 2);
        // Delete takes the byte after the caret; End moves to the end.
        for byte in [text_edit::key::DELETE, text_edit::key::END] {
            text_field_key(
                &f,
                FieldWords {
                    text: &mut text,
                    view: &mut view,
                    style: NAME,
                    max_len: 20,
                    has_callback: true,
                },
                160,
                4,
                4,
                key(byte, false),
                None,
            );
        }
        assert_eq!(text, b"Ra");
        assert_eq!((view.anchor, view.caret), (2, 2));
    }

    #[test]
    fn enter_submits_and_releases_the_focus_with_the_release_style() {
        let f = font();
        let mut text = b"klean".to_vec();
        let mut view = created_view(&text);
        let e = text_field_key(
            &f,
            FieldWords {
                text: &mut text,
                view: &mut view,
                style: NAME,
                max_len: 20,
                has_callback: true,
            },
            160,
            4,
            4,
            key(ENTER, false),
            None,
        );
        assert_eq!(e.call, Some(SUBMITTED));
        assert!(e.release_focus);
        assert_eq!(text, b"klean");
        // A number field keeps the focus.
        let mut text = b"1".to_vec();
        let mut view = created_view(&text);
        let e = text_field_key(
            &f,
            FieldWords {
                text: &mut text,
                view: &mut view,
                style: NUMBER,
                max_len: 2,
                has_callback: true,
            },
            28,
            4,
            4,
            key(ENTER, false),
            None,
        );
        assert_eq!(e.call, Some(SUBMITTED));
        assert!(!e.release_focus);
    }

    #[test]
    fn a_digits_field_drops_a_typed_letter_and_stops_at_its_limit() {
        let f = font();
        let mut text = b"1".to_vec();
        let mut view = created_view(&text);
        let press = |text: &mut Vec<u8>, view: &mut EditView, k: FieldKey| {
            text_field_key(
                &f,
                FieldWords {
                    text,
                    view,
                    style: NUMBER,
                    max_len: 2,
                    has_callback: true,
                },
                28,
                4,
                4,
                k,
                None,
            )
        };
        let e = press(&mut text, &mut view, key(b'x', true));
        assert_eq!(e, KeyEffect::default());
        press(&mut text, &mut view, key(b'2', true));
        press(&mut text, &mut view, key(b'3', true));
        assert_eq!(text, b"12");
        // Not notify: no call on an edit.
        let e = press(&mut text, &mut view, key(text_edit::key::BACKSPACE, false));
        assert_eq!(e.call, None);
        assert_eq!(text, b"1");
    }

    #[test]
    fn a_combo_box_edits_its_text_and_submits_with_its_bits() {
        let f = font();
        // The merchant's boxes: not editable, no notify; keys still edit.
        let style = 0x840_3320;
        let mut text = b"Anything".to_vec();
        let mut view = created_view(&text);
        let e = combo_key(
            &f,
            FieldWords {
                text: &mut text,
                view: &mut view,
                style,
                max_len: 255,
                has_callback: true,
            },
            0xa8,
            4,
            4,
            key(text_edit::key::BACKSPACE, false),
            None,
        );
        assert_eq!(text, b"Anythin");
        assert_eq!(e.call, None);
        let e = combo_key(
            &f,
            FieldWords {
                text: &mut text,
                view: &mut view,
                style,
                max_len: 255,
                has_callback: true,
            },
            0xa8,
            4,
            4,
            key(ENTER, false),
            None,
        );
        assert_eq!(e.call, Some(COMBO_SUBMITTED));
        // Without a callback Enter only redraws.
        let e = combo_key(
            &f,
            FieldWords {
                text: &mut text,
                view: &mut view,
                style,
                max_len: 255,
                has_callback: false,
            },
            0xa8,
            4,
            4,
            key(ENTER, false),
            None,
        );
        assert_eq!(e.call, None);
        assert!(e.redraw);
    }

    #[test]
    fn a_press_places_the_caret_and_a_drag_selects() {
        let f = font();
        let text = b"abcdef".to_vec();
        let mut view = created_view(&text);
        assert!(press_places_caret(1, 0x1_0000));
        assert!(!press_places_caret(1, 0x2_0000));
        assert!(!press_places_caret(2, 0x1_0000));
        // Bytes 8 wide: x 20 from the origin is past half of the third.
        assert_eq!(press_caret(&f, &text, &mut view, 100, 120, 200), 0x15);
        assert_eq!((view.anchor, view.caret), (3, 3));
        drag_caret(&f, &text, &mut view, 100, 139, 200);
        assert_eq!((view.anchor, view.caret), (3, 5));
    }

    #[test]
    fn a_dropped_colour_fills_the_emblem_slot_whole() {
        let mut word = 0xff28_3ca0;
        assert_eq!(swatch_drop(&mut word, 0x1312_1110), 3);
        assert_eq!(word, 0x1312_1110);
        let mut colours = [1, 2, 3, 4];
        emblem_drop(&mut colours, 2, 0xd3aa_bbcc);
        assert_eq!(colours, [1, 2, 0xd3aa_bbcc, 4]);
        emblem_drop(&mut colours, 9, 0);
        assert_eq!(colours, [1, 2, 0xd3aa_bbcc, 4]);
    }
}
