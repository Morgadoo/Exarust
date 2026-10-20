# Design

## Behaviour

The variant is `(word >> 20) & 0xf`. Above 5 nothing is written and only the finalizer runs. Otherwise it rises by one, keeping every other bit of the word.

When the widget at key `0x100` is present, the caption comes from the new variant less one, which is never negative here, so it is the byte `(variant − 1) + 0x41` where `variant` is the new value. The two refresh calls run either way.

## Model

`advance_creation_variant(word, has_widget)` returns the same `CreationStep` shape as the down step. The stored maximum is 6; a word already at 6 is left alone.

## Open questions

The widget and refresh routines are not in.

## Docs

This method is 249 bytes. `SG_Character_Create` moves from 8% to about 9%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
