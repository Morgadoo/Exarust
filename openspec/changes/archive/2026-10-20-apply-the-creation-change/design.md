# Design

## Behaviour

When bit `0x40` of the object's flag word is clear, the method jumps to the finalizer and nothing else runs. Otherwise the child's record word gains bit 0 when the option word equals 2, and loses it otherwise. The appearance label is then taken through the label routine.

With the label known, the method sets the child's text, refreshes the view when the view object is present, runs the global update when the global guard byte is clear, sets the busy flag, runs the two rebuilds, clears the flag, runs the callback slot, and finally runs the finalizer.

## Model

`apply_creation_change` returns the updated word, the label and a boolean for each call. The word keeps its other bits. The finalizer always runs, including on a disabled change.

## Open questions

What the two passes and the finalizer do is not established.

## Docs

This method is 218 bytes. `SG_Character_Create` moves from 3% to about 4%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
