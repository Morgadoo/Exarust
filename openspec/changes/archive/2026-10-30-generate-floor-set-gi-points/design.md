# Design

## Walk

The original's loader walks the children of each centre-floor variant
and tests the first name character against `#`, `+` and `[` only there;
the recursive walk it calls tests class and flags on every node and
recurses into every child without a name test. Exarust marks nodes in
one pass over the stored parent links. The generator runs on the node's
master (the instance chain's end), and the loader's instance copy then
hands the master's points to every instance, as the original copies the
count and buffer.

## Generation

The generator's store is unconditional, so a marked node's chunk would be
overwritten; no shipped marked node has one. Exarust's positions are
metres, so the bounds and triangles are scaled by 100, sampled at
parameter 20 (a 4 cm step) and scaled back; the recovered values are
within a float ulp of the original's centimetres.

The sphere query is a bounding-volume tree over the node's triangles
(both children kept per node; a leaf tests up to four triangles with the
plain predicate). The original reaches the node's collision triangles
through its kd-tree; only the cost differs.

## Cost

The shipped marked nodes are `gefs_uwa01`'s box (164 points, about
14 ms) and `gefs_uwb02`'s chute (964 points, about 130 ms) in a release
build, once per loaded set and cached in the model. The cell list is a
stable sort instead of repeated inserts.

## Body parts

The original also generates points for each class-`0x89` body part when
a character's physics body is made. Exarust's characters spawn no GI
points and nothing gathers them, so running it at load would only cost
time; it stays a tested function.
