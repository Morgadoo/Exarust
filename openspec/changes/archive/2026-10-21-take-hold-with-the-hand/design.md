# Design

## The grab in the solver

The original adds a world link to the clicked part's body: self point = the
hit in the part's frame, parent = the world anchor object, slack 0, the
anchor at the hit. Exarust keeps the hand's grabs in a list on `Ragdoll`
(`Grab`: part, weights, anchor, strength, last pass) rather than among the
skeleton's links, and runs them for a part right after its own links in a
limp body's iteration, which is where the original's appended link runs
(own links, then contacts, edges, children). The pass index is counted on
the body: reset by the integration step, raised at the end of each
iteration, so the original's `pass > limit` skip reads the same index from
0 in both the batched and the interleaved solver loops.

The law is computed in centimetres from the assembly: the gap with its
height zeroed, the squared length against the single-precision epsilon
(NaN continues), the unit gap by a reciprocal, the cap's `>` 5 and its
`cap < len` branch (NaN takes the full pull), the approaching share's dot in
the instructions' order (equal to the SIMD projection's because the unit's
height is zero), and the push spread by the point's weights unshared. A
part's mobility is `Part::inverse_mass`, which is the original's contact
mobility for both limbs (the per-bone value) and rigid bodies.

## The hand

`exa_game::hand` keeps the hand object's state as `Holding` (nothing, a
drag, the lift, a refusal) and the out-of-reach flag. Three systems run in
the frame after the camera and before the cursor: the press (the left
button just pressed: the pick, its name, the outcome), the hold (the drag's
per-frame handler and the releases), the reach (the request's reach
branches). The press and the drag read the previous frame's reach, as the
original's event handlers read the request's last result.

The rules are pure functions (`press`, `lets_go`, `drag_anchor`,
`pick::within_reach`) pinned by tests; the systems only gather inputs.

## The pick

The original ray-tests every object node's mesh after a padded bounding
sphere. Exarust has no per-node meshes for collision: the level and the
props' meshes are rapier colliders (the props' are their stand-ins, named
by `StandIns::owner_of`), and characters are only their bodies' collision
spheres. The pick takes the nearest of rapier's fixed hit and the spheres'
first hit within 40 m. A stand-in names its body and part directly, which
stands in for the original's walk from the hit node up to its rigid body.

## Open questions and how they were answered

- Event mask 0x4001 vs 0x2001/0x8001 (press, click, double click?): not
  traced; the earlier notes established 0x4001 as the left press, which is
  all this change needs. Recorded as open in the notes.
- Whether the reach's height is the world's or relative: the assembly
  compares the hit's own y (not the offset's) with 245, and the pick's
  fallback plane is y = 0, so the world's height (REA, assembly).
- The pass limit: `JG` on `pass > limit` with passes from 0 (the loop's
  start at −1 + 1), so limit 1 runs two passes (REA, assembly), correcting
  the notes' "only the first".
- The hip (the body's reference point the swing whoosh plays at): no
  direct writer found; taken as the pelvis part's
  position, labelled as inferred.
- The Ctrl drag's release not clearing the object hold: only two writers
  exist (the press sets, the lift's release clears), so the quirk is kept.

The page this change extends is `docs/PLAYER.md` (The reach, The hand); no
format page changes.
