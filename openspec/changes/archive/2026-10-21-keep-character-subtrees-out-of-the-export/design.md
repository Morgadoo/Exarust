# Design

Format page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md), Writing
chunk trees.

## Character subtrees

The original's collection routine files a character and returns at once.
Only the prop and scene-node branches reach the loop over the node's
children. `Walk::visit` now returns after pushing a character, so a
character's child nodes are not touched: their flags (stale exported bit
included) and numbers stay as given. That matters because another node's
parent or data-source test reads exactly those values.

## Tracks without keys

The keyframe record stores the last key's index and then (last + 1)
16-byte keys. With no keys the index is −1 and no key bytes follow.
`write_track` writes that instead of refusing.
