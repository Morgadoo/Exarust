## Context

The cloth's controller starts once per root cloth. It copies the mesh's position stream into its particles, one particle per position element. Then it builds its pin list from the editable mesh's incidence (each element's corners, each naming a face) and the per-face flag words. Exarust's cloth is built later, from the drawn part's Bevy mesh, whose vertices are split by UV. The pins were guessed (every vertex at 0.1), because the shipped meshes carry no skin and the builder had been read as a skin test.

## Decisions

- **Face flags, not skin.** The builder's words are the per-face flags (the `0x3DC3` chunk; their low 16 bits are the smoothing groups). The two masks and the 1.0/0.1 constants come from its assembly. The rule does not depend on the walk's order: the walk breaks on the first unmarked face, and that face leaves the element free anyway. So `face_flag_pins` keeps two flags per element. An element no face touches is pinned firmly, as the empty walk leaves it.
- **Where the data lives.** The face flags and position ids exist only in the parsed file. `ClothMesh::of` keeps them on the model node for cloth-marked nodes only. Each part's vertex elements are listed in `group_mesh`'s first-use order, so they line up with the part's Bevy vertices. The world checks that the lengths match before using them, and falls back otherwise.
- **Welding.** `cloth_of_elements` maps each vertex to the particle of its element, at the element's first vertex. The triangles and pins are remapped. The mesh write-back reads `vertex_particle`. The hand's grab already works in particle indices.
- **Fallback.** A cloth with no file data (built in code) keeps every vertex at 0.1. It is labelled as Exarust's own.

## Verification

- `face_flag_pins`: a hem face, a loose face, no flags, and a corner past the elements.
- `ClothMesh::of`: faces over elements, flags with and without the chunk, and a seam-split vertex. The part's element list matches the built mesh's vertex count.
- `cloth_of_elements`: a seam-split square welds to four particles with six links, and only the hem element is pinned.
- A 5 × 5 banner pinned by its top row of flagged faces, against two sleeping spheres that cut into its rest pose, falls asleep within 3000 ticks, with its hem within 1 cm of its rest. A temporary probe of the same scene with every vertex at 0.1 stayed awake at 32 with a velocity sum² near 7.8. Four milder sphere scenes slept under both rules.

The GPU check is for the integrator: `EXA_CLOTH_CONTACTS=1 EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5`. Each banner line should show `pins` well below `particles`. A few seconds after the load, the stage line should read awake ≈ 0 and well under 2 ms. A banner still stuck at 32 shows its `velocity sum²` against `up`, and the sleeping parts that keep pushing it under `pushed by`. Only after this passes may `ClothContacts` default to on.
