## Why

Exarust's player writes its capsule's velocity straight from the keys:
W walks at once and stops at once, the body turns by one law (240°/s with
a floor, ×0.9 with the keys) whatever it does, a standing quarter turn is
216°/s and allowed once a second, Shift always runs, and a wall seen by a
ray from the hip zeroes the velocity. The duellist snaps its yaw to its
target and sets its own velocity.

In the original every character's behaviour writes one locomotion request
(heading, look, a flag word, moving, free walk, a goal point), and the body
tick decides from it each tick: free walk or toward a goal, the gait's stop
distance and the brake, the gates that let a body walk (full energy, steady,
not stepping), the stop within 25 cm at 0.5 cm a tick, the body's turn by a
table of rates, and the alignment of the pelvis with the heading. The
player's behaviour holds a released walk until its key segment is 0 or 4,
stops softly, turns a quarter only more than a second after any turn key,
and spends Shift's sprint latch on a jog with the keys while a far cursor
keeps it.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing man";
the master plan's lane D change M1 (milestone MS2).

## What Changes

- The locomotion request and the body's locomotion state as components on
  the body model root (`exa_physics::body::locomotion`: `Locomotion`,
  `Locomotor`), with the original's flag word.
- The movement decision in the character tick's movement stage, before the
  gait floor and balance: the move vector, leg mode and brake by the stop
  distances 50/200/250/50 cm (blended by the run), the gates (cooldown,
  energy, stepping, balance 0.9 moving / 0.99 standing, airborne 80,
  Rebalance 20), the brake below 1 cm a tick, the move or stop branch.
- The body turn by the original's table (free walk (1 − 0.1·v)·B, the slow
  turn 0.5·B, the attack's law from M22's swing, otherwise (1 − 0.6·enc)·
  1.25·B/√scale; ×0.6 crouched, eased over 0.6 rad, ×0.9 moving by the keys,
  the step ÷√scale), held while helpless; the alignment exposed for
  later consumers (M16, C18, M8); the cooldown after a stumble.
- The player's exploration behaviour (`exa_game::steering`): the request
  (W, RMB over the view, S with brake and turn-about), the hold until the
  walk's key segment 0 or 4, the cursor's direction from the plane 80 cm
  over the ground and its second-order follower, the look (30 % toward
  the smoothed cursor walking by the keys, eased standing), the standing
  quarter turn after more than 1 s of the game clock, Shift's latch, the
  reversal guard, the look-ahead 75 + 6v² cm (−70 backwards) that ends the
  free walk at a wall and cuts the run within 100 cm, the soft stop
  8v + 20 cm ahead; in combat mode no slow turn and a soft stop.
- Until the body walks by its carry (C18), the capsule executes the
  decision at today's speeds (1.4 m/s walk to 3.6 m/s run, 1.22 m/s
  backpedal), braking by the original's brake drive and enough to stop on
  its goal; its speed along the heading is the capsule's. The look-ahead's
  line walk is still a ray from the hip, clamped a capsule radius and a
  30 cm cell short of what it meets (M5 walks the grid).
- The duellist writes the request (heading to its target, a goal 1.5 m
  toward it while farther than 1.15 m, its capsule at 1.2 m/s); its body
  turns at the table's rate instead of snapping, the attack law while it
  attacks.
- Debug: `--trace` gains the locomotion columns (flag word, moving, free
  walk, yaw, heading, turn rate, alignment, speed, reach, stop reason);
  `--press` takes `Shift`.
- Docs: PLAYER.md (Movement), formats/RSQ.md (Movement decision, trace
  columns), DEBUGGING.md, PLAN.md, crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and `context/re/exe.asm`, re-read for this
  change: the player behaviour tick, the exploration steering, the soft
  stop, the per-frame movement request, the cursor smoother and its yaw
  follower, the movement decision, the can-locomote gates, the body turn
  and the alignment, the locomotion's reads of the sprint latch, the
  sampler's key segment, the writers of the quarter-turn block; constants
  read from the PE (`context/re/master-movement.md` §3–§5, §21).
- RB23 and RB24 settled in the binary (§21); the retail game was not
  running, so RR2/RR3/RR4 were not taken.
- Corpus: the installed Walk, Run and Backpedal clips are keyed every 10
  frames from 10 to 80 (the release segments).
- Exarust in game through `gpu_run.sh` (light runs, traces).

Out of scope:
- the locomotion drive through the carry, the run blend's alignment and
  slope terms, the gait columns' thrust and clip rate (C18, M16);
- the grid line walk and the look-ahead on the navigation grid (M5);
- combat facing, combat RMB walk and crouch (M2); combat footwork steps
  (C16; the interim capsule footwork stays as the request's stepping);
- NPC path following and the AI driver (M7); the gamepad (M13);
- the head look's sampler (M8), which will read the request's look;
- the balance gates against the body's balance: Exarust's standing balance
  can settle under 0.99 with no recovery step to restore it (C15), so the
  stage reads a steady body until then (the gates are implemented and
  tested);
- the Rebalance countdown (none kept yet) and the turn lock (no writer).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: tank controls, walk to the cursor, combat footwork's
  turn and stop; the locomotion request and movement decision, the body
  turn, the capsule as executor, the duellist through the request.
- `physical-body`: the body trace's locomotion columns; the movement
  stage's decision.
- `world-viewer`: `--press Shift`.

## Impact

- `exa_physics` (new `body::locomotion`; `Balance::land`, `Drive::land`,
  `Ragdoll::pelvis_forward`): a new module and two small edits in lane A's
  files
- `exa_game` (new `steering`; `player`: the behaviour writes the request,
  `PlayerBody` loses yaw/heading/run; `stages::movement`: the decision;
  `stages::posture`, `stages::sampler`, `limp::bearing`, `balance`: read
  the locomotion components; `trace`: columns)
- `exa_ai` (the duellist through the request)
- `exarust` (spawns with `Locomotion::facing`, `--press Shift`)
- `docs/PLAYER.md`, `docs/formats/RSQ.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, crate READMEs
