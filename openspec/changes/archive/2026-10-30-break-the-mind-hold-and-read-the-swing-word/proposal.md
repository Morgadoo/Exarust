# Proposal

## Why

TCombatant next pass (combatant4, round 41; docs/PLAN.md Phase 6 — AI).
The perception skipped the original's mind-power break, the duellist
published the foe's arc from a stun guess and a phase threshold instead
of the swing word, the position score carried a boolean for the contact's
engagement level, an invented four-way footwork reduction sat beside the
combat state's real place search, the step clear on entering combat used
a speed of 0, and the engine's cheap arc sine was implemented twice (one
copy sending NaN down the wrong branch). Round 40 made the duellist's foe
the fighter under the target root; the blade and skill lookups still read
the root.

Evidence: the original's perception (its mind check after the familiarity
factor), the mind unit's hold-break routine (bind compared on the source,
charm on the target, state set to 2, first match), the attitude wrapper
(same argument order: the observer's powers, the observed character),
the arc publisher (the attack word's bit 4 is the recovery, 0x30000 the
commitment), the position score (level 3 gates the strike and approach;
the off-level clear doubles the close term and leaves the danger), the
queued-step clear (8 × the body's speed along its heading + 20 cm) and the
engine's arc sine (an unordered compare against zero, so NaN takes the
non-negative branch), read in the objdump listing and the pack.

## What Changes

- `exa_ai::combatant`: `break_mind_power` and `Effect::state`; the
  perception breaks the first binding sourced on, or charm aimed at, the
  observed character when its concern is over 1.5 and the raw attitude
  plus 0.1 is under the effective one. `asin_fast` is now
  `exa_physics`'s `asin_approx`.
- `exa_ai::place`: `Spot::level` replaces the fight flag; the
  `choose_footwork`/`Footwork`/`still_closing` reduction is removed.
- `exa_ai` duellist: the foe's published blow reads its swing word
  (cleaving = recovery, continuation bits = committed); its own bearing
  from the measured origin feeds the thrust's arc test; the entering-combat
  step clear passes the body's speed; the foe's blades, reach and skill are
  read from its fighter.
- `exa_ai::brain`: the trust toward third parties is taken before the
  perception may break a power.

## Out of scope

The running game has no mind-effect component, so the brain passes an
empty power list and the break is exercised by tests only. The hit's
mind-power call, the head tracker's caller, the movement word's stepping
bit and the pathfinder's move-point slot stay unported.
