# Proposal

## Why

The parry check (trigger, counter, start, re-pick and end) and the
auto-parry deadline were ported as rules but nothing in the running game
called them, so no fighter ever parried and the hit path's parry branches
ran only in tests. A first wiring put the check in the character tick but
placed it wrongly against the original's callers: it skipped the parry
permission and the muscle-tone gate, read the strike window for the
original's continuation flag, let weapon holders parry with the unarmed
speed, never stored the countered motion and clip on every re-pick, ended
every parry into the stance clip rather than by the fighter's state, and
duplicated the swing's stall in a second stagger component.

Phase 4 (Combat and items), the combat-stage wiring started by
`reproduce-the-motile-combat-body`.

## What Changes

- The ticked fighter runs the parry rule in its attack stage after the
  swing routine, from the combat idle (muscle tone 0.85 or more, the
  auto-parry deadline passed, something in the hands) and then every tick
  it parries; a parry that starts takes its counter twice that tick.
- The check keeps the last-check time and stores the countered motion and
  clip on every re-pick; the counter follows the stored clip; a fighter not
  parrying lets its counter drift back to 0.
- A parry that ends returns to the Defence stance when the fighter is
  dying or dazed, else the combat idle, with a 40-frame smooth blend,
  unless the condition's clip stands still. A fighter whose hands hold
  nothing only drops the parrying flag.
- The trigger's window is skipped by the swing's continuation flag, not by
  its open strike windows.
- `Guard::Parry` is raised only while parrying or countering and never
  replaces a shield's guard; the hit path opens it when it ends the parry.
- The duplicate stagger component is removed (the swing's stall is the
  stagger); the Force wall's weapon interrupt stays wired in the debug
  barricade.
- A trace column pair for the parry.
- The two stale test-only `spheres` duplicates from the rebase are dropped.

## Out of scope

- The armed parry (speed and grip links from the weapon's handling, the
  shield's weight): a fighter holding a weapon does not parry yet.
- The explicit target (the nearest attacker stands in), the Fend entry, the
  player's request-an-attack clearing the permission, the race and claw
  forbids of the permission and the learned-skill word of the deadline.
- The previous-motion record of a continued swing (nothing sets the
  continuation flag yet).
- The body-side stagger helpers in `exa_physics::body::stagger`.

## Evidence

REA (Ghidra 12.1.4) on the original's body tick, its parry check and the
stance helper it ends into, read against their assembly: the check's
clear conditions and order, the unarmed decision, the re-pick and counter
rules, the 40-frame blend and fast ease, the round-to-even of the blend,
and the three call sites of the check in the body tick.
