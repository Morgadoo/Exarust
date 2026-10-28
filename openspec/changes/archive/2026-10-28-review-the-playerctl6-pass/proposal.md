# Proposal

## Why

docs/PLAN.md, Phase 3 (bodies: skeleton, physics, control), the player's
controls and behaviour: the playerctl6 lane (TPlayerBehaviour,
SG_PlayerControls unit routines) wired the aim point's bone lock and raised
surface, the combat auto-target, a 50 cm combat aim gate and a
deep-crouch footwork step, and added the movement goal's direction, step
vector, tapped stride and 400 cm² release. Its review checked each rule
against the executable's instructions and corrected three.

## What Changes

- Combat aim: the point under the cursor becomes the combat aim only once
  it is 50 cm or more from the player across the ground (was 20 cm).
- Auto-target: each combat tick the behaviour picks the visible opponent
  within 500 cm with the best `(500 − d)·(cursor·dir/d + 0.5)` and stores
  it on the player body; the aim point locks onto its bones.
- Aim point: with a stored target the cursor's height locks onto the
  target's bones; without a lock the point the cursor's ray meets on the
  level is taken when above the plane's point and under the ground plus
  200 cm.
- Deep crouch: past a crouch of a half the footwork's step is 0.7 of the
  stored reach in double precision, narrowed, then the legs' extension
  (the review moved the extension after the 0.7).
- Movement step vector: the keys sum with the full side vector (the review
  removed a wrong 0.8 share) and the height is dropped for keys and pad.
- Movement goal: its direction is flattened and its tapped stride takes
  the legs' extension after the double product (review corrections).

## Evidence

The behaviour tick's step assignment (ordered compare against 0.5, the
double 0.7 product, then the extension multiply), the word's step-vector
routine (no side scaling, height zeroed before the length), the goal
routine (0.8 side, height zeroed, double tap stride, extension, 400 cm²
squared distance summed x, y then z), the aim routine's 50 cm gate on the
flattened length, and the aim point and auto-target routines, read from
the disassembly.

## Out of scope

The behaviour tick's mode word and its 1.5 step for a counted tap, the
goal's navigation write and its target-approach bend for a lone forward
word, the original's own three aim points (three part origins stand in),
the auto-target's can-see callback (a fixed-level ray stands in), door
and wheel bodies, level placement markers and the GUI windows.
