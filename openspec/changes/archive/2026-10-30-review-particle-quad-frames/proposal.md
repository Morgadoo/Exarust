# Proposal

## Why

A lane extended the particle emitter, the fire presets and the electricity
effect: the quad frames' node-axes and view-plane flags, the fixed quad,
the emissive pass onto the host mesh, the attach rules, the fire marker
import, re-creation, the legacy key path, the key index range, the level
device's record fields, the arc's buffer sizes, the arc draw's per-pass
clock, the equipment check and the discharge. A review against the
original's assembly found that the node-axes bit took the transpose of
the original's matrix product, the view-plane scale dropped the double
precision and the reciprocal-square-root normalisation, the flag docs
claimed fire templates set bits none of them sets, the light's
first-frame bit was tied to the at-rest test (a dim light asked for a
full update every frame), the key loops were shifted by one entry, the
legacy sRGB path used the decode threshold and a wrong luma weight, the
arc draw's "branch" loop is the renderer's supersample-pass loop (not the
tendrils), the spark burst's buffers counted 26 records instead of 20, the
device arc's direction default was invented, and a discharge on an item
whose record is gone did not clear its arc effect.

## What Changes

- The node-axes bit dots each camera axis with the node's three axes; the
  view-plane bit's −0.9 scale is double precision and the axis is
  renormalised by `RSQRTSS`, as the stretched quad's axes and the diamond's
  facing line are; other quad modes draw nothing.
- The first-frame bit clears after every frame.
- The emissive pass reaches the host's geometry children.
- The load-time key loops scale entries 0..count; the legacy path encodes
  with the engine's sRGB curve (linear to 0.0031308), luma 0.299/0.587/0.114
  and the saturating byte store.
- `Arc::pass_clock` replaces the tendril pass count and branch clock.
- A spark burst's buffers are sized for its twenty records.
- The device record's active bit and its fixed end point (flag 0x02).
- A discharge clears the effect on a missing record or a spent charge.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: particle quad frames, fire key loading, arc draw clock and
  buffers, device arc fields, discharge.

## Impact

`crates/exa_world/src/{particles,fire}.rs`,
`crates/exa_render/src/electricity.rs`, `crates/exa_assets/src/human_skin.rs`,
`docs/RENDERING.md`, `docs/formats/RFC.md`, the `exa_world` and
`exa_render` READMEs.

## Phase

Phase 5, visual and audio fidelity (Effects: particles: fire, sparks).

## Evidence

The particle system's per-frame pass (the quad axes, the node-axes and
view-plane blocks, the first-frame and at-rest bits), the engine's
matrix-times-vector and vector-times-matrix helpers and its approximate
normalisation, the emissive and attach routines, the fire marker import,
the key loaders and the colour unit's sRGB encode and desaturation, the arc
draw, the arc geometry allocator, the tendril emitter's setup and free-record
search, the level device's arc import and the queued discharge, in the
shipped executable's disassembly.

## Out of scope

The emissive scale on the host's geometry children, the body-attached
emitter's exact frame (the body matrix times the node's local matrix), the
level device's caller, the arc draw's supersample passes (Exarust's camera
draws once), the exa_render arc ribbon's model of the burst's extra
tendrils (the exact rule is `exa_game::thaum::arcs::spark_burst`), and the
particle material's parameter block.
