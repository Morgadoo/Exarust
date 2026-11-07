## Why

TRenderer's gap: Exarust drew blended surfaces in Bevy's back-to-front
distance order, while the original walks render-state lists in a fixed
order and never sorts blended surfaces by distance.

## Evidence

REA on the scene pass, the list draws (deferred-queue and direct forms),
the sector walk and the two compile routines. The lists are sorted only at
compile time: the opaque list grouped by program, the blended list by
layer and opacity. The decal and alpha-tested lists keep registration
order. Each frame walks the scene's sectors in order, then the states in
list order, then each state's objects in the sector's registration order.
Decals are drawn for all sectors before the blended lists; the hook lists,
the particle pass and the weather come after them.

## What Changes

- exa_render: `passes::BlendState`, `blended_ranks`, `order_bias`;
  `ExaMaterial::blend_state`, `draw_order` and `depth_bias`; the
  `order_blended_states` system.
- exa_assets: blended records carry their state (class word, opacity,
  build order).
