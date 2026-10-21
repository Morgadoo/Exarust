# Proposal

## Why

A first pass extended the original's grid environment manager, its terrain
unit, its terrain vegetation and its chunk-tree importer: level strings,
camera and sector nodes, instance nodes, placement node overrides, the
character locale filter, the material lookup order, palette versions 1 and 2
and the default palette, the plants' density and chunk lookups and shadow
gate, and the rooms' door portals, light reach and light spill. An
instruction-level review against the importer's, terrain unit's, vegetation's
and room manager's assembly found rules that did not hold: a 0.75 vertex
tint on the plants that the loader never applies (it writes their normal
stream), a palette fallback on a parse failure, a version 1 upgrade with the
wrong fields and plant names, the fading flag replaced on every node kind,
placed-object node overrides read flat and matched without case (the
transform records nest, and names compare as 16 raw bytes), mesh position
overrides written into a material group's vertices instead of the position
stream, the light spill measured from the door point instead of the cell
centre, portal sizes read as full widths, and the plant density in single
precision.

## What Changes

- Plants: untinted (normals up for grass and small plants, as before); the
  density from the mask byte in double precision; the 240 × 240 mask texel
  and nearest-vertex hole lookups with ties to even; each chunk in its own
  6 m block's room; no collider; shadows only above shadow quality 4.
- Terrain palettes: version 1 upgraded field by field as the loader does;
  version 2's four inserted parameters; `default.ftb` only when the named
  palette is missing.
- Importer: the level's string table and the database-only read (records
  and strings); camera and sector nodes; the instance word and instance
  nodes drawing their owner's data; the fading flag replaced on meshes only;
  placement node overrides (nested transform records, exact names, position
  lists matched to the position stream and rebuilt into the object's model);
  world-database characters only in their own locale; a library material
  winning over a file's own record of the same name.
- Rooms: door portals (half widths and tops from plane B and C), the door a
  placed node stands on, the original's light reach and light spill, and
  the reset that hides every room.

## Capabilities

### New Capabilities

### Modified Capabilities

- `terrain`: palette versions and fallback; vegetation lookups, shading,
  collision and shadows.
- `level-decoding`: placement node overrides; instance, camera and sector
  nodes; the level string table and database-only read; the character
  locale filter; the material lookup order.
- `room-visibility`: door portals; light reach and spill.

## Impact

- `exa_formats::{terrain, rfc, level, characters}`, `exa_assets::model`
  (`ModelLoader::edited_model`, instance nodes, material order),
  `exa_world::{lib, rooms, terrain}`, `exa_physics` (`NoStaticCollider`).
- `docs/formats/TERRAIN.md`, `docs/formats/RFC.md`, `docs/RENDERING.md`,
  `docs/NAVIGATION.md` (pins move with nested overrides), crate READMEs.
- Evidence: REA/Ghidra pseudo-code and assembly of the vegetation's loader,
  attach, density sampler, per-block generator, activate hook and shadow
  draw; the terrain palette opener and its version 1 and 2 readers; the
  importer's root dispatch, node reader, mesh body reader, material-list
  handler, local database reader, database-only entry, placed-object and
  prop readers, the transform and position record readers, the character
  section reader and the finaliser; the room manager's sector and portal
  builder, door binding, light reach, light spill, reset and the cell
  helpers; the vector helpers they call (lerp, scale, multiply). Corpus: the
  plant libraries store no colour stream; palette versions (two version 2,
  none version 1); placement record sizes, names and words in every level;
  every exanima01 record resolves and its 14 position lists match their
  meshes.
- Out of scope: the grass wind program and its terrain parameters, the
  shading-quality normal-map toggle, the per-chunk rotation sort, the
  plants' ground-normal correction; per-room light aliases, the room-mode-off
  path, the occluder and lid segment tests, the omni shadow ceiling shell;
  the composite pools and tile upload; the unknown-chunk handler registry,
  `0x3DE0EC00` tree records (none ship), the position record's soft-body
  word, character motion state and AI home, scene-node visibility flags,
  door and dynamic-body registration, the collision-only and streaming entry
  points; a retail capture of the plants.

## Phase

Phase 2, static world: level decoding, terrain and room visibility.
