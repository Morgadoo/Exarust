# Design

## Behaviour

The object clears 40 bytes at `0x198`, then sets the count at `0x188` to 2. It allocates two 16-byte entries at `0x190` and zeroes them. The first entry's first two words become `0x101` and `0x5FFF`; `0x100000` is ORed into the first, giving `0x100101`, which is copied to `0x198`. The second entry's first two words become `0x201` and `0x5FFF`, and its first word is copied to `0x19c`. The value at `0x1c0` is 4.0 and the word at `0x180` is `0x510000`.

## Model

`creation_defaults()` returns the count, the stride, the two entries, the two stored words, the value and the flags word. The entries are four words wide because each is 16 bytes; only the first two are written.

## Open questions

The allocation routine is not in.

## Docs

This method is 209 bytes. `SG_Character_Create` moves from 9% to about 10%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
