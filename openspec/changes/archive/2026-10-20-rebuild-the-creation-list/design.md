# Design

## Behaviour

The list is built from the object's `0x1b0` and the index plus one. When the constructor's first word is not positive, the rebuild ends after the finalizer.

Otherwise the index is stored. An existing list is released and the new list takes its place. The child's record word receives the index. When the widget at key 7 exists, its caption is the byte `index + 0x41`. The new list is drawn unless the busy flag is set.

## Model

`rebuild_creation_list(index, old_exists, new_count, has_caption_widget, drawing_allowed)` returns whether the list was built, the index, whether an old list was replaced, whether the new list was stored, the caption byte and whether the draw ran. The finalizer always runs.

## Open questions

The list constructor, release and draw are not in.

## Docs

This method is 355 bytes. `SG_Character_Create` moves from 14% to about 16%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
