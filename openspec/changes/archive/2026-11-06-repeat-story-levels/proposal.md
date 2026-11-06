## Why

Round 84 playerctl lane: two runs of the same story level did not agree
(exanima01's NPCs from tick 1–10), so no lane could prove parity there.

## Evidence

Bisected with traces, `--sim-digest` (step counts of held items, body
parts and props at step 1) and logs:
- the animators played the load's varying frames into the bodies' spawn
  poses (fixed by holding game time behind the loading screen, as the
  original's load holds its clock; `--hold-loading` already did it);
- under load the stage timeout started the game with characters
  undressed and their gear not yet held (a `--fixed-frames` run now
  waits);
- limb trees built on worker threads installed on varying ticks (the load
  now waits for them);
- a body built before its dressing in one run and after it in another
  (behind the loading screen a body now waits for its dressing);
- equal waits for the pass and planner turns went to whichever brain the
  query visited first (now the earlier spawned).
No random generator is involved: placed characters' rolls start from a
fixed value.

REA: the AI's method slot 25 (death hook: a follower's activity 0 and the
party removal; a golem body's glow material faded), called from the body's
death routine.

## What Changes

- exarust: `hold_while_loading` always on; `stage_waits`; the dressing
  stage also waits for unbuilt bodies and limb trees.
- exa_game: `build_bodies` waits for dressing behind the loading screen.
- exa_ai: the turn tie-break; `brain::death_hook`.

## Out of scope

- The golem glow fade at death: Exarust draws no live skin material for it.
