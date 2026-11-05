# Proposal

## Why

Round 54 left two inventory gaps:

- The player's interaction on a living character whose interaction
  condition passes (or that carries the player's Loyalty or Command Dead
  marker) opens that character's inventory window in the original, with
  the same call the player's own body makes. Exarust had no view for it
  and only logged it; a script's `ShowInventory` of another character was
  dropped the same way.
- The hub's trade window and loadout form show a character preview, but
  dressing it from the item panels and taking items off it were not
  ported (only the arsenal box's equip drop reached the loadout preview,
  and the trade window drew no preview at all).

Phase: Phase 6 (story mode: NPCs, companions and their inventories) and
Phase 7 (arena hub: trade and loadout) of docs/PLAN.md; no PLAN.md edit in
this change (the owner's).

Evidence (REA, Ghidra on `Exanima.exe` 0.9.5, checked against the
assembly): the inventory show call, its per-character finder, its bind
(place for another character, the remembered place written only for the
player's own window), its close; the controls' interaction hook and the
`ShowInventory` builtin (actor index clamped to 0, no toggle); the
character view's constructor, accept test, drop and release; the trade
window's constructor, opener and layout, its drop-back and give-back
callbacks; the loadout form's constructor and the preview, kit list and
arsenal box callbacks it installs. Corpus: `charroles.rdb` of the installed
build, scanned for interaction-condition sections.

## What Changes

- `exa_game::gui_screens`: the inventory window is bound to a character
  (`inventory_owner`, `inventory_body`): the show rule runs for that
  character, the window registers under its tag, the pack binds its
  carried list and its owner, another character's window opens at the
  screen's right edge less its width and four units, and only the
  player's own window remembers its place. A `ShowInventory` of another
  character opens that character's window (no toggle).
- `exa_game::character_use`: an interaction that opens the inventory now
  asks for that character's window (with the toggle).
- `exa_game::doll_view`, `figure_pick`, `panel_live`: the character view's
  figure, its hands, its drop and its drag off, and the pack's moves, read
  the bound character instead of the player; the view's scene is rebuilt
  when the window is bound to another character.
- `exa_game::loadout_kit`: a kit-list entry dropped on the preview dresses
  the tab's loadout behind the kit accept test (a copy the hands do not
  hold) and the rank and points gate; a press on the preview's figure
  picks the item under the pointer and drags it; released over the kit
  list it comes off, over the arsenal box it comes off and goes back to the
  arsenal (the kit list losing one copy, each loadout trimmed).
- `exa_game::shop_preview` (new) and `arena_hub_live`: the trade window's
  preview shows a copy of the player's record; a panel's item dropped on
  it dresses the copy (displaced pieces and hand items that are not the
  shop's go to the pack at its centre, the braies never); a drag off it
  takes the item off, a shop item wherever it lands.
- `exarust --character-use ID@FRAME[+]`: a debug tool sending the player's
  interaction on a character (`+` first records the player's Loyalty
  marker on it).

## Out of scope

- More than one inventory window at once: the original keeps one per
  character; Exarust has one, and asking for another character's closes
  the open one [Exarust stand-in].
- The hub previews' camera and backdrop: both use the creator's preview
  view (floor scene, its camera), not the character view's own; the
  previews show no held items, so a held item cannot be picked off them.
- The trade window's "Trade" commit of the preview's copy into the
  player's record.
- The character view's mind costs (the inventory of a mind-controlled
  character) and the doll's lock byte for a second character.
