# Design

## Context

The original runs the combat state as the AI's state routine every AI
tick while it has a foe or a focus. It reads its body's condition, clock,
attack flags, weapon class, energy, scale, size, encumbrance, reach and
step; it writes the body's step goal, step state, step length, queue and
look direction, and its own eagerness, press, overshoot, heading and blow
request. The routine calls into the per-contact position score and into
navigation queries for each place.

## Goals / Non-Goals

**Goals:** every rule of the routine in order, as pure functions over the
body's view (`Body`, `Foe`, `Contact`, `Bystander`), with what the rest of
the game answers through a `Field`; the duellist driven by it.

**Non-Goals:** the position score itself, the navigation queries, the
chase route, the follow check, powers, the body's step physics.

## Decisions

1. **Pure state, explicit inputs.** `CombatMind` holds the AI fields the
   routine owns; `Body` and `Foe` are what it reads; `Tick` is what it
   decides. The caller owns the body and applies the outcome.
2. **Precision.** Products and sums keep the instructions' order; where
   the original widens to double (the estimate chance, the eagerness drift,
   the clock thresholds 0.3, 0.4, 0.6, 0.68, 0.78, 0.9, the 0.8 energy
   gate, the scale tests 1.3 and 1.4, the re-aim angle) Exarust does too.
   The angle wrap compares in double against π and steps by the f32 full
   turn; a vector is made a unit with the approximate reciprocal square
   root; the yaw uses the engine's quick arccosine.
3. **Dead stores keep their draws.** The routine works out a first place
   count and a jitter it then overwrites; their generator draws are kept so
   the later draws line up.
4. **Uninitialised read.** The keep-off builds a test vector whose x comes
   from z and whose second component is unset stack; Exarust reads 0 there
   and labels it.
5. **The duellist.** It runs one combat tick per body tick (at most four a
   frame), with one contact (its target at the engage level). The position
   score is `exa_ai::place` with the bearing, distance and facing term the
   original's score adds. A chosen step becomes a walk to the step's goal
   in the combat walk until it arrives (10 cm) or 1 s passes [Exarust's];
   a queued step starts when it ends. Without a record's personality the
   skill is 0 and the blend 1 [Exarust's default]. The position and the
   aim point are the chests (the original's pelvis height is not traced).

## Open Questions

- The per-contact ground cost, line walk and sight queries in the running
  game: answered when the navigation grid is live (the Field defaults
  stand in: cost 1, no clamp, in sight).
- The meaning of the run mode the foe estimate and the charge read (the
  body's flag the original clears on each motion start).
- Whether the routine's stale global place table (the long steps read
  place 0's distance from the previous tick when the search starts at 1)
  matters: Exarust reads this tick's.

## Docs

`crates/exa_ai/README.md`; research notes in `context/re/npc-combat-state.md`.
