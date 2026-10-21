# Design

## Behaviour

The owner's morph target carries a tag word. Only the tag `0xedc0c589` proceeds; the method returns otherwise. On a match the blob's base point and the target's points are copied, the point and its third word are stored on the owner, and the owner refresh runs.

## Model

`switch_morph_target(valid, point)` returns whether the switch ran, the copied point and the refresh flag. The copy and refresh themselves are out of scope.

## Open questions

The copy and refresh bodies are not in.

## Docs

This method is 122 bytes. `RF_Morphing` moves from 2% to about 3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
