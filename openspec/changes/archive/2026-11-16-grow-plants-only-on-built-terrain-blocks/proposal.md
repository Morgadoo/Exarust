## Why

Found by lane `terrainrel` while releasing the held terrain lane: after
leaving blank blocks' plants out of the camera draws, 14-15 plant chunks
per outdoor arena (arenao3, arenao5) still stood over the void past the
reach. The new debug count (`EXA_TERRAIN_DRAW_DEBUG=1`) showed they name no
block: Exarust grew plants in every one of a tile's 64 chunks, also where
the tile build made no block (no triangles), and drew the random numbers
of those chunks.

## Evidence

REA (Exanima.exe 0.9.5), the per-block plant generator (the grass
extension's tile hook): both of its passes walk the tile's block node
list and take each node's chunk by the node's block column and row. A
block without a node is never visited: its points draw no random numbers
and it holds no instance list, so the original has no plants there. A
visited block draws two numbers a point in the first pass (whatever the
density) and two in the second (its size and turn, or two skipped draws
when nothing grows or the block has no instance list).

## What Changes

- `vegetation::grow` takes which chunks have a built block; a chunk
  without one grows nothing and draws nothing.
- The tile spawn passes the blocks it built.

## Impact

- `exa_world::terrain` (`vegetation::grow`, the plant spawn).
- On screen: no plants where no ground block was built (stray plants past
  the reach and in fully holed blocks); on a tile with a missing block, the
  plants of the blocks after it take the original's draws (they move).
- Lockstep traces unchanged (plants have no collider and no simulation).
