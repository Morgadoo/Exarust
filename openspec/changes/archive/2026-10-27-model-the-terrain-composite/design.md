# Design

Rules: [`crates/exa_world/README.md`](../../../crates/exa_world/README.md)
(terrain), `docs/formats/TERRAIN.md` (the composite) and `docs/RENDERING.md`
(Terrain).

## Decisions

- The compositor is kept as data and state transitions, tested
  synthetically; the per-pixel shader stays the one content path, so no
  offscreen set is allocated.
- The selector is a separate struct fed the blocks in tile order, each
  block's visibility and nearest light distance; it is not driven by the
  running game (every tile is inserted behind the loading screen).
- The wet-floor term is applied in the shader from a material flag, set
  when the palette's source exceeds 0.4; only the sewer palette (0.5) does.
- The tile bits are recorded during the level build; the frame hook's walk
  is a model with nothing to do after a load.

## Not wired

The selector, the frame hook, the pass keys and values and the pools have
no GPU consumer; they are listed in the proposal's out-of-scope paragraph.
