# Design

## Behaviour

The female test is bit 0 of the `0x54` word. Female writes 16 to the four widget values at `0x154`, `0x1bc`, `0x224` and `0x28c`, and writes the default caption at `0x100315e38`. Male writes 0 to those four values, then reads the variant from bits 20 through 23 of the word, less one. A negative variant uses the default caption; otherwise the caption is the single byte `variant + 0x41`, built through `0x100008af0` and written to the widget at `0x208`.

## Model

`update_creation_look(female, variant)` returns the four values and an optional letter. `None` is the default caption. The letter is the byte `variant + 0x41`, so variant 2 gives C.

## Open questions

The string and widget routines are not in.

## Docs

This method is 334 bytes. `SG_Character_Create` moves from 4% to about 6% (about 5.6% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
