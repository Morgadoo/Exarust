## Why

Phase 7 (UI) in docs/PLAN.md, `TGUIDrawSkin` and `TGSLines`. The draw skin's panel, shell, scroll bar, slider and
selection band had models but no running caller, and the line skin's slider and scroll bar (one method, 3,339 bytes)
had not been re-verified against the assembly.

## What the original does

REA (Exanima.exe), the skin classes' virtual tables and the assembly of the skins' makers and the record classes'
draws:

- **Which skin a form keeps.** The base form constructor gives every form the draw skin's default instance, then an
  owner's skin when it has an owner. At start the game gives the desktop the textured skin and makes the message box
  maker ask for the line skin; the game's screens set the line skin (inventory, journal, settings, hub windows, shop,
  creator, manual) or the textured skin (dialogue choices, skill selector, divination card, item parchment).
- **The line skin overrides** the form draw, hit test, scroll refresh, programs, alpha hand-off, the three frame slots,
  the label, the panel, the shell, the slider and the scroll bar, so the draw skin's panel, shell, slider and scroll
  bar run only for forms on the default instance or the textured skin.
- **The colour picker** is made with no owner and no skin set: it draws with the default instance (its own scheme:
  blue-grey groups, alphas 224 and 125). Its six channel trackbars are marker trackbars whose colour lists the
  constructor points at the picker's ramps and the hue table.
- **Record draws.** A button's back calls the skin's shell slot and its front the label slot; a check box's back the
  panel slot; a trackbar's back the style-frames and slider slots, its front the plain label draw.
- **The line skin's slider** (flat and marker modes), checked instruction by instruction: the corner quads, the thumb
  at `unit + round((w − 6·unit)·value)`, the marker's stripes over the trough, its twelve edge quads with the dispatcher's
  two-unit lip and its two-bar thumb, and the inks (0x10 disabled, 0x200000 the highlight colour at two thirds of the
  GUI alpha, otherwise the label group's seventh shade; hovered thumbs in the highlight colour).
- **The selection band** is called directly by the table, item and tree views, the drop-down list and the popup menu;
  the drop-down list's band is drawn in the hub's combo popups.

## What Changes

- `exa_ui::skin`: the default instance's scheme (`BASE_SCHEME`), its palette and shown palette, and the titled client
  top under its measures (`BASE_TITLED_CLIENT_TOP`).
- `exa_ui::record_form`: a record form can keep a client top of its own skin's (`client_top`).
- `exa_game::base_form`: a form on the default instance draws its default background and title, its buttons through
  the skin's shell and label slot, its trackbars through the style frames, the skin's slider and the plain label, and
  its other records through their own draws in its palette. The hub's colour picker draws this way; its trackbars show
  the striped lists and the value bar the original draws, replacing the line-skin frame and the approximate gradient.

## Impact

- Crates: `exa_ui`, `exa_game`.
- The duel and exanima01 lockstep traces are unchanged.
