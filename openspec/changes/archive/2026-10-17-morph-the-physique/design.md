## Context

How the original gives a character its body shape. Format page:
[`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (Morph sets).

When a race's base model loads, the loader also loads `<stem>morph.rfc`
and wraps the morph set its first node carries (sub-chunk `0xEDC0C589`
after the mesh body; an older layout of numbered target nodes is built
otherwise and never shipped). Only humans and ancients have such a file
(byte-identical). The set holds 18 landmarks (bone k + 1's origin in the
morph body's space), the base positions of the morph mesh's position
stream and 11 targets of landmark and position offsets, in cm. Then
every mesh node of the base model gets a set of its own by the transfer.

The transfer maps each of the piece's position elements into the morph
mesh's space (the piece's frame, then the inverse of the morph node's),
asks the mesh's kd-tree for the triangles within a 10 cm sphere and keeps
the nearest closest point; its barycentric coordinates blend each
target's three corner offsets. The landmark nearest the piece's origin
(landmarks offset by the morph node's translation) gives the origin's
offset, which every element's offset loses, and the result is taken into
the piece's frame (dot with its rows). Elements with no surface within
10 cm take, pass after pass, the inverse-distance mean of their
neighbours (each incident triangle's next corner) set before the pass;
those never reached keep minus the origin's offset. Garment pieces get
theirs when first dressed, after the apparel cache placed each piece on
the base body part of its bone (the files' own frames are an artist's
layout). The same mapping also hands the morph body's skin weights to
garments; Exarust keeps its own nearest-vertex rule for those.

Per character, the builder clears the weights and sets them from the
record (picker corners 0–3, age 4, female 5 and 6, height 10; 7–9 are
never weighed), applies every mesh node's set (positions = base +
Σ w·offset for weights of at least ≈1.2e-7; the node's world position =
origin + Σ w·origin offset; box and sphere refitted from the positions),
recomputes each morphed node's local translation from the moved world
positions under its parent's (rotations kept), and stores the pivots.
Only then does it build the limb bodies (voxelising the morphed meshes),
bind the motions and capture the part list: the joints, links and pivots
follow the physique. The dressed mesh is skinned per character with the
morphed pivots.

## Goals / Non-Goals

**Goals:** decode the sets; the weights, the transfer and the morph as the
original's; placed characters, their garments and joints in their
physique; the default body unchanged; a debug flag and a tool command.

**Non-Goals:** limb spheres from the morphed meshes (C20); the skin-weight
transfer to garments; character creation's sliders; hair and skin tones.

## Decisions

- **Rules Bevy-free in `exa_formats::morph`.** Parsing, weights, the
  transfer (a 10 cm uniform grid over the body's triangles in place of the
  kd-tree; ties go to the lowest triangle), applying, bounds and the joint
  walk are plain data, unit-tested with synthetic sets and pinned by
  corpus tests.
- **Variants built on the CPU before skinning.** `exa_assets` builds a
  character's variant of a model (morph, then meshes) and caches it by the
  race stem and the physique's bits; the transfer is cached per race and
  model. GPU morph targets were not used: the physical body reads part
  boxes from the meshes, normals are recomputed from the shape, and C20
  will voxelise the morphed meshes.
- **Skinning order.** Body parts are skinned on the morphed shape with the
  morphed joints (the original skins the dressed character after the
  morph); a garment's nearest-vertex weights are taken on the unmorphed
  geometry against the unmorphed body, where their correspondence is the
  same.
- **Normals** use Exarust's existing area-weighted smoothing groups on the
  morphed positions (the original's angle-weighted builder is a
  pre-existing difference for every mesh).
- **Records without one.** The player, the duellist and the character view
  have no record: they keep the model's body (physique ½, 1 weighs
  nothing), and `--physique` morphs them for checks.

## Risks / Trade-offs

- Every distinct physique builds its own meshes: a level's characters
  cost a build each at spawn (the body synchronously, garments on the
  async pool behind the loading screen) and their own GPU meshes.
- A zero-distance neighbour in the fill is skipped (the original would
  divide by zero).

## Open questions and how they were answered

- RB42 (transfer rule; do morphed origins reach the links?): REA of the
  transfer, the mapping, the sphere query, the joint walk and the
  builder's order: yes, the morph precedes the limb bodies and the motion
  binder (`context/re/master-animation.md`, "M18 findings").
- Which frame a garment piece is transferred from: REA of the apparel
  cache placement (the base body part of the node's bone), confirmed by
  the corpus (3559 of 4001 garment bone nodes sit far from their bone in
  their files).
- The height target against the standing height: corpus, the pelvis
  landmark drops 11.20 cm (the standing pelvis rule's 11.25 cm).
- Retail comparison (RR16, the creator's sliders at their extremes) stays
  open; the in-game check is the light character-view shots.
