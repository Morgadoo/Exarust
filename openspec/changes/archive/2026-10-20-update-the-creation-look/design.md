# Design

## Behaviour

The female test is bit 0 of the appearance word. Female writes 16 to the four slider widgets' values and writes the default caption. Male writes 0 to those four values, then reads the variant from bits 20 through 23 of the word, less one. A negative variant uses the default caption; otherwise the caption is the single byte `variant + 0x41`, built as a one-character string and written to the caption widget.

## Model

`update_creation_look(female, variant)` returns the four values and an optional letter. `None` is the default caption. The letter is the byte `variant + 0x41`, so variant 2 gives C.

## Open questions

The string and widget routines are not in.

## Docs

This method is 334 bytes. `SG_Character_Create` moves from 4% to about 6% (about 5.6% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
