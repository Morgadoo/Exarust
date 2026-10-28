# Design

- The converters stay in `exa_formats::objdb::upgrade`; `Store::parse` hands
  the body converter a fixed seed stream and `Store::parse_with_seeds` lets a
  caller pass the game's own source. Readers stay total: every old-section
  read past its end reads zero and every write outside the new body is
  dropped.
- The condition word conversion runs the existing `FractionWord` setters in
  the original's order on one word instead of computing fields from the old
  word, so the original's read-after-write (the damage read after wealth is
  written) is kept.
- The editor roll and the scene walks live in `exa_world` beside the
  existing `Draws` streams and `PlacedItem`; `PlacedItem` gains the placed
  object's tagged id.
- Loaded item values in play: the shipped databases are `0xDBAC0D09` and
  `0xDBAC0D08`, which these converters do not touch; the light share differs
  from the earlier single-precision value in the last bit for some
  conditions.
