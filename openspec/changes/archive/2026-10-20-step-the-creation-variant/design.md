# Design

## Behaviour

The variant is `(word >> 20) & 0xf`. A variant below 1 leaves everything alone and only the finalizer runs. Otherwise it falls by one and the word keeps every other bit while those four receive the new value.

When the widget at key `0x100` is present, its caption is chosen from the new variant less one: below 0 it is the default caption, otherwise it is the byte `(variant − 1) + 0x41`. The two refresh calls run whether or not the widget is present.

## Model

`step_creation_variant(word, has_widget)` returns the new word, whether it changed, the optional caption byte, whether the default caption was chosen, whether the refresh ran and the finalizer. Stepping an original variant of 2 gives a word variant of 1 and the caption A.

## Open questions

The widget and refresh routines are not in.

## Docs

This method is 249 bytes. `SG_Character_Create` moves from 7% to about 8%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
