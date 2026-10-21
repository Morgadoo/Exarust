# Design

Docs: [`docs/UI.md`](../../../docs/UI.md) (Retail screens and popups),
[`docs/RENDERING.md`](../../../docs/RENDERING.md) (Item icons).

## Shape

Every module is data in, plain record out: no Bevy types, no drawing. A
form's constructor, hit test, key and release handlers return what the
original would store or call, so a later painter only has to follow them.
Existing code is reused rather than copied: `pickers` for the base pick,
`col_swatch` for the swatch popup, `icon_cache` and `possession` for items,
`activity` for the actor setter, `save_package` for the package constants.

## Rules checked against the instructions

- Float sums follow the instruction order; compares the original makes in
  double are made in double; conversions round half to even.
- Picker keys hold the point with an ordered clamp, so an unordered value
  stays unordered (`picker_fields`).
- The physique wait compares `last + 2 > clock` in 64 bits
  (`exa_game::create::sync_physique`).
- The save families carry their prefix, id word, count and package magic as
  traced; the older `SAVE_MANAGERS` table is not touched.
- The script role's commit walks the 12-byte entry headers (`12 + count * 4`),
  rewrites in place when the count matches, and otherwise rebuilds from the
  last mismatched entry. Its tail lands `stored * 4` bytes too far and a
  shrinking commit with nothing after it copies the old entry past the new
  allocation; both are kept and reported (`Commit::Rebuilt { overrun, gap }`).

## Faithful quirks kept

- A tooltip shown while fading out reverses the fade.
- The picker's move key answers handled for every code.
- A draft invention was dropped wherever the assembly shows no such rule
  (a hash in the role blob, an inverted auto-commit, a bounds check, a
  second setter for the actor); roles not traced keep neutral names.
