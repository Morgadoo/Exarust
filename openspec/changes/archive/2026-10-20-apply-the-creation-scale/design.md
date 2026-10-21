# Design

## Behaviour

The scale is the source value, converted to double, multiplied by the double `0.17`, added to the double `0.9` and converted back to a float. It is stored on the child and in the child's record.

With bit `0x100000` set on the object's flag word, a clear global guard byte runs the global update. When the child's label is null, one is built from the label base and the index plus one and stored. The label is then drawn with the scale.

## Model

`apply_creation_scale(source, special, global_flag, has_widget)` returns the scale, the store flag, the rebuild flag, the global call, the build and the draw. The finalizer always runs.

## Open questions

The global call, build and draw bodies are not in.

## Docs

This method is 307 bytes. `SG_Character_Create` moves from 21% to about 22%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
