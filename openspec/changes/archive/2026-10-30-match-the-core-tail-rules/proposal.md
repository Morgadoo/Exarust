# Proposal

## Why

Phases 2 (static world: terrain materials, the object manager) and 4
(items: the world-load fill) of docs/PLAN.md. The coretail lane reviewed
the core helpers and small units (formatting, engine math, the prelude,
the object manager, terrain materials, the randomisers, the save package)
against the original. Most were already in; the terrain channel bytes,
the used-material list and merge were not wired, the world-load walk
stopped at the refresh, and the fast logarithm, exponential and
approximate reciprocal square root were copied in several crates.

Evidence: the terrain tile evaluation, its mask merge routines and the
world-load walk of the randomiser unit, read instruction by instruction
in REA and the objdump listing (the 0.001 gate in double, the per-leaf
64-bit rounding and signed low-word compare, the single-child paths of a
rule, the remainder's successive subtraction, the f32 1/255 product, the
container walk's slot append and the palette byte's two shared readers).

## What Changes

- `exa_world::terrain`: each vertex accumulates the original's 256 × 256
  byte channels per leaf; the used list keeps materials whose summed leaf
  weight is above 1/510 with material 0 forced; the used channels are
  merged byte-exactly and packed into the tile's covers. The rule walk
  follows one child for an ordered input outside the bounds, splits
  otherwise (a NaN splits), and the remainder loses each layer in turn.
- `exa_world`: a model-less record spawns its container volume (from its
  parsed volume section) or its operative placeholder, with its sections,
  mechanism records and device.
- `exa_items::randomise::world_load`: the palette generator hook on the
  palette byte's signed variance plus wealth, and every container's
  table children added as the fill's slots with the container's item
  limit.
- `exa_physics::engine_math` is the one fast logarithm, exponential and
  approximate reciprocal square root for every crate above it.
- `exa_game::prelude`: the tips popup follows the live `ShowGameTips`
  option.

## Out of scope

The renderer's sixteen-layer cap (the original packs up to 32 planes),
the tile-flag gate on the merge (always on), the Gaussian blur kernel and
height-rule scaling (tested, unwired), a live world-load walk over a
writable object database (only the placed containers are filled, by
`exa_game::container_fill`), the apparel palette generator's body, the
practice opponent's fixed placement, and the tips texts (retail data).

## Impact

`exa_world`, `exa_items`, `exa_formats`, `exa_physics`, `exa_game`,
`exa_ai`, `exa_combat`, `exa_assets`, `exa_render`; docs/formats/TERRAIN.md
and the crate READMEs.
