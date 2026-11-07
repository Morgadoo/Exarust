## Why

PLAN Phase 5 (NPC combat). Rounds 101–108 gave every race a body and let the
demon be summoned, but two non-human fighters never closed on their foes:
the harvester held at exanima67's rock with a seven-point chase route
planned, and the demon's footwork mostly stayed. Round 108 blamed the
harvester's body (its recovery step); the trace shows the body's step flag
clear (`body_step` in `chase-walk`) and the AI's driver switched off again
every tick.

## Evidence

- Trace (`EXA_DUEL_TRACE`, exanima67 at the harvester's rock, main build):
  14 351 chase-walk ticks, `driving false` on every one, the body's step
  episode off; the fighter never walks (condition word 0x8 throughout).
- Exarust's idle walk soft-stopped any NPC whose driver was driving while
  combat held the body, once a tick, after the combat chase had started the
  driver. The original's AI ticks its current state only (the combat entry
  makes the combat state current, the idle return makes the idle state
  current again, as `lifecycle` ports them), so the idle state's leg is not
  ticked while the combat state walks its chase.
- REA (Exanima.exe), the path follower read from the instructions: a walk
  of urgency 1 or more (the chase walks at 1) soft-stops when its ground
  point is within 100 cm (3-D squared distance under 10 000) of the path's
  last point, and the soft stop clears the path, so the walk no longer
  follows. Exarust cleared the chase route at 50 cm across the ground and
  never cleared the chase's follow state, so a chase that ended (or whose
  re-plan was not followed) kept ending every combat tick at the chase.

## What Changes

- exa_ai: the idle walk soft-stops a driving body only when its own leg was
  under way; the chase walk arrives by the path follower's hurried rule
  (`path::hurried_arrival`) and its follow state ends on arrival, when the
  route is empty, and when the fighter leaves combat.
- exa_ai: `EXA_DUEL_TRACE` also logs the blow, the executor's readiness,
  energy, the clip's loop, the attack and the reach (`duel-trace`) and the
  body's step (`chase-walk`).

## Out of scope

The demon its shaper summons in exanima67 starts 9 to 14 swings at 1.8 to
2.1 m against the standing player (the place score's ideal is its weapon's
220 cm reach) and none touches anything (no hit, no block). The same demon
spawned in exanima07 or summoned in the arena lands blows from the same
distances. Not traced further: the held weapon's pair contacts in that
scene.
