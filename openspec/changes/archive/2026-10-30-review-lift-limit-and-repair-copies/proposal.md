# Proposal

## Why

A lane extended the player controls unit: the lift's placement limit at
the player's height, the last frame a failed lift frame returns to, the
world double click's use cursor, the GUI drag claim's reach flag, the live
game-mode table, the timed list's section order, the wheel grip's NaN
branch, a GUI drop's record components, the repair's item copies and the
sweep's support pull-back. A review against the original's assembly found
that the lift's own placement keeps a hit only when it is strictly over the
limit and refuses a climbing ray even with a hit (the lane reused the GUI
drop's tests), that a free placement's last frame was the grab point's and
moved the part onto it when restored, that the wheel's grip test lets a NaN
take the first grip (the lane flipped it), that the timed list's 0x1000
section is the apparel one and falls through to the 0x4000 weapon section
when written (the lane stopped after it), that the repair's copies read the
apparel bytes eight too low and based the weapon price on the template's
quality while duplicating the refresh's condition passes, that the world use
cursor must carry the tagged object id doors name as their key and close the
item card, that the Spectate mode record has its own hooks, that the
support pull-back takes the world point into the frame's axes, and that a
GUI drop inserted its placed object twice and could look up another
record for a level-tagged id.

## What Changes

- `item_cursor::lift_point` for the live lift; `place_point` stays the GUI
  drop's.
- A free placement keeps the part's frame as the last one.
- `hand::first_wheel_grip` lets a NaN take the first grip again.
- `TimedList::tick_sections` tries the apparel (0x1000) section, then the
  weapon (0x4000) section when the apparel byte was written or absent.
- `exa_items::refresh::{copy_apparel, copy_weapon}` over the new shared
  `apparel_condition_pass` and `weapon_condition_pass`; the raw-offset
  copies in `instance` are gone.
- The world use cursor holds the tagged id, slot and panel zero, after the
  card closes.
- `GameModeHook` names the Spectate record's own pair.
- `support_corrections` uses the frame's transposed rotation.
- `gui_drop` builds the placed object once and only for a game record.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: lift placement, timed list, game modes, lift frames,
  world use cursor, GUI drop record, sweep pull-back.
- `item-instances`: the repair's copies.

## Impact

`crates/exa_game/src/{item_cursor,hand,player_controls,object_click,gui_drop}.rs`,
`crates/exa_items/src/{instance,refresh}.rs`, `docs/PLAYER.md`, the
`exa_game` and `exa_items` READMEs.

## Phase

Phase 4, gameplay systems (player controls: lifting, using and dropping
items).

## Evidence

The controls frame's lift branch and its marker/free placement helper, the
constrained drop helper, the wheel turn's grip test, the timed list, the
two section copies and the shared apparel and weapon condition routines,
the double click's object interaction, the GUI drag claim hook, the unit
init's mode records and their name strings, and the sweep's target search,
in the shipped executable's disassembly.

## Out of scope

The lift placement's navigation-region refusal (a hit node with a region
whose segment test fails refuses the frame; Exarust has no live region), the
weapon copy's factory stat pass, a live store of item conditions for the
timed list, the Spectate session, the sweep's CPU sphere-against-mesh narrow
phase (the support pull-back has no caller), and the full 4×3 body frame the
original restores (Exarust shifts the part frames rigidly).
