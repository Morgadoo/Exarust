## Why

With the round 27 contact rules, five of exanima01's fourteen banners still never slept with `EXA_CLOTH_CONTACTS=1`: countdown pinned at 32, eleven sleeping body parts handed over, about 1.4 ms each and 8 ms for the cloth stage. A countdown pinned at the cap with sleeping bodies means the cloth's own velocity sum stays above the count-up threshold every tick. The cause is the pin rule, which was a labelled guess: every vertex was held at weight 0.1 toward its rest pose. Where a prop's spheres cut into that rest pose, each constraint pass pulled the cloth back in and each body push threw it out, so the banner never came to rest.

The original's cloth controller builds its pin list when it starts. It reads the mesh's face flags, not skin weights. A position element is pinned only when every face about it carries a bit of `0xF0000000`. The pin is firm (weight 1) when each of those faces also carries a bit of `0xC0000000`, and loose (0.1) otherwise. One face without the bits leaves the element free. Every shipped cloth (the banners `uw01_banner01`, `uw01_banner02`, `bannerubma01`, `mawbanner01` and the wraith's robe) marks its hem faces with `0x80000000`. So the hem holds firmly and the rest of the cloth hangs free. The same controller makes one particle per position element. Exarust made one per drawn vertex, so a UV seam split the cloth.

## What Changes

- `exa_physics::cloth::face_flag_pins`: the original's pin list from the face flags (with the `PINNED_FACES`, `FIRM_PIN_FACES`, `FIRM_PIN` and `LOOSE_PIN` constants).
- `exa_assets::ClothMesh` (`ModelNode::cloth_mesh`): for a cloth-marked node, the mesh's position elements, its triangles over them with their face flags, and each drawn part's vertex elements.
- `exa_world::cloth`: each cloth part carries a `ClothSource`. It is built over its position elements (`cloth_of_elements`), and the vertices a seam splits draw one particle (`ClothInstance::vertex_particle`). It is pinned by `face_flag_pins`. A mesh built without its file's data keeps the old 0.1 fallback.
- Debug counters (`EXA_DEBUG_CLOTH=1`): each cloth line adds the velocity sum² its countdown read, the two thresholds, its pins, and the parts that pushed it (named node, entity, countdown). `ClothBody::part` and `Cloth::pushed_by` are debug tags that no rule reads.

The contacts stay opt-in until the GPU check passes.

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

REA assembly:

- the cloth controller's start-up, which makes one particle per position element and calls its pin-list builder for a root cloth only;
- the pin-list builder: for each element, the incidence walk over the face-flag words, the `0xF0000000` test that leaves the element free, the `0xC0000000` test that picks 0.1 instead of 1.0, and the 20-byte pin records (element, rest point, weight);
- the pin pass, which lerps each pinned particle toward its rest point by the record's weight.

The data: the face-flag chunks (`0x3DC3`) of every cloth-marked mesh in Objlib and Characters were scanned. Each one marks part of its faces `0x80000000` and the rest 0: banner01 127 of 387, banner02 182 of 364, bannerubma01 278 of 1064, mawbanner01 69 of 853, the wraith's robe 32 of 256.

## Out of scope

- The pin rest following the node's or the owning body's matrix each tick. Exarust's cloth is local to its node, which carries it.
- The geodesic ranking of pins.
- A multi-part cloth node sharing one particle set. Each drawn part is still its own cloth. No shipped cloth has more than one part.
- The body's share of a contact. It still needs a mutable body in the cloth stage.
