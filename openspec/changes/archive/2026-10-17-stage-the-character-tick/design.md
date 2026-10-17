## Context

The original runs one sector step per 83 Hz tick: sim objects against
nearby bodies, every character's body tick (the character list from its
last entry down to 0, the player at 0), then the solver. Each body tick
runs its parts in a fixed order (upkeep, readiness and the daze block,
chest velocities and anchor, behaviour, posture, attack, movement,
sampler, carry and feet). The research notes, with addresses, are
git-ignored: `context/re/body-tick-order.md` §1 and §8,
`master-body.md` §10, `master-movement.md` §2, `master-animation.md` §3.4,
`master-combat.md` §16, `master-plan.md` §3 (the tick contract), A0 and
the Critic's K.4.

Exarust has one function for every living body's character tick and one
for the solver. The master plan's lanes all add work to both. This change
names the stages so each lane adds a module to its stage.

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body, how a
tick runs).

## Goals / Non-Goals

**Goals:**
- the fixed step's sets in the original's order: pre-step, character
  ticks, solver, post-step;
- a character tick of nine named stages, run per character in the
  original's order, each character's whole tick before the next;
- solver stages at each point of the original's sector step where later
  changes add work;
- `exa_audio` in modules with one shared sound-set picker;
- no behaviour change.

**Non-Goals:**
- moving the systems that already run ahead of the character ticks
  (stance, falls, attack stamp, player input) or once a frame (AI) into
  their stages;
- any new rule (pairs, sim objects, world links, mechanisms, post-step
  hits).

## Decisions

- **Character-major order through a schedule run per character.** The
  original finishes one character's body tick before the next starts, so
  a character's behaviour reads the whole of an earlier character's tick
  and only the last tick of a later one. A Bevy set per stage would run
  each stage over every character first. Instead the character tick is
  its own schedule (`CharacterTick`), its stages chained sets, single
  threaded; the driver (`exa_game::stages`) runs it once per living
  character with the character in a resource (`TickedCharacter`: its body
  root and the controller above it). Stage systems are ordinary systems
  that read that resource; they must not rely on change detection, which
  spans the other characters' runs. A lane adds a system to its stage
  without touching the driver or another stage.
- **Which bodies tick.** Every body with muscles whose clips its ticks
  advance (`TickedClips`), the player's last, the others in query order;
  nothing before the motion set is loaded. These are the bodies the old
  loop ticked.
- **Exarust's work by stage.** Posture: what the body does this tick
  (`Bearing`). Movement: the gait floor while moving, the controller's
  step left out of the carry, the balance and the ground under the feet.
  Sampler: the capture, the clock and the sample, kept together because
  the clock and sampler share `limp::clip_tick` (the original advances the
  clock at the end of its movement stage, just before the sampler).
  Carry: the targets on the frame, the lift, the controller's follow. The
  sampled pose passes from the sampler to the carry in a resource
  (`SampledPose`), written for every character before the carry reads it.
  Each step keeps the order of reads and writes the old loop had (the
  bearing's fields and the carry's root are disjoint, so setting the
  bearing before the controller's step is the same).
- **Balance before the sampler.** The original measures balance, feet and
  support in its carry stage, after the sampler, so its pose hold reads
  last tick's support; Exarust measures them in the movement stage, before
  its sampler, as the old loop did. Kept to change nothing; the lanes that
  rewrite the feet (C12) and the clocks (M14) decide when to move it.
- **Solver stages as registered hooks.** `SolverStage` has one method per
  stage, each a no-op by default: `integrated`, `pair_pass(i)` at the
  start of iteration i = 1…20 (the original's numbering), `links(i)` after
  every body's own links, `iterated` (where the pair list is dropped,
  before the level contacts), `sim_objects`, `detect` (body–body),
  `sim_object_contacts`, `friction` at the start of each of the 8 passes,
  `post_step`. `SolverStages` keeps them in the order added and finds one
  by type; `step_bodies` passes them to `solve_with`. `solve` is
  `solve_with` with none, so every test of a single body is unchanged. A
  stage receives the awake bodies being solved; C17 extends `detect` with
  the sleepers it may wake. Link kinds inside a body (world links, sliders)
  stay in `Link`; the `links` stage is for links between bodies.
- **Post-step.** `BodyPostStep` follows `BodySystems` for per-character
  work after the solver (the pending hit and its damage). The debug
  readers (trace, joint trace, scripted pause) keep running after the
  solver.
- **Audio.** `SoundSet { prefix, count }` names variants as the original
  does (prefix plus 01…count); `SoundBank::pick_from` is the one picker.
  The picks are drawn in the same order as before, so the same variants
  play.

## Open reverse-engineering questions

| question | how it was answered |
|---|---|
| Does the character list run from its last entry to the first, the player last? | `exe.asm` of the sector step: count − 1 down to 0, each entry's registered-in-a-sector flag tested, its body tick called |
| 20 iterations, the pair pass first in each? | `exe.asm`: counter 1…20, the pair pass called, then every group root's link iteration with the counter |
| Detection after the iterations, body–body before body–level? | `exe.asm`: pair list dropped, per-body repair and bucketing, sim objects stepped, body–body contact generation in the 3×3 cells, sim-object passes, body–level detection |
| 8 friction passes, pairs first? | `exe.asm`: counter 0…7, pair friction then world friction for every body with friction above 1.19e-7 |
| The body tick's stage order? | REA call targets of the body tick in address order: burning, Break Tangle, the daze capture, the behaviour slot, waking, the swing routine, the focus tick, the chest point and locomotion, the physical capture and sampler, balance, the feet routine |

## Risks / Trade-offs

- Running a schedule per character costs a schedule run per living
  character per tick: 0.33 µs with four empty stage systems (measured, 30
  characters × 1 000 ticks in the `play` profile), so 30 characters at
  83 Hz cost under a millisecond a second.
- Stage systems see one character at a time; a lane that needs all
  characters at once (an AI manager) adds a system to the fixed step's
  sets instead.
