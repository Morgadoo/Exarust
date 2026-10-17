## Context

The original checks a fall in one routine, called by the body tick's
posture block at the end of its falls branch (every sub-path: the Defence
or Rebalance stance of an upright body, the stagger, Fall Forward or
Backward), so for every living body off its balance and still moving. It
keeps a state (0 to 4) and a time stamp on the character. A second call
site, in the collapse branch, runs it for a body dead at that point and
sinking more than 6 cm a tick; a body dead when its tick starts returns
before that branch, and nothing else makes a body dead in between, so
that call never runs. The kill plane is in the plain integration step:
only a character's dead body is integrated by it. The research notes,
with addresses, are git-ignored: `context/re/master-body.md` §9 (the
rules), `body-feet.md` §0 and §6 (the chest's velocities, the character
ground), `body-q8-q9.md` §5 (the muscles flag), `body-tick-order.md` §3
(the kill plane) and `m44-notes.md` (§24, this change's reading).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body: a new
Falling to death part after The collapse); the fade is in
[`docs/UI.md`](../../../docs/UI.md).

## Goals / Non-Goals

**Goals:**
- the fall check and its timelines as the original's, a pure state machine
  pinned to its numbers (−5 cm a tick, 2,500 cm, 84, 334 and 251 ticks of
  12 ms);
- the inputs it reads measured as the original measures them: the chest's
  slow smoothed vertical velocity, the pelvis height, the ground under the
  body without support, each foot's ground;
- the player's pit sound and fade, death and removal from the world; the
  kill plane for dead characters.

**Non-Goals:**
- the pause and the menu at the fade's end; world-audio fade; the camera's
  tilt nudge; voice sets and their hold-off; possession; impact damage
  (C22); RR18's capture.

## Decisions

- **Where the check runs.** In the posture stage, after `track_condition`,
  for a living body whose branch this tick is the falls branch. The branch
  is kept on the fighter (`PostureBranch`) by `track_condition`, which
  already computes it, instead of recomputing the decision (the condition
  has changed by then). The system lives in `exa_game`, which knows the
  player's body, plays sounds and starts the fade; the rules live in
  `exa_combat::fall`.
- **What it reads.** The chest's slow velocity, the legs' ground and the
  pelvis are last tick's measure, as the original's posture block reads
  the feet stage of the tick before. The ground under the body: the
  original's feet stage sets it from the feet with support and from a
  Height Query under the pelvis without; a supported body can never pass
  the per-foot test (a grounded foot is within 30 cm of its ground), so
  `Balance::ground` is the query without support and none with it. The
  get-up support test, which made the same query only while getting up,
  now reads it.
- **Out of reach.** The original keeps a foot's last ground while the
  ground is out of reach; Exarust's leg has none then, and the check
  counts it as not deep enough. A fall over nothing has its −10,000 cm
  miss within reach, so the two agree where it matters.
- **The clock.** The game clock (`BodyTicks::millis`, 12 ms a tick), with
  the original's strict comparisons: the pit on the 84th tick after the
  loss (1,008 ms), the player's death on the 334th, anyone else's on the
  251st. The timeline moves only while the body stays in the falls branch,
  as the original's.
- **Death and removal.** The original's death writes 0xA00 and takes the
  node out of the scene in the same call, items freed with it; Exarust
  writes dead, empties yellow and red, and despawns the character's root
  (the body model's parent, holding its controller) with everything under
  it. Death by falling and by the kill plane leave the original's muscles
  flag on, but the body is gone (the fall) or was already limp (the kill
  plane only meets bodies whose collapse took their muscles), so no corpse
  ever keeps the living link rules; Exarust never makes these bodies limp.
- **The kill plane.** Exarust's body flags its root below −50 m for every
  body; the game takes it only for a dead character (the original's plain
  step is only theirs), once (`FallDeath::out`, the original's body flag,
  also set when lost).
- **The scream.** Voice kind 2 is the voice set's `_GroanH` slot. Exarust
  has no voice sets yet: a random `vhm01`–`vhm09` heavy groan, as the hit
  sounds pick a random grunt, by the character's voice volume and pitch
  (its stats), at the chest.
- **The fade.** The original's session hook starts it at the pit: alpha
  round(t × 255), t the share of 3 s of game time gone, drawn before the
  HUD's bars. Exarust's `DeathFade` runs on virtual time (paused with the
  simulation), at Z 895 between the heartbeat's border and the bars, black
  remapped by `linear_opacity` for the original's gamma-space blend.

## Risks / Trade-offs

- [The player's removal leaves the camera and the HUD without a player] →
  both already skip a missing player; the screen is black by then.
- [Exarust's fallen living body sinks faster than the original's (about
  14 cm a tick against 8.6)] → the scream and the loss come at about the
  same time (the threshold is crossed early); recorded for the physics
  lane.

## Open questions

- RR18: the exanima02 chasm in the retail game (state, chest velocity and
  character ground per tick) — a read-only capture when the user runs it.
- What the pause at the fade's end opens (a death menu) — with the game
  modes (Phase 7).
