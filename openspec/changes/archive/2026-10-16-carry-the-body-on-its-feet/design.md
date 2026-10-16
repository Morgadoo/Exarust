## Context

The original's body tick resets the carry to gravity when the muscles are
on, adds the locomotion push while moving, and ends its feet routine with
the target pelvis height and the lift servo; the carry is copied from the
pelvis to every part, and each limb step adds it after inertia. The
research notes, with addresses, are git-ignored:
`context/re/body-drive.md` (§0, §4–5, §9 and the new §14),
`body-feet.md` (§7, §14), `body-tick-order.md` (§8.2) and
`body-controller-plan.md` (C8, the answers to Q1–Q3).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the original's carry, lift servo and target pelvis height, in its cm and
  ticks;
- the capsule carrying only x, z and yaw; the tone gravity and the
  vertical hold gone;
- a get-up that rises without a hop or ringing.

**Non-Goals:**
- the frame anchor (C9), support pins (C14), walking by the carry (C18);
- foot contacts (C12), leg IK (C13), walking feet (C19): stairs.

## Decisions

- **Where the lift runs.** The original computes the target and the lift
  after sampling, in the feet routine; Exarust's character tick measures
  the balance and the ground first (as C6 placed it), then samples, then
  aims. The lift runs at the end of `aim`/`hold`, so it reads this tick's
  targets, this tick's ground and support. The grounded test reads each
  leg's ground offset, which the lift sets: Exarust's is a tick old (the
  original's sampler sets it before the feet routine). Standing it is the
  sole height either way.
- **The clip root over the ankle.** The original subtracts the foot
  contact's character-space height (its ankle under the clip pose) from the
  clip root's: the clip root's height over the leg's animated ankle. Exarust's targets are the body root's world transform times the
  clip pose and the root only turns about the vertical, so the pelvis
  target's height less the ankle's (the shin target times the ankle point)
  is the same number.
- **Moving.** The original's moving ground offset is the ankle's height
  over the lowest ankle of the first key of its walk (blended with the run
  for its other gaits), plus the sole. Exarust's character tick computes
  that lowest ankle from the moveset's Walk clip while the body moves
  (`Bearing::gait_floor`); the jog and sprint blends are not used. The
  moving target is neither capped nor raised, as read; the walking blend
  weights and swing flags are left for C19, so a moving body blends the
  other foot in by half like a standing one.
- **The other foot's test.** As read in the asm, the spread is the other
  foot's contact height less its ground plus the lowest foot's contact
  height (world heights), not a difference between the feet. It is ported
  as read; on high floors it blends nothing.
- **No ground.** The original's −100000 cm target gives a negative f; the
  port keeps the value (the trace shows it) and the same result.
- **Support shares.** The constructor gives every part 1.0, the muscle
  setup gives the part with the foot contact (the lower leg) 0.5 and its
  foot 0.25 (asm), so the notes' "pelvis and legs 1" is corrected to
  pelvis and thighs 1, lower legs 0.5.
- **The capsule's carry.** The root's motion since the last tick, with the
  rise dropped (the root turns about the vertical, so heights are kept).
  The C7 c-damper reads the carried-frame velocity; with the rise gone its
  vertical is the world's.
- **The hold.** Exarust's own until C14: kept across the ground only (pull
  and brake have no vertical). With no vertical hold a get-up rises on the
  lift: the target follows the get-up clip's root, and the end of `Get Up
  2` overshoots 3–5 cm once and settles within 45–100 ticks.
- **Perched capsules.** `Bearing::carried` stays: a body whose capsule
  stands where its feet do not (perched, spawned over the floor) keeps its
  balance while it drops onto its feet, since nothing places a spawned body
  on its feet yet.
- **Tests' standing body.** The tests' humanoid has no foot parts; its
  soles are set to 3.3 cm so the lift's balance point (1.7 cm over the
  target, which is its legs' metre plus the sole plus 5 cm) is where its
  legs are straight.

## Open questions

- Whether the original's walking body climbs stairs on the swing flags
  (the lowest foot lifted hands the target to the other) and the probe
  ahead: C19, by REA on the feet routine's swing flag and the gait bump.
- Which gait flags select the jog and sprint gait floors: REA on their
  writers, with C18.

## Risks / Trade-offs

- Stairs regress: the capsule steps up first and nothing lifts a walking
  foot onto a step, so it catches the risers (exanima01: climbed one or two
  steps, then fell). A coupling of the moving target to the capsule's floor
  climbed them in a test run but re-couples the body's height to the
  capsule (which floats 2–5 cm over the arena floor), so it is not taken.
- After a get-up the feet bear weight and stay where the clip left them,
  so the body may stand leaning (balance 0.97 after `--shove 60`) until
  recovery steps and clip root travel (C10, C15).
- Blows no longer sag a standing body (C7's sway came from its tone
  gravity); the original's sway comes with knockback steps and the strike
  model (C16, C21).
