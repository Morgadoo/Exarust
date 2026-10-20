# Design

## Behaviour

The mode is the global object's `0x274`. When it is 2, the widget at key `0xcc` gets "Continue from save" and bit `0x40000000` is ORed into its `0x18` word. Any other mode gets "Continue from checkpoint" and the bit is cleared. The `0x10` bit of the `0x1c` word is set when the mode is at or above the threshold at `0x1004f0400`, and cleared below it.

## Model

`creation_continue_button(mode, threshold)` returns the label, the mirrored flag and the enabled flag. The two preparation calls and the lookup are out of scope.

## Open questions

The preparation calls are not in.

## Docs

This method is 153 bytes. `SG_Character_Create` moves from 11% to about 12%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
