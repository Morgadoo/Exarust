# Design

## Behaviour

The requested frame is a field of the object. Below 1 the method ends. The child holds the current frame in its record; when it already equals the requested frame less one, the method ends without a refresh.

Otherwise the child's frame becomes the requested less one. The model refresh runs with the child's model, the part search runs on the model, and its result is stored on the view object.

## Model

`select_creation_frame(requested, current, part_found)` returns whether the frame changed, the resulting frame, whether the part was found and whether the refresh ran. The part result is supplied because the search is shipped separately.

## Open questions

The refresh body is not in.

## Docs

This method is 128 bytes. `SG_Character_Create` moves from 12% to about 13%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
