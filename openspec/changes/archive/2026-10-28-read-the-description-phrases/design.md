# Design

Format page: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md),
section "Item description phrases (in `Exanima.exe`)".

## Decisions

- **Read, do not ship.** The phrase text is game data, so it is read from
  the user's executable at start (one 3.7 MB read, a few milliseconds), the
  same way packs are read. Without it the card keeps the bare description.
- **Anchors instead of addresses.** A table is located by the constant
  string of its first phrase and the aligned pointer to it in writable data.
  "in great condition" begins one table and is the third row of another; the
  reader takes the occurrence outside tables already found, reading them in
  the order flawless, perfect, high, great, made. The variant words are the
  run of eight pointers starting at "light" after a null entry.
- **Sentence choice.** A weapon page (weapon section not withheld) uses the
  weapon rule only on a factory model; else any apparel body (whatever its
  armour type) uses the apparel rule; else a shield section uses the weapon
  rule on a factory model, reading the shield body's bytes in the weapon
  places. This is the builder's register flow: the apparel pointer is
  cleared before the weapon page and set only when no weapon page was made.
- **Roll.** The card keeps a `u32` roll seeded from the clock at the first
  card, advanced only when a card is built.

## Open questions

- The meaning of each row's zero second word: unknown, all shipped rows are
  zero (corpus parse); the reader ignores it.
- Whether other callers (the dormant object editor) need the tables: they
  are editor-only and out of scope.
