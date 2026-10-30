# Proposal

## Why

Review of the round-44 UI sweep lane over sixteen small GUI components
(docs/PLAN.md Phase 6, interface and menus). The lane drew the seven basic
control classes, the table's flat header, the choice list, the colour
picker's callbacks, the drop-down band, the inline text picture, the edit
page's refresh tail, the destination screen's colours and several
constructor constants. Several rules were ported from pseudo-code or
guessed: they are corrected here against the assembly.

Evidence: the seven class dispatches and the skin slots they call (the
draw skin's class table, its panel, box and image slots), the held label
draw, the table draw and its choice-list opener with the general form
placement, the colour picker's control callbacks, the drop-down draw and
the skin's selection band, the text renderer's tag draw, the edit page
refresh and destroy, the body form constructor, the destination paint,
the skill window constructor, the splash constructor and load, and the
physique cross's weight split (REA and the objdump listing).

## What Changes

- `exa_ui::gui_controls`: the check box's back is the skin's panel slot;
  the image button's front calls the existing `skin::draw_skin_image`
  instead of a partial copy; an icon button's own-atlas icon draws as it
  is while pressed; a held label ends by drawing plainly, takes the
  fourth shade whole while focused and keeps fading on an unordered
  fade; the invented marker-stripe fallback is removed.
- `exa_ui::table_view::ChoiceList::place_request` replaces the misread
  "height": the list's placement request for `gui_core::place_form`.
- `exa_ui::colour_picker::PickerEvent`: the callbacks dispatch to the HSV
  sliders, the field's hue and saturation and the revert; the invented
  preview colour is removed.
- `exa_ui::drop_down::DropDownList::paint`: the band's last layer is the
  skin's selection band.
- `exa_ui::text_layout::TextPicture::corners`: the original's vertex
  order.
- `exa_ui::picker_fields::body_shape`: an unordered point takes the right
  half again (the lane had inverted it).
- `exa_game::create`: the duplicate `preview_update` is removed; the body
  form's records 7, 9 and 10 take one image atlas (`atlas_controls`).
- `exa_ui::selection::skinned_palette`: the backdrop keeps its guessed
  form shade; docs lose raw object offsets.

## Out of scope

- Wiring the control class draws into a live form page (no running form
  lays out these records yet).
- The skin's first flag bit (the shipped skins never set it; its line
  slot is abstract in the draw skin's class).
- The emblem form's picker sync beyond the palette bytes, the backdrop's
  own draw on the destination screen and the marker words.
