# Design

The node kind is computed from the decoded item rather than read off a
built node, because Exarust's database builder does not keep the
original's node record. `node_kind` follows `scene_tree`'s order (model,
volume section, operative section) and reuses `tree_kind` for model trees,
so the two cannot drift. A volume section shorter than its 20-byte body is
not decoded into the item and so reads as no volume (labelled as a guess;
no shipped record has one).

The stream's kind constants (`OBJECT_*`) stay in `exa_assets`, which cannot
depend on `exa_world`; a test pins them equal to `db_objects::KIND_*`.

The character section keeps one reader. `ChunkCursor` stays as the tested
counterpart of the reader's push and pop; it errors on a chunk whose size
runs past the bytes where the original trusts the size.
