## Why

Roadmap task B1 (wiring lane `wireskin`): the GUI draw skin (TGUIDrawSkin)
and the game's line skin (TGSLines) had many ported routines with no live
caller, and the live painters drew some line-skinned controls with stand-ins
(findings F27, F29, F72, F87 and the separator's routine). In 0.9.5 every
form that holds control records is line-skinned, so the control classes'
skin calls land in the line skin's slots; the draw skin's own control slots
are reached only by the editor's colour picker.

## Evidence

REA (Exanima.exe), checked against the assembly where order, constants or
branches matter:

- The separator's draw calls the four-ink line loop for every arm, with the
  hot-form group's shades in the order top, left, right, bottom.
- The label class's draw message: held bit, then style 0x20000000 (block
  draw, fitted to the record less twice its margin), then an ink pointer,
  else the plain draw with its hovered glow; its back calls the skin's
  frame slot, which the line skin leaves empty.
- The line skin's label slot (buttons): disabled, hovered (glowing text a
  pixel lower while lit), on or lit, else idle.
- The combo class's back calls the line skin's rail; the rail's thumb sits
  at the box's right edge plus three units, less four and six units, and
  its hovered ink is the highlight group's eighth shade at the GUI's alpha.
- The text field's back calls the frame slot (empty) and, with a frame bit,
  the inner frame slot (the capped span) from one pixel in.
- The image-button class's back calls the frame and box slots (both empty
  on the line skin) and its front the line skin's centred icon.
- The domain wheel and the power tree call the skin's rounded fills; the
  spline renderer writes one fade byte per row, from the row's first
  vertex index over the begin's segment count, and the tree adds every
  point in mode 0.

## What Changes

- `settings_form::paint_control`: separators through `skin::edge_frame`;
  labels through `gui_paint::paint_label_front` (no frame on the line skin).
- `gui_paint::paint_line_button`: the hovered caption glows
  (`push_glow_text`, shared with `base_form`).
- `gui_paint::paint_combo_back` (rail) in `hub_forms` and `save_screen`;
  `skin::draw_line_rail` thumb place and hover ink fixed.
- `gui_paint::paint_text_field_back` (capped span) in `option_forms`.
- `item_card::paint_flip_button` through `line_skin::draw_line_icon`.
- `wheel_draw::label_draw` through `skin::rounded_fill`, `tree_draw::skin_box`
  through `skin::rounded_fill_fixed` (`tree_draw::submit_rounded`).
- `tree_draw::link_ribbon`: one dim fade byte per row.
- The test-only label models `place_line_label` and `place_skin_label`
  (duplicates of the live label draws, with a wrong alignment switch) are
  removed.
