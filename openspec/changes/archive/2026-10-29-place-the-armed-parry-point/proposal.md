# Proposal

## Why

The motile body (TMotileDynamics) has been ported a piece at a time with no
routine-by-routine account of what is left, so each lane re-discovered the
gaps. The armed parry's geometry (where the parry takes the attacker's
blade) was missing, its grip strengths took float products where the
original takes double ones, its blend rounded half away from zero where
the original's conversion rounds half to even, and the AI's return to a
stance by energy compared in float where the original compares in double.

## What Changes

- `docs/re/motiledyn-map.md`: every routine of the class mapped to the
  Exarust items that reproduce it, with status, share, liveness and byte
  totals (done / partial / missing / not needed), in Exarust's own names.
- `exa_combat::parry`: `parry_blade_height`, `blade_point`, `parry_point`
  / `ParryPoint`, `parry_distance` — tested, not called; `armed_grip`
  takes its strengths in double, passes a NaN distance through the
  falloff and rounds its blend half to even.
- `exa_physics::engine_math::normalize_approx` (the approximate
  normalisation with the zero guard).
- `exa_combat::stance::stance_by_energy`, used by the AI's guard choice.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the armed parry's point, its grip arithmetic, the stance by
  energy.

## Impact

- `exa_combat`, `exa_physics`, `exa_ai`, their READMEs, `docs/INDEX.md`.
- Evidence: the armed parry routine and its maths helpers (barycentrics,
  4×3 product, subtraction, projection removal, normalisations, length)
  read as instructions; the parry start's blend conversion; the stance by
  energy routine.
- Live change: only the AI guard choice at its boundaries (an energy of
  exactly 0.95 under a most of 1 now guards, a yellow of exactly 0.01 is
  dying). The armed grip is not called by the game.
- Out of scope: wiring the armed parry (grip links, the refusal path).

## Phase

Phase 4, combat and items (TMotileDynamics audit lane; the class spans
Phase 3 bodies).
