# Design

## Behaviour

The refresh makes four calls in turn, with the child's model and record, the object's view and the child's model, and then two without arguments.

The flag test is the option word equal to `0x11`. When it matches, bit `0x10000` is ORed into the child's flag word; otherwise the bit is cleared with the `0xfffeffff` mask. The model refresh then runs with the child's model, the part search runs on the model, and its result is stored on the view object.

## Model

`refresh_creation_view()` returns the four call flags. `set_creation_flag(word, flag, part_found)` returns the updated word, the refresh flag and the stored part result.

## Open questions

The four refresh calls and the part search body are not in.

## Docs

These methods are 65 and 108 bytes. `SG_Character_Create` moves from 20% to about 21%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
