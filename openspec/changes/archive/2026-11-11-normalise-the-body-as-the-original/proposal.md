## Why

Phase 3 (the character body) in docs/PLAN.md, TMotileDynamics. The component's rating kept three gaps this change
settles against the assembly: the "approximate normalisations" (credited to the leg IK and the locomotion routine), the
locomotion routine's keyless walk start (its clock "inferred 0"), and the full tone reset's call (no caller, its reach
untraced). The whole capture's half lean was also listed missing.

## What the original does

Read in `exe.asm` and the research pack's decompiles:

- **Normalisations.** The engine's in-place normaliser that scales by `RSQRTSS` of `(x² + z²) + y²` (a zero vector
  stays zero) is called by the turn toward a fall, the get-up (its direction, then the facing it writes), the armed
  parry, the stab arm drive, the dying tick (the wandering facing), the femur posture (its axis), the body tick (the
  twist's first normalisation), the three placements (the facing), the pose-only and living samplers (the lumbar's half
  turn and the hunch's rebuilt basis, three rows each), the whole and plain captures (the half lean's three rows) and
  the daze pose (its random axis). The standing and locomoting leg IK and the locomotion routine call only the exact
  normaliser (`1/√` times the vector). The captures then take the engine's quaternion of the matrix and its four-lane
  normaliser (`(y² + w²) + (x² + z²)`, exact reciprocal root); the femur posture and the daze pose take the engine's
  quaternion of an axis and angle; the twist's followed yaw gives its direction by the engine's sine and cosine; a turn
  toward a fall and a get-up keep the engine's heading of the facing they write as the yaw.
- **The keyless walk start.** With the heading more than 1 rad on the yaw's positive side the locomotion routine sets
  no start key: the field keeps the jump-in key the playing clip left (the start routine sets 0, the samplers set none,
  −1, once the clip's bracket passes that key, a queued switch sets its key), and the clock is read from the walk's key
  times at that key less one. Key 0 reads the word before the times and none the word 8 bytes before; their
  allocation was not traced. Exarust takes both as 0: as the halves of a 64-bit length or size word below 2²³ they
  read as 0 and a denormal, nothing once added to the clock.
  The walk is set as the clip without the start routine, so with none its sampler never leads in from the capture: it
  enters from its own last key. A resumed start keeps the field the same way.
- **The full tone reset.** Its one call is the body tick's upkeep on a tick whose muscles-on byte is 0. The base
  constructor sets the byte, the revive sets it again; the dying tick clears it and at once calls the death routine,
  which writes the condition 0xA00 and makes the tick return before the upkeep; the character generator's corpse is
  written 0xA00 and muscles off together; a 0xA00 body returns at its tick's start. No living body reaches the call.
- **The whole capture** half-leans the twist-flagged parts under the head exactly as the plain capture does.

## What changes

- `engine_math::normalize_quat` (the four-lane normaliser).
- The twist (`body_twist.rs`): its first normalisation approximate, its followed direction by `direction_from_yaw`; the
  midline half turn (`square_half`, shared by `twist_spine` and `half_lean`) with approximate normalisations; the half
  lean's quaternion by `mat3_to_quaternion` and `normalize_quat`.
- The hunch (`body_hunch.rs`), the femur posture (`body_bend.rs`), the fall and get-up headings and their yaw
  (`exa_game balance.rs`), the dying heading (`exa_combat collapse.rs`) and the daze pose (`exa_anim clock.rs`, which
  now depends on `exa_physics` for its engine maths) as read.
- The walk start (`exa_game stages/movement.rs start_walk`) keeps the playing clip's jump-in key for a keyless or
  resumed start, or none (no lead-in).
- `Lean::capture` half-leans the whole capture too.
- `docs/re/motiledyn-map.md` corrected (#14, #15, #21, #31 and the rest).
