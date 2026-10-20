# Design

## Behaviour

Each append increments the global count at `0x4f17b0`, grows the vector at `0x4f17c0` with twenty-byte entries and writes the new entry at `(count − 1) × 20`. The first store is the id; the value appender writes one word at `+4`, and the word appender copies sixteen bytes at `+4`.

## Model

`ArenaLog { records }` with `ArenaRecord { id, words: [i32; 4] }`. `append_value` writes `[value, 0, 0, 0]`; `append_words` writes the four words. Both return the slot index.

## Open questions

The readers are not in.

## Docs

These methods are 113 and 127 bytes. `XA_Arena` stays about 34%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
