# Proposal

## Why

The game GUI unit and the GUI form class still lacked the loot window
("Body"), the power slot's draw, the inventory preview's settings and build
callback, the preview bars' rectangles, the HUD reset, the status marker's
draw values, the form tree's eager layout and the message sent by control
id. A cheaper model drafted these; this change keeps the pieces that match
the instructions and corrects the rest.

## What Changes

- `exa_ui::game_gui`: the loot window's title, size, place and panels; the
  power slot's draw (frame, centred icon, inks, empty shadow and kind
  icon); the preview's model values, words, build callback and bars; the
  HUD reset with the GUI hold count; the status glow radius, share, tint
  and hide clock; the selector's unchecked row write (a byte of 9 marks
  the forget row).
- `exa_ui::form`: the show's eager layout down the form tree and the
  message sent to a control by its id.
- `exa_game`: a dead body's double click opens its loot window, one per
  body, drawn with its pack and equipment panels and registered as an open
  container; a right-button click release over the inventory or a loot
  window closes it; the deferred close's windows come in window order.
- Docs: `docs/UI.md` and the `exa_ui` and `exa_game` READMEs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the rules above.

## Impact

- Wired: the loot window (open, draw, registry, close), the right-release
  close. Not wired: the character view and its preview bars, the power
  bar's slots on the HUD, the status marker's draw, the HUD reset's caller,
  the deferred close's hook, the form tree layout (the show still places
  each screen itself), and the loot panels' contents (no body inventory
  exists for characters other than the player; the body's flag word is
  passed as 0).
- Evidence: the game GUI and form routines read as assembly
  (`context/re/packs/gamegui/`), plus the loot window's constructor, the
  inventory's camera refit and its right-release handler.
- Out of scope: the 3-D character view, the technique selector's form, the
  power tree, the item description card.

## Phase

Phase 7, game modes and content: the UI section (in-game screens and HUD);
the loot window also belongs to Phase 4, combat and items.
