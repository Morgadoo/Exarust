# Proposal

## Why

This is Phase 4 (combat) of docs/PLAN.md, with the golem guardians of
`exanima06` (Phase 6) as the driving case. Factory weapons (the `@` items:
the golems' halberds and every assembled sword, axe and polearm) were fixed
to their hand's bone. They passed through what they struck, and their
strikes, like every other held weapon's, were found by Exarust's blade
sweep: a segment tested against generous spheres around the target's parts.
The polearm lane (round 44) left this as the remaining cause of golem stabs
landing on the haft.

The original builds a rigid body for every factory weapon it assembles and
grips it to the hand like any other held item. A held weapon wounds where
its body touches a limb: the pair stage's hit event is classified on the
striking side, and a wound is the classifier's kind 1.

Evidence (REA, Ghidra listing of `Exanima.exe`):
- The factory weapon builder: unless its caller asks for no body, it makes a
  body for the merged node with the factory-weapon shape builder and the one
  fit record (resolution 1.1, even axes, 6 cells, no free corners, tag 2, no
  side growth), registers it on the node, and gives it the factory's hit sound
  set by the weapon section's sound byte (`WH_SwordA` without a section).
- Its five callers: the hand's item-instance builder and the world item
  loader pass "build a body". The inventory icon renderer and the colour
  preview pass "no body". The streaming loader's choice follows a request bit.
- The hand attach (owner, the class bits 0x20001 over the chain, the grip
  links to the hand bodies) has no branch on the node being a factory weapon.
- The striking part's classification of a hit event (context/re/body-pairs.md
  §11.3, master-combat.md §5): the blade side, kind 3 block, kind 2 clash,
  kind 1 wound on a living limb (refused while a hit is pending, on an
  engaged arm, or on the shielded hand). A later kind 2 or 3 replaces a
  pending kind 1, and a kind 1 never replaces anything. There is no speed gate.

GPU runs: `exanima06` with the golems woken, and `exanima01 --opponent`
(sword), against the round-44 binary.

## What Changes

- `exa_character::item_bodies`: an item's bodies and sound sets. An item
  with no stored rigid node that is a factory weapon gets
  `assembled_item_body` (already ported and tested, now attached) and the
  factory's hit set (`exa_formats::weapon_stats::impact_sound`). The main
  hand, the off hand and the GUI drop use it. The equip logs the fitted body
  at debug.
- `exa_combat::block::strike_held`: a held weapon body's hit event on
  another fighter's muscled limb is classified (`part::classify`) and becomes
  a `Hit` with `Struck::Limb` at the contact point. It is dealt once per
  target per swing, refused while a clash or block waits, and never replaces
  a clash or block found in the same tick.
- `exa_combat::detect_hits`: the blade sweep skips held weapons that are
  their own body. Fists and held models without a body keep it.
- Docs: the `exa_character` README and the stab drive's comment.

## Out of scope

- Factory weapons that a level places stay static models. The original's
  world loader gives them a body, but Exarust's placed-object spawner
  (exa_world) has no access to the builder.
- Fists still strike by the sweep. In the original, a hand part is the
  striker.
- The engaged-arm bit (class bit 8 on the arms in a clinch or a two-weapon
  attack) is not set on Exarust's limbs. Only the shield hand is guarded.
- The part-tree mass redistribution is still not read by the solver. This
  applies to every held weapon.
- The golems' standoff: they attack from 188–230 cm, and their halberd's
  point carries past the player, so the first touch after the window opens
  is often the haft in either model.
