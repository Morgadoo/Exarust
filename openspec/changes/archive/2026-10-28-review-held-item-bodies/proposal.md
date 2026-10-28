# Proposal

## Why

The motile part class still drew a held item as a mesh fixed to a hand bone,
with no body of its own. The original gives a held item its own rigid body.
Grip links tie that body to the hand, the second hand and the arm, and the
weapon-body branch of the part's impact callback answers its contacts. The
fourth pass on the component made held items solver bodies. A review then
checked each rule against the executable's instructions.

## What Changes

- `exa_physics::grip_solve`: a cross-body solver stage pulls a held item's
  body to its holder with the limp link law. The stage shares that law with
  the body's own links. It builds the grip links once both bodies exist,
  gives the item the held group mask, the held bit and friction 0.5, and
  frees the item (friction 0.75) when its holder loses its drive.
- `exa_physics::grip`: the builder's raw arm point on the hand side, the
  link init's 0.001 smoothing, the world welds' anchors, the conditional
  relax mark of the mode setter, the stored requested mode (the callback's
  half-sword turn stays local), the combat-ready flip gate with the engine's
  sine and cosine, and the empty-hand words.
- `exa_physics`: the second flag word on a lower leg's children when the
  leg joins the world. The shape builder's initial mass word (the reciprocal
  of the mobility).
- `exa_combat::impact`: the weapon-body branch for a striking held item. It
  covers the hard impact (interrupt, stab and overhead strength with a zero
  stall), the sound gated on the item's own cooldown, the cooldown stamp and
  the contact kind. `exa_combat::grip`: the held reach and aim height, worked
  in double.
- `exa_character` and `exa_game::torch`: an equipped weapon or a torch whose
  model has a rigid root gets a body and a `HeldGrip`.

## Capabilities

### Modified Capabilities

- `motile-part`: held item bodies, grip links, the weapon-body branch, the
  held reach.

## Impact

- `exa_physics`, `exa_combat`, `exa_character`, `exa_game`, `exa_audio`, and
  their READMEs.
- Evidence: REA disassembly of the grip builder, the world-weld builder, the
  mode setter and its two-hander callback, the relax pass, the flip step,
  the held-item refresh, the empty-hand reset, the part's impact callback
  and the link init. All of it was read as instructions (double multiplies,
  the callback's local mode, the unsigned cooldown compare).
- Out of scope: the soft weld's gates and the 0x30 hands case of its
  height, enabling the welds on death, the mode and flip callers, shield
  bodies, multi-node held bodies, factory weapons' bodies, the camera kick
  and the base collision sound. Also out of scope: the shared tick field
  added to a stab's stall end, and the branch against a limb. Exarust's
  blade sweep already answers a weapon striking a body.

## Phase

Phase 3, bodies: skeleton, physics, control, with Phase 4's held weapons
(TMotilePart lane, fourth pass).
