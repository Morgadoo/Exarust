# Design

## Behaviour

The band is the child's `0x54` masked with `0xff00`. A non-zero band stores the index at `0x1a4` and at the child's `0xa4`, then ends without a list or a draw.

With a zero band the list is built. A count at its first word that is not positive ends the rebuild. Otherwise the index is stored, an old second list is released, the new list is stored, the sub-routine runs with the index, widget 9 is captioned with the byte `index + 0x41`, and the list is drawn unless the busy flag is set.

## Model

`rebuild_creation_second_list(index, banded, old_exists, new_count, has_caption_widget, drawing_allowed)` returns the band path, the build result, the index, the replacement, the stored flag, the caption and the draw. The finalizer always runs.

## Open questions

The constructor, sub-routine and draw are not in.

## Docs

This method is 414 bytes. `SG_Character_Create` moves from 16% to about 17%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
