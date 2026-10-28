# Proposal

## Why

The fourth pass made held items solver bodies but left their runtime out:
no caller ran the grip modes, the flip or the shield links, the world welds
were built without the original's gate, and the camera kick and the base
collision sound had no consumer. This is the fifth pass on the motile part
class (docs/PLAN.md, the TMotilePart row of the engine-component phase),
reviewed against the executable's instructions.

## What Changes

- `exa_character::grip_runtime`: each tick the holder's clip picks the grip
  mode and runs the grip callback, the post-change relax pass runs, and an
  active flip steps, rebuilding the `Blade` record from the flipped section
  when it lands. A `FlipWeaponRequest` names the character; F sends it after
  a successful light toggle, as the original starts the flip in the same
  branch.
- `exa_physics::grip`: the flip's middle and end tests are double compares
  on the single-precision progress, the step's scale and offset double; the
  soft world weld is the parry weld, built when the holder may parry, at the
  grip for a polearm; the death stiffening skips index 0; a shield item takes
  the shield builder.
- `exa_character`: every stored rigid node of a held item is a part, the top
  one moving the item's root entity; the factory weapon's fitted body is a
  tested function, not attached.
- `exa_physics::part::base_impact_sound` and `exa_combat::impact`: a
  non-striking held item's contact rings the base collision sound, drawing
  its cooldown spread only when it rings; a hard overhead by a big attacker
  sends a `CameraHit` that kicks the chase camera; the stall adds the 12 ms
  simulation step.

## Evidence

The hotkey handler, the flip step, the world-weld builder, the death
routine, the part impact callback and the base body's impact sound, read
in the disassembly (context/re/exe.asm) and the motilepart pack.

## Out of scope

Held shields in the equip path, second weapons and starting flips, the
fitted factory body in the game, the go-limp weld slack, the torch's own
soft weld, the parry routines that use the soft weld, the original's flip
gate on the section's flipped-side kind (Exarust keeps its light test).
