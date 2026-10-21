## Context

The combat branch of the behaviour tick reads the right button: held (or a
deflected stick), the behaviour's walk mode is the cursor walk, chevrons
for the cursor (two beyond 300 cm); the request then keeps only its sprint
latch and its move bit, sets run for a cursor beyond 300 cm or turns the
sprint latch into the run, takes the aim direction as heading and look,
and, unless a foot steps, casts the look-ahead `6·v² + 45` cm along the
frame's forward axis: a hit stops the free walk and, within 100 cm, the run
and the sprint. The release drops the mode on the next tick.

The aim cursor: a stick whose `x² + z²` (single, compared in double) is over
0.01 places the point `250·m + 50` cm out on the aim plane; the screen
cursor eases toward its projection by `(0.4·AimSpeed + 0.1)²` (double, then
single) from where the pointer was, and the pointer is picked as the mouse
is.

The controls' frame: a blocking screen runs the blocked turn (ten seconds
past the stamp, then 0.0001 rad a millisecond) and returns; two other
blocks return; the active view not the game's restamps and returns; then
the idle test, in which a dialogue or the power bar is busy.

## Goals / Non-Goals

**Goals:** the combat walk, the pad aim ease and the hand's mechanism hold
in the running game without changing the existing feel elsewhere; the
idle camera's gates; the behaviour's and the controls' rules pinned by
tests from the instructions.

**Non-Goals:** windows, item cursor, door/wheel/cloth bodies, the reach's
room tests.

## Decisions

- The pad's ease runs on the world point (the screen ease's factor) in the
  fixed behaviour tick, so it does not depend on the frame rate; the
  original's projection path differs slightly but meets the same point.
- The combat walk reuses the exploration walk's look-ahead (its 45 cm
  combat base) and run cut, and sets the cursor mode so the chevrons show.
- Duplicates of existing Exarust code were removed in favour of the
  existing code (steering's cursor follower, `exa_combat::stance`,
  `pick::within_reach`, `exa_ai::roles`, `exa_world::teleporters`).
- The entering-combat cancel keeps letting go of a drag: the original's
  link would stay with nothing to release it in Exarust.

## Risks / Trade-offs

- [The combat walk changes how the right button feels in combat mode] →
  it is the original's; nothing else in combat changes while the button is
  up.
- [The pad aim lags the stick] → by the original's AimSpeed ease.
