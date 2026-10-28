# Proposal

## Why

The fifth pass left held shields, second weapons, starting flips, the
torch's own links, the death weld change and the flip's section gate out of
the running game. This is the sixth pass on the motile part class
(docs/PLAN.md, the TMotilePart row of the engine-component phase), reviewed
against the executable's instructions.

## What Changes

- `exa_character::offhand`: the spawn's attach plan hangs one off-hand item
  as its own body: a shield (back pivot to its forearm, face pivots to the
  left hand, soft weld) or a second weapon on the grip table's off-hand
  entry with its own `Blade` in the second slot.
- `exa_physics::grip`: a torch carries its own two pivots, to the hand that
  holds it, and its soft weld; the soft weld's grip point is gated on the
  weapon class word 0x30; the flip command needs the section's flipped-side
  kind byte and a non-switching holder, and turns held slot 0 only.
- A dead holder's soft weld stiffens and the corpse keeps its items; a
  collapse drops the weapons and the torch and keeps the shield
  (`GripRelease`). A grip may wait for its holder's body (`PendingGrip`); the
  record's flipped hand flag starts the main weapon turned over.
- `exa_physics::body`: the limit-link law (squared slack, part-only move
  capped at 10 cm, 2 cm horizontal with the y gap zeroed) is a tested
  function. Character bodies have none: the registration gives every
  parented link the pivot flag, and the shipped skeleton stores the word 0.

## Evidence

The shield, torch and weapon attach routines, the shield link builder, the
flip command and step, the collapse and death routines, the living
iteration's link branches and the skeleton registration, read in the
disassembly (context/re/exe.asm) and the motilepart pack; the shipped
humanoid.rsq's link words.

## Out of scope

The shield guard in combat, the collapse's 10 cm slack (race word 0x800,
written to the item parent's link), the flipped record of a weapon that
starts turned over, the off-hand torch's own placement numbers, the limp
solver's limit law, the shield node's forearm placement (Exarust parents it
under the hand).
