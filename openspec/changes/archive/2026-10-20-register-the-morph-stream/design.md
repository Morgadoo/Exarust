# Design

## Behaviour

The cook registers the stream only when the controller's weight array exists and the owner's flags carry `0x200000`. It scans the owner's stream list for an entry whose low 16 flag bits are `0x1000`. An existing entry keeps its buffer; otherwise the list grows by one 0x18-byte entry, zeroed, with flags `0x04000001`, type `0x02a41000`, the vertex count, and a `V·32`-byte buffer. Either way the buffer is zeroed before the fill.

The target list starts empty. Each mesh triangle whose flags carry `0xC00000` contributes its three corners remapped through the owner's vertex map, in the corner order `{1, 2, 0}`, each only if not already listed.

## Model

`register_morph_stream(has_weights, owner_flag, existing, verts) -> Option<MorphStream>` reports the reuse or creation and the zeroed size. `collect_morph_targets(flagged, triangles, map)` returns the ordered unique targets.

## Open questions

The blob's layout and the owner's re-bind are out of scope.

## Docs

`RF_Morphing` is **43%** (7,353 of 17,099 bytes). The headline stays 14.4% and 5.7%.
