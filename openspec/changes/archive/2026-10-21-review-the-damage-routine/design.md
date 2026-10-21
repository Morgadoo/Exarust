# Design

## Context

Review fixes to the damage lane; see
[formats/OTHER.md](../../../docs/formats/OTHER.md) (Damage) and the
striker's recoil rules there.

## Decisions

- **Rounding**: the end of a recoil or a follow-through is the scaled
  float converted with the default rounding, so `round_ties_even`; ties
  are rare, the common change is a fraction at or above one half adding a
  millisecond.
- **Stab without a number**: the listing compares 2 against the energy
  and takes the root branch on "greater" or "unordered"; Exarust writes
  that as `T < 2 || T.is_nan()`.
- **Turned blow's sag**: the armour pass writes the sag before the turn's
  check, so the hit reaction sends a shove with no velocity and no daze,
  carrying the sag (a parry's shove does the same).

## Open questions

None new; the reviewed changes' open questions stand.
