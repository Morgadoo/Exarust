# Proposal

## Why

Round 60 lit the burning blade (ability 3) but left four things open:

- abilities 1 and 2 (the lightning mace and the self-sharpening sword) were
  traced but not ported;
- the checks ran for the player only;
- nothing filled a socket: the item card drew the socket row, the panels
  dragged items, but no drop reached `Sockets::drop_crystal`;
- lighting a blade sets a body byte whose reader was not traced.

Tracing them also showed that two earlier ports had read these routines
under the wrong names. The "glow" tick and level callback in
`exa_game::item_effects` are the self-sharpening blade's check and hit hook:
their "level" is the weapon's slash byte and their "light" the weapon
record's slash share (W11). The grip's "half-sword" turn of mode 2 into 1
is the burning blade's: it tests the ability word 3 and the lit-blade byte.

Phase: Phase 5/6 combat and world milestones of docs/PLAN.md (S12 item
abilities). This change does not edit PLAN.md.

## Evidence

REA (Ghidra on `Exanima.exe`), checked against the disassembly:

- the held-item ability install and its three checks;
- the lightning check's charge test, the arcs' raise and end, the hum arcs'
  preset, its hit hook (shock call, deferred charge and spark burst);
- the self-sharpening check, its hit hook and the `ln`/`exp` helpers;
- the burning blade's lighting and putting out;
- the moveset choice (the byte cleared), the two-handed grip callback (the
  byte read);
- the socket control's procedure, its drop, the card's commit callback and
  the give-back to the player.

Data: `objdb.rdb` (the four ability records, read on the worker with
`EXANIMA_DIR`).

## What Changes

- `exa_game::item_ability`: every character's checks; types 1 and 2; the
  lit-blade byte and the grip re-run on lighting; putting out keeps the
  overlay; hit hooks for all three types; `--ability-item N[,cold][,npc|,npc-only]`
  (`--flame-sword` stays).
- `exa_game::item_effects`: the "glow" helpers renamed for what they are
  (`sharpen_tick`, `blunt_on_hit`, `slash_share`); the dispatch names
  lightning, self-sharpening and burning.
- `exa_game::socket_drag` (new) and `exa_game::panel_live`: a crystal
  dropped on the shown card's socket, dragged out of it, given back; the
  panels follow the scripted pointer.
- `exa_game::item_card`: the socket row's place is kept for the drop.
- `exa_game::thaum::arcs`: the hum arcs' preset.
- `exa_character::held`: `BurningGrip`, the lit-blade byte; a moveset choice
  from new hands drops the overlay and the byte.
- `exa_character::grip_runtime`: `HeldAbility`; the two-hander's turn reads
  the ability and the byte.
- `exa_physics::grip`: the turn's parameters renamed.

## Out of scope

- The AR crystal's fourth game function (logged only).
- The original's event queue that defers the lightning hit's charge and
  burst (run at once here).
- The spark burst riding the struck body (it stays where it struck).
- One counter for the held abilities and the worn effects: the original
  keeps one list per body; Exarust keeps two clocks.
