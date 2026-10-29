## Context

Extends [docs/NAVIGATION.md](../../../docs/NAVIGATION.md) (Columns, the new Moving props section, the shipped-level table) and [docs/formats/RFC.md](../../../docs/formats/RFC.md) (Props, `0x3DE0ECAC`).

## Decisions

- **Records as level frames.** The original writes the world matrix and derives only the local rotation (rows dotted with the parent's rows), keeping the model's local translation. Exarust derives the full local transform from the parent's frame at the placement, so the node's frame is the record's, which is what the original draws and hands to the body. The shipped records match the model's local translation within 1 cm, so the two agree on the data.
- **Which meshes are bodies.** A model node with a decoded rigid body that is not a placeholder (a node marked static in its file already has its body dropped by the model loader). Registration in the solver does not matter: the attach that sets the flag runs for every body read or copied, doors and levers included.
- **Level geometry.** A prop placed static marks every node `0x80000`, so its body meshes are stamped and still raise floors; a placed object's static bits mark its root only.
- **The raise.** Reuses the refresh's per-sample rule and voxel rounding; samples go one after another over the current floors, the floors as built untouched, so a later refresh starts from the floors without bodies.

## Open questions

- Whether the original's scene-wide refresh at level start uses sleeping bodies' samples: answered from the refresh read (it samples every registered mesh with points in the rectangle), so the built-grid result stands; queuing moved bodies is left out (proposal).
- Float order of the raise's pre-scaled matrix: Exarust rounds the world point in double precision like its stamp; a tie on a cell or voxel edge may differ (not seen in the corpus counts).

## Risks

- NPC plans no longer cross closed doors, and an opened door stays closed for the grid until the refresh is wired.
