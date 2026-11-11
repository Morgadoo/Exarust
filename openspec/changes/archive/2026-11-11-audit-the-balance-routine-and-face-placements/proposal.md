## Why

Phase 3 (the character body) in docs/PLAN.md, TMotileDynamics. The component's rating left the balance routine (#18)
never audited block by block, the placements' facing (#58, #59, #62) kept as Exarust's yaw, the whole capture's
femur-flag exception (#83) open, and the armed parry's and stab drive's normalisations (#25, #32) open with their reach
unchecked.

## What the original does

Read in `exe.asm` and the research pack's decompiles:

- **The balance routine.** Its feet-centre pass, run every tick for a body with two legs or more, puts every leg's grip
  back at 1.0; the solver's iterations zero it on an anchored foot and raise it 0.001 at a time (to 1) on a touching
  one. With the balance under 0.2 (a double compare; a NaN is not under it) it only clears the step's stepping byte,
  kind and out-of-balance bit; the body tick then finds no step, skips its episode checks and never calls the step's
  end. The rest of the routine was compared block by block (docs/re/motiledyn-map.md, the block table).
- **The placements' facing.** The placement at a matrix, the init-time placement and the placement on the ground each
  take the matrix's forward flattened, normalise it with the engine's `RSQRTSS` normaliser as the heading, copy it to
  the stance facing, take the body's yaw as the engine's heading of it (its arc cosine), and write the raw forward as
  the cursor's smoothed and raw directions and the look.
- **The whole capture.** A part carrying the femur flag takes its stored clip turn instead of its measured one; the
  part classification sets that flag on the parent of each part with foot contacts, the thigh.
- **The stab drive and the armed parry.** The stab drive normalises its guide with `RSQRTSS`; the body tick calls it on
  attack clip 11 while the weapon's handling flag 8 is clear. The armed parry's two normalisations are `RSQRTSS` ones;
  the auto-parry and the swing tick call it.

## What Changes

- The balance routine puts every leg's grip at full each tick, and a step under the balance floor is cleared, not
  ended.
- `Locomotor::face` writes the placements' facing; the ground placement runs it, and the fighters' spawn runs it with
  `Locomotor::placed`'s twist and reach reset.
- The whole capture leaves the thighs at their last target.
- The stab drive's guide is normalised approximately (still not applied to a body). The armed parry was already.
