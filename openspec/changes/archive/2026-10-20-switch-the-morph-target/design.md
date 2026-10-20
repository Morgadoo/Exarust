# Design

## Behaviour

The target at `0x138` carries a word at `0x1c0`. Only the tag `0xedc0c589` proceeds; the method returns otherwise. On a match the point at `+8` and the target's points are copied, the point is stored at `0x64` and its third word at `0x6c`, and `0x10005d160` refreshes the target.

## Model

`switch_morph_target(valid, point)` returns whether the switch ran, the copied point and the refresh flag. The copy and refresh themselves are out of scope.

## Open questions

The copy and refresh bodies are not in.

## Docs

This method is 122 bytes. `RF_Morphing` moves from 2% to about 3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
