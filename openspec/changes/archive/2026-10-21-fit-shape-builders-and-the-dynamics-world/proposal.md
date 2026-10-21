# Proposal

## Why

The original's dynamics world unit (`RF_Dynamics`: the simulations' fixed
step driver, their two grids, the sectors' purges and flags, the node and
body tree helpers, the placement overlap query) and its rigid-body shape
builders (the limb, root and factory-weapon bodies fitted to a mesh) were
ported by a first lane and reviewed against the listing. The review found
the builders scaling only three of the four rest points and summing three
of the four gravity shares (both loops run 0 through 3), the side flag's
axis override missing from the root and weapon builders, and the world
model's grid shift, backlog drop, purges, flags and overlap-query mask test
inverted or invented. Props then fell at 4/3 g. Hung props (lamps, chains,
wall rings) were registered with their stored links as world links.

## What Changes

- The three shape builders over the dynamics volume fit
  (`exa_physics::volume::fit`): the sphere sort, the box (a limb's cut at
  its origin, a mirrored root's or weapon's grown over its children), the
  extent clamps in double precision, the axis order and its side override,
  the tetrahedron scaled and placed for all four points with its +x flip,
  the point masses, mobility and gravity shares; the quality, wide-query
  and factory-weapon fit flags (the shipped record: 1.1, even axes, 6
  cells, no free corners, tag 2, not mirrored).
- Gravity shares sum to four over all four masses again (props fall at g).
- The dynamics world unit as records and rules: clock, driver, closed
  simulations, environments, two grids with the original's cells, shift
  and rebuild, tree helpers, particle matrix, freeze, sector purges and
  update order, placement query, watch list.
- Hung props register with world links anchored in their parent node.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: props registered as bodies (hung trees too, gravity
  over four masses), builder resolution, shape builders, the dynamics world
  unit's grids, driver, purges and placement query.

## Impact

- `exa_physics::shape` (new), `exa_physics::dynamics` (new),
  `exa_physics::body` (world links), `exa_physics::body_rigid`,
  `exa_physics::prop`, `exa_world::bodies`; `crates/exa_physics/README.md`,
  `crates/exa_world/README.md`, `docs/formats/RFC.md`.
- Evidence: REA/Ghidra pseudo-code and assembly of the 37 unit routines and
  the 9 builder routines, the body setup, the vector and matrix helpers they
  call (subtract, lerp, scale, weighted sum, the floor helper with its
  rounding mode, the sphere separation test), the factory's fit record read
  from the executable's data; `context/re/body-links.md` §6.4 for the world
  links, `context/re/body-q8-q9.md` §2 for the environments.
- Out of scope: the refit resync after a limb's mesh changes; the callers of
  the builders (character generation and the item factory still build their
  bodies the old way); driving the dynamics records from the game (Bevy's
  fixed step stands in for the driver, the body solver keeps its own sleep
  and pair state); the swept shape and insertion tests of the placement
  query and the sector step's grid linking (the sector lane); the world-link
  laws' own listing (the body kernel); doors, levers and plates as bodies.

## Phase

Phase 3, bodies: physics (the rigid-body shape builders of C20 and the
dynamics world infrastructure under the body solver).
