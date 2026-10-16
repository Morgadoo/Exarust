## Context

`bleed-from-wounds` queues a bleeding blow's wound and paints it seven
ticks later; `stain-weapons-with-blood` adds the blade's blood at the blow.
In the original the same blood branch first adds a jet to the scene's one
blood-spray particle system: a source record on the shared emitter, fixed
to the struck body's matrix, whose life is how many droplets it still
sends. The emitter is the engine's generic particle system (the one fires
use, `docs/RENDERING.md` Fires) driven by a built-in template whose flags
pick other paths than the fires' (no fire ageing, the spawn's velocity
blend, the camera's forward axis for the quads, per-slot sizes, colour
keys without the parity offset), drawn with the built-in `_BloodPrt02`
material in the multiply blend.

Exarust's fighters are entities whose body parts carry `BodyPart` (bone)
and, while simulated, are parts of the body root's `Ragdoll`, ticked in the
83 Hz fixed step and drawn between their last two ticks.

This change extends [`docs/RENDERING.md`](../../../docs/RENDERING.md)
(Blood sprays; Fires holds the shared particle rules) and
[`docs/formats/RFI.md`](../../../docs/formats/RFI.md) (the droplet atlas's
flags). Notes with
addresses are git-ignored (`context/re/wounds-and-blood.md` §2.1 and §11,
`combat-look-plan-final.md` L9, `fire-particles.md`).

## Goals / Non-Goals

**Goals:** a jet per bleeding blow with the original's direction, droplet
count, frame and part link; the droplets' emission, spawn, step, life and
drawing exactly as the template and the particle routines give them; the
droplet material; flags to shoot a jet with and without a fight.

**Non-Goals:** floor blood, creatures' own blood, blood across saves or
respawns, distance fog, the 8-bit clamp before the multiply.

## Decisions

- **The jet is a message at the blow.** `react_to_hits` sends `Spray` in
  the bleeding branch before the wound is queued (the original's order),
  whatever the queue does; `exa_character::spray` turns it into a jet the
  same frame. The direction is the negated striker velocity of the hit
  (`Hit::velocity`, the tip's velocity: the original negates its smoothed
  tip velocity), normalised by the jet.
- **One resource, ticked in the fixed step.** `BloodSprays` holds the jets
  and droplets of the whole scene, like the original's single emitter, and
  ticks in `FixedUpdate` after the body solver, so its clock is the body
  tick and a scripted pause or a single step freezes it with the bodies.
  The part frames come from the `Ragdoll`'s last tick (`Part::drawn(1.0)`,
  scale dropped: the original's body matrix is rigid); a body without one
  (a character view body not simulated, a race without a body) falls back
  to the part's drawn transform.
- **Jets live in the part's frame.** The jet frame (second axis along the
  jet, first = axis × +Z, third = first × axis, as the original builds it)
  is stored relative to the part; each emission re-reads where the part
  is, as the original multiplies the stored frame by the body's matrix.
  The jet's motion between two emissions feeds 0.15 into the new droplet.
- **The seed is copied, quirk and all.** The original stores the first
  "previous droplet" velocity as twice the jet's axis after turning it into
  the body's frame, then blends world velocities with it; Exarust stores
  the axis in the struck part's frame likewise. The part frames are the
  skeleton's own, so the lean of the first droplets matches as far as the
  frames do; it halves with each droplet.
- **Pool.** The original's ring holds 512 + 1 slots (its quality global is
  a constant 5, above the 4 that would shrink it) and would overrun past
  them; Exarust drops a droplet beyond 512 live, which needs more than
  eight full jets at once.
- **Drawing.** One mesh in world space rebuilt each frame with the
  droplets' diamonds for the scene camera (forward axis = the original's
  negated camera row), α = the fixed step's overstep fraction as the bodies
  use; spawned with the first droplets and hidden while none fly, so a
  quiet scene draws nothing. `SprayMaterial` is a Bevy material with the
  multiply blend and an opaque fragment `mix(1, texel, alpha)` (the
  original's blend mode 4 branch of `shadeopt_f`), culling off as for the
  flames, no prepass, no shadows.
- **Random numbers.** The original's integer generator (seed × 0x08088405
  + 1, n × seed >> 32) for the gate, the looks and the velocities; slot
  sizes from its uniform. Streams are not meant to match the original's.
- **Debug helpers.** `--debug-spray` writes the same `Spray` message from
  the debug wound targets (the player, the character view's body) every
  second of game time, at the part front the debug wounds use
  (`PartFronts`, shared). `--pause-after-jet n` is a fourth scripted pause
  in `sim_control`: the tick that first sees `BloodSprays::started` above 0
  plus n.

## Open reverse-engineering questions

- How long a jet lasts. *Answered (REA):* the source's life is decremented
  only when it spawns, in the emission loop; round(0.3 × strength) is a
  droplet count (36·R, at most 60), so a jet runs 0.9·R s at 40 a second,
  not 36·R ticks as the plan read it.
- Which way a jet points. *Answered (`exe.asm`):* the blood branch passes
  the negated smoothed tip velocity of the striker (the strike window's
  tracker), so droplets fly against the swing; the plan's "along the
  swing" is corrected in the notes.
- The flags the fires do not use. *Answered (REA):* `0x80` blends each new
  velocity with the source's previous one and the seed (0.15 of the
  source's motion, lerp ½, 0.8/1.2 placement) and, in the quad builder,
  takes the camera's forward axis as the view; `0x2000` gives each ring
  slot a size 24·(0.5 + 0.5u) at insertion; `0x200` alone colours without
  the parity offset; no `0x40000`, so droplets lose one life a tick.
- The material's output. *Answered (bytes, shader source):* blend word
  `0x0004` (ZERO, SRC_COLOR) sets blend mode 4, whose unlit TexEmi branch
  writes `mix(1, MatEmi·EmiTex, vertex alpha × fog)`; MatEmi is (1, 1, 1)
  at the default exposure 1.
- The texture's sampling. *Answered (bytes, file):* the built-in payload's
  texture load flags are `0xF0` (trilinear, mipmapped, LOD bias on) and
  `BldPrt02`'s file flags `0x20A80000`; neither has a wrap bit, so both
  axes clamp to the edge. The splats keep off the cells' edges, so the
  clamp hardly shows.
- What the droplets look like in the retail game. *Open:* retail request
  RR35 (rapid shots during a bleeding hit) was not taken.
