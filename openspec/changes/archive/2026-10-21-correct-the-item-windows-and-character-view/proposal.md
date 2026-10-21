# Proposal

## Why

The item-window and character-view lane extended the item panel, the item
GUI controls, the card text and the character view's rules. A review of
every new rule against the original's instructions found departures:

- Layouts: the grid divided in the wrong order and precision, gave its
  first slot to the first entry and clamped once instead of after each
  pass; placing a new entry compared the overlap's magnitude, so far-away
  entries pushed it; the scatter drew from a generator seeded with a
  constant the original stores for the run-time library's own generator;
  the worn panel's spot was redrawn over the whole panel (the original
  draws a quarter plus half the width) and its push used each axis's half
  extent where the original reads the horizontal one for both.
- Codecs: the list save wrote the x position as y, both codecs rounded
  half away from zero and saturated instead of rounding half to even and
  keeping the low word, a record load kept a placeholder id, and a record
  was saved for any panel.
- Drops and cards: a drop from the panel matched its entry by id instead of
  icon, a drop from the linked panel added bulk, the dropped entry was
  clamped by half the cell and bottom before top; the accept test's own
  check ignored the drag mark; the mouse rules lacked the press bit; the
  card request was modelled as a toggle opening after two frames where the
  original asks for a delayed card on a 200 ms timer by the item's use
  type; the card's pronoun markers were matched as numbers where the
  original names them (`HeShe`, `HimHer`, `HisHer`, `HisHers`); the
  weapon handling and striking rows were computed in single precision.
- Controls: a socket row's commit claimed both item sections, a finished
  socket drag cleared the charge byte, and a duplicate, unpacked copy of
  the striking rows was added.
- Character view: the camera offset pointed the wrong way, the background
  used the 1.4 boost the original applies only without sRGB targets, the
  "camera follow" was the race-share easing with an invented step factor,
  the AL test read the wrong race word, the hover index is a held item's
  hand (not a worn index), and the release decision gave items back to the
  pack where the original takes the dragged item off.
- A gift to an open pack panel that refuses it was given to the carried
  list; the original loses it.

## What Changes

- The layouts, codecs, drops, accept, hit test, mouse and card rules follow
  the original; the card text's condition blocks (the template's first
  half) are added.
- The socket commit names its target section; the striking rows are packed.
- The character view's camera, background, easing, drag acceptance, shown
  item, drag start and release follow the original.
- Docs: `docs/UI.md` and the crate READMEs describe the corrected rules
  without retail offsets.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: item-panel relaxation, hit test and drop, accept and
  positions, mouse intent; new item-panel records, card request and
  character-view requirements.
- `inventory-pane`: panel entries (the worn panel's push), item GUI
  controls (socket commit); new weapon handling and card text requirements.
- `carried-items`: giving and having (a refused gift).

## Impact

- `exa_game::item_container`, `exa_items::{item_gui, card, possession}`,
  `exa_ui::character_view`; `exa_assets::human_skin::srgb_decode` becomes
  public for the background.
- Evidence: the itemgui evidence pack's assembly listing for each rule
  (the grid's narrowing and its round-up helper, the placement's signed
  compare, the generator and seed globals, the worn panel's index without
  the axis, the codecs' conversions, the drop and accept bodies, the card
  request and its timer, the pronoun table read from the executable, the
  weapon card's mixed precision, the character view's constructor, its
  background and task callbacks, hover pick, drag procedures and drop).
- Out of scope: drawing any item window, card or the 3D figure; wiring the
  rules into a running GUI form (there is no host form yet); held-model
  attachment and the record calls behind a figure drop; the trade view's
  prices.

## Phase

Phase 4 (items), `docs/PLAN.md` section 7 item 2 (the item windows' rules).
