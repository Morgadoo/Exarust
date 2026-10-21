# Design

## Context

The damage routine runs in each character's post-step for the hit its
striker registered this tick. Exarust's hit pipeline (`detect_hits`,
`react_to_hits`) stands in for the post-step: one `Hit` per attack and
target. The rules live in `damage::resolve`; the striker's swing state in
`swing::SwingState`, which already had the recoil and follow-through as
pure functions. See [formats/OTHER.md](../../../docs/formats/OTHER.md)
(Damage, Weapon record).

## Goals / Non-Goals

**Goals:**

- Every number `resolve` returns equal to the routine's for the same
  inputs (order of operations, single and double precision, the
  epsilon tests, MAXSS against 0, the fast power).
- The routine's outputs the rest of the hit needs, and the striker's
  reaction after a wound wired into the hit reaction.
- The holding pass bit-exact.

**Non-Goals:**

- The clash and block paths, the bite link, sound playback, powers.

## Decisions

- **Inputs.** The routine reads the attacker's tracked slot speed (a
  stab twice its velocity along the weapon); Exarust keeps its blade's tip
  speed scaled to the struck point, in cm a tick, as before. The unarmed
  test is the attacker's weapon class, as the original's. The stab rules
  are the stab motion's unless the race is a quadruped (the race's second
  flag word, bit 8).
- **Victim.** A `Victim` carries the maximum health, the readiness, the
  steadfast rule bit, the Security perk and the potion flag; the Force
  Shell factor is a field set to 0.
- **Layers.** `resolve` takes the part's armour with its roll, or none for
  a blow that met a shield or a weapon: the original draws no roll then
  and meets no layer.
- **The fast power.** `fast_math` re-expresses the engine's fast
  logarithm and exponential from their listings; `resolve` raises crush
  and pierce with them, as the original does (so a zero impact's pierce is
  0 through ln 0 = −∞).
- **The striker.** `react_to_hits` collects each hit's striker reaction
  and applies it after the struck side, through
  `SwingState::after_wound` (the 0x724 bits, the window shut, the
  cleave-or-follow choice) or `SwingState::recoil_from`. The window shut
  stands for the weapon's lost striker bit.
- **The generator.** The hit reaction's own instance draws the coverage
  roll and, for a blow on flesh, the part sound cooldown's number, so the
  stream advances as the original's does per blow (Exarust has one
  generator per system, not the original's single global one).
- **The sag.** `Shove` carries the blow sag; the solver's tick writes it
  on the struck body with the shove, before the step that fades it.

## Open questions

- What sets the state flag 0x400 (a turned blow) and the attack-state bit
  0x40 (a refused wound): REA xrefs of their writers; neither is set in
  Exarust.
- The fists' hit point: the original measures it along the hand body's x
  (negated for one hand); Exarust's fist striker runs along its own
  segment across the hand, which gives the same fraction of W21.
- The hand body's own scale divides the unarmed W26; Exarust uses the
  character's body scale (inferred).
