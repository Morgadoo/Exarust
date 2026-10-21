# Design

Format pages: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(Damage, Parry, Regeneration) and [`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md).

## Review method

Every rule of the lane was checked against the instructions rather than the
pseudo-code. The decompiler reorders float sums, turns MINSS/MAXSS into
ordered compares (the instruction returns its second operand on NaN), hides
sign-bit compares and drops index registers; the review kept the
instruction order, the double intermediates with one narrowing, and
round-half-to-even conversions.

## Corrections

- Attack reach: size × ((1 − age × 0.3) × base) in double, one narrowing.
- Part hit sound: an armour layer is skipped only when `coverage < 0.5`
  (NaN proceeds).
- Voice request and wake: double sums, unwrapped 64-bit clock sums.
- Part state: the mark routine's separate bit is kept next to the window and
  the chain bit. The chain bit's meaning in the original is shaky (the
  contact routine tests it with the window; the base body class sets it); it
  is kept as the lane modelled it.

## Not wired

The rules have no callers in the running game except the part-material hit
sound (`exa_audio::hits`), the wound tally (`exa_combat::fighter`) and the
strike window (`exa_physics::body`). The game's `Hit` carries no physical
contact-impact scalar, so the impact sound's conditional random draw cannot
replace the unconditional per-blow draw safely. Behaviour of the running game
is unchanged, so no arena comparison was needed.
