# Proposal

## Why

Round 40's platform/frame/renderer lane added three rule models: the
super-sampled draw's per-pass schedule, the renderer's list draws with the
particle pass, and the timed log overlay's layout. A review against the
original's assembly (the packed listing and REA) found that the list
draw's `0x1000_0000` bit is an entry's scissor switch, not a sector's
rectangle bit; that the particle pass's `0x6000_0000` / `0x2000_0000`
words are the visibility mask it swaps in, not alpha words; that the
textured particle list binds its group textures in the depth pre-pass,
not the colour pass; that the overlay's `0x3E8` word is a 1000-pixel text
box height, not a colour; and that the resolve draw also takes a weight of
`1 / (4·pass + 1)` the schedule left out.

Phase: docs/PLAN.md Phase 5 — Visual and audio fidelity (the renderer's
pass order and super-sampling) and Phase 8's platform layer (the frame
log overlay).

Evidence: the super-sampled draw, the per-sector and per-list draw
routines, the blended per-entry draw, the particle pass and its two list
draws, and the overlay draw with its text draw's argument use, all read
instruction by instruction.

## What Changes

- `exa_render::passes`: `ENTRY_SCISSOR_BIT` and `scissor_switches`
  replace the sector region bit; `PARTICLE_MASK_SINGLE` /
  `PARTICLE_MASK_SUPERSAMPLED`; the particle stages are `Depth`,
  `TexturedDepth` and `Colour`.
- `exa_render::view`: every pass draws `min(count, 4)` sub-views; the
  pass carries `resolve_weight`; `PassState::particle_mask`.
- `exa_game::frame`: `OVERLAY_TEXT_MAX_HEIGHT` (1000) replaces the
  supposed colour word.
- docs/RENDERING.md and the exa_render README follow.

## Out of scope

Wiring: all three remain tested rule models. Bevy's phases own the draws,
the super-sampler still draws an ordered 2 × 2 grid, and the running app
has no live overlay ring. The meaning of the pass state's low-bit flag and
of the program-state bit `0x80` the textured colour draw clears was not
traced.
