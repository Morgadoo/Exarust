# Design

## Behaviour

The core returns at once when the owner's data pointer is already set. It saves each part's weight (`w1 = w0`, `w0 = 0`), binds the controller to the owner, and allocates the blob. The blob is `12(P+1)(V+1)+12` bytes: the magic and size, the owner's base point, `V` rest points of 12 bytes each, the part count, then for each part its pivot (12 bytes) and `V` deltas (12 bytes each).

For each part the pivot is the candidate the owner's nearest search picks; for each vertex the triangle's three corners are the controller's joint table, remapped through the owner's map into the rest points. The triangle point is `Σ bary·corner` (the three corners taken as rows), the pivot is subtracted, and the result is rotated by the owner's rows.

## Model

`build_morph_data(base, rest, parts)` returns the `MorphData` with the magic and size; `morph_blob_size(parts, verts)` gives the size; `morph_delta(rotation, bary, corners, pivot)` gives one delta.

## Open questions

The blob's exact byte order beyond the documented fields, and the target bitmap and skin stream, are out of scope.

## Docs

`RF_Morphing` is **32%** (5,472 of 17,099 bytes). The headline stays 14.4% and 5.7%.
