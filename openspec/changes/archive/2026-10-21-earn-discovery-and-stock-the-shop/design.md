# Design

## Behaviour

Story experience is a state (`Experience`: the account experience and its
pool, the two conversion clocks, the item kinds seen, one bitmap per
level and the current one) and learners (`Learner`: experience, the six
skills with their techniques, combat skill, courage, a clock, powers and
their progress, the body state). The original keeps these on its
character records and in unit globals; Exarust passes them in.

- Item, kill and cell discoveries award `points` to the pool and
  `4 × points` to the player and every companion, in that order.
- Cells: the position times the float 1/600 is rounded ties-to-even and
  offset by half the grid (an unsigned halving). The cell must be inside
  `width × height`, have a sector table and a sector word of at least 1.
  The 5 × 5 window is clamped to `1 ..= size − 2` on both axes. A cell's
  bit is `stride × y + x / 32`, bit `x % 32`, with `stride = cell width / 32`
  rounded toward zero; the cell flags and the sector are read at
  `y × cell width + x`.
- Conversions use the original's integer widths: the clock difference is
  taken in 64 bits, divided as doubles and truncated through a 64-bit
  integer (out of range reads 0); counts are compared as 64-bit signed;
  sums wrap at 32 bits, so a clock ahead of `now` moves the account clock
  back by the negative count while the player's and a companion's stay.
- The trained skill gains `2n` as a 16-bit add; a new level snaps the word
  to `level × 5000`. The player's power progress and level are updated
  before its clock and skill.
- The fighting estimate keeps each float or double step as the original:
  the level term in doubles rounded to a float, the five weapon hundredths
  summed one float at a time in the order impact, slash, crush, pierce,
  thrust.

The shop works on a `Vec<u32>` shelf and a `ShopWorld` that gives the
object database's table (ids, section flags, `TradeRecord`s), an
instance's parent, and the copy factories.

- The two rolls come from the performance counter as the arena's other
  rolls do (`career::roll_from_counter`); the second seeds the variant
  roll the copies step.
- Entries counted as candidates but given no weight still count toward
  the draw's loop; past the last weighed one they read as nothing (the
  original reads past its array there, guess: harmless while the total is
  positive, since the draw breaks first).
- Weights are rounded to 64 bits and kept to 32; the duplicate division is
  a signed 64-bit division by 3; the halving is unsigned.
- The pick is `(total × roll) >> 32` in 64-bit signed arithmetic, kept to
  32 bits and compared signed.
- `find_entry` repeats the original's halving search on id differences.
- Coins pack as copper byte, silver byte, 16-bit gold; a sum under 1
  empties the dword.

## Open questions

- What the two model-section size words hold (guess: the icon's size) —
  answer with REA on the icon builder.
- Which of the trade's two tallies is spent and which earned —
  `Company::settle` maps the first to spent [guess]; answer with REA on the
  shop window that fills them.
- The body state bits a companion needs (mask 0xa equal to 2) — answer with
  REA on the motile state word's writers.
- The scene word the story presets set (1.8, 1.4, 2.0) — answer with REA
  on the scene environment.
- Whether the 15-character music comparison also compares stack bytes past
  the shorter name — REA on the short-string conversion.

## Docs

`docs/formats/OTHER.md` gains the trade fields of an object database
record. The crate READMEs of `exa_game`, `exa_formats` and `exa_audio`
name the new modules.
