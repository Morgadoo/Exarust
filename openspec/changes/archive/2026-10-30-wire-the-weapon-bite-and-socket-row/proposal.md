# Proposal

## Why

Backlog items C2 and P2 (docs/PLAN.md Phase 6, the live combat and
inventory wiring of round 43). The bite stage (`exa_combat::bite::BiteHold`)
was built only by its tests, so no blow ever stuck a weapon in a body; the
item card drew no crystal-socket row although the card facts carried the
socket words, and the crystal sheet was never loaded.

Evidence (REA, Ghidra listing of `Exanima.exe` 0.9.5, and the objdump
listing):
- The damage routine opens the bite only on a wound on flesh, after the
  attacker's follow-through, when the bite depth widened to a double is
  above 0.1 and the striker's weapon class has a hands bit (0x30). Fists
  (0x80), claws (0xC0) and nothing held never stick, whatever the race;
  a turned blow returns before it.
- The bite's lifetime unit is the link-iteration count of the weapon
  body's tree root: the holder's body (round(1660/83) = 20 for a living
  character; the body tick ramps it to 15 for a fully feral one, 10 once
  dead). The count is the product rounded to nearest (the processor's
  conversion), not truncated. The count is the iteration limit of flag
  0x4000, which the bite's flags do not carry, so only the strain matters.
- The grab's axis is the striker's per-tick velocity (cm a tick) dotted
  with the weapon matrix's rows; its points are the contact on the weapon
  and on the struck part.
- The bite layers (0x40, 0xC0) are the bodies' layer masks, the word the
  pair stage reads, not their class words.
- The socket control's draw: the empty ring is the rating-dot sheet's
  third cell in the GUI's icon atlas, drawn in the skin palette's ink one
  after the rating dots' (the label group's eighth), at x advancing five
  units per existing socket, vertically centred on the large-icon size;
  then each socket with a crystal draws, at its own slot, the crystal
  sheet's frame (cell 1, 2 with kind bit 0x10) in opaque white with alpha
  blending and the crystal (cells 3–6) in white at alpha
  round((255 − charge) × 1.0) with additive blending. The crystal sheet's
  cells are the GUI level's large-icon size. The card builder adds the
  socket control (0x78 wide, 0x18 high) when the first socket exists and
  adds 0x18 to the page.

The task was framed as "feral bites" (zombies). The original has no
separate feral bite: the bite is the weapon stick, open to any armed
striker. A feral holder only shortens it through its smaller
link-iteration count. Unarmed zombies never bite.

## What Changes

- `exa_physics::body`: the grab layers live in the layer mask
  (`Pairing::layer`); `Ragdoll::pull_grab_across` (a weapon body and the
  struck body), `weights_now`, `layer`/`set_layer`,
  `STUCK_WEAPON_LAYER`.
- `exa_combat::bite`: `bites` (the routine's gate), the rounded count,
  `grab_axis`, `BitePoints`, a cross-body `BiteHold` stage of many bites
  found by part entities, a `BiteTaken` message, `open_bites` (link,
  grab, layers, log) and the break log; installed by `CombatPlugin`.
- `exa_combat` `react_to_hits`: a wound by a held weapon with β > 0.1
  asks for the bite.
- `exa_game::item_card`: the socket cells (ring + six crystal cells at
  the large-icon side), `socket_row_quads`/`socket_row_layer`, the row
  after the striking rows and the description moved down by its height.
- `exarust`: `--opponent-feral <f>` (debug duellist with feral stats).

## Out of scope

- The disabled body state's writers (the grab reads it).
- Clinch/claw holds (none exist in 0.9.5 melee).
- The socket row's drag and drop, the grip-side button, rating dots,
  coins and the material swatch on the live card.
- Live socket words of item copies (the card reads the record's).
