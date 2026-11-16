## Why

Roadmap task B4 (lane `arena3`, docs/roadmap/phase-b-close.md), after the
`arenafade` lane (round 254): the end menu's Continue still returned to the
hub, and a dead manager's Continue still left, at once instead of behind the
transition screen; a tournament's later bracket round rebuilt the company's
bodies instead of keeping them; the arena arsenal's columns were never
filled and its controls never answered; the board's item slots opened no
card; and the hub entry did not start the manager's entry motion.

## Evidence

The disassembly (`context/re/exe.asm`, `context/re/staging-freeze.md`):

- The end menu's Continue after the last round ends the tournament's
  bracket, sets the return placement and passes the return routine to the
  transition screen's opener with the music fade on; the return routine
  raises the pause counter while it is zero, writes the health words back,
  settles the match, loads the hub and ends with the playing phase. A dead
  manager's Continue makes the manager the controlled record and passes the
  leave to the title (music fade on).
- The next round and the bracket round both write the company's health
  words back first. A later bracket round drops the dead from its kept
  list, takes the kept bodies back after the reload (kit equip, level
  attach, the kept-body reset, whose non-player clocks step the roll) and
  stands them on the company side's places, body `k` at place `k` of the
  side's member-count spread; only the other sides are generated.
- The arsenal's open runs its shelf load on the first shelf button; a shelf
  button that goes down stores the five columns into the shown shelf's
  boxes, makes its shelf the shown one and loads its five boxes; the arrows
  scroll by one and lay out; the toggle shows the inventory while down; the
  close stores the shown shelf.
- The board's five item controls' callback looks the icon's item up and
  builds its card at once with no flags.
- The hub entry starts the manager's motion 0x13 with no blend and no
  capture before its living placement.
- The match start, after the map load, compares the scene's wind (single,
  widened) with the double 0.1 and over it calls every sim object of the
  simulation's first sector at VMT +0xf8 (slot 19: the FPC VMT's virtual
  methods start at +0x60) with the word 0x20; the cloth's slot 19 raises
  its sleep countdown to that, the base slot clears two state bits.

## What changes

- `exa_game::flow`: `Action::ArenaReturn` (a staging; its stage
  `ArenaStage::Return` counts as a level change, the phase ends playing).
- `exa_game::arena_fade`: the held round slot also holds the return.
- `exa_game::arena_live`: the Continue's return and the dead manager's
  leave open the screen; the bracket ends at the Continue and the match
  settles at black; both round stagings write the health words back; a
  later bracket round keeps the company's living bodies.
- `exa_game::match_layout::kept_side_places`.
- `exa_game::arsenal_view`: the shelf load, the store at the close, the
  controls' clicks (`ArsenalButton`).
- `exa_ui::match_board`: the item slots' hit and `BoardAction::ShowItem`;
  the hub's click asks for the item's card (`item_card::WorldCard`).
- `exa_game::arena_hub_live`: the hub entry's motion before the placement.
- `exa_game::arena_live::wake_cloths`: a windy first round wakes the
  level's cloths to 32 (posted to the fixed step).

## Determinism

The return and the dead manager's leave now run 600 ms later (after the
fade-out), the return's three frames at black are paused, and the hub's
load runs behind the black screen. A later team bracket round keeps the
company's bodies (their health, their places) and steps the shared roll
once per kept non-player. Determinism runs owed: the arena scenes with
`--arena-auto` (standard and tournament), the hub's Begin and return.

## Impact

Arena traces change at the return, at every next round and bracket round
(the health write-back changes the records, not the bodies) and at a team
tournament's later rounds. The hub's traces change at the entry (the
manager's first clip). The arsenal now edits the company's stores.
