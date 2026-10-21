# `exa_assets`

The bridge between `exa_rpk`/`exa_formats` and Bevy: the `Game` resource
holding `GameData`, conversion of parsed data into Bevy assets, and
installation discovery (`--game`, `EXANIMA_DIR`, `.env`).

**Phase:** 0 (started), 2 (asset source `exanima://`, loaders, direct BC upload, streaming).

Model nodes carry their rigid body with the physical divisor of their first
mesh part's material (`ModelNode::rigid`, `NodeRigidBody::mobility`), what
the solver registers a prop with.

## Textures

`texture` turns a `.rfi` into a Bevy image with the original's texture
parameters. A texture's creation word (`texture::flags`) holds the
magnification, minification and mip filters, the data bit (`0x100`: linear,
never sRGB), the skip-LOD-bias bit (`0x200`), one wrap bit per axis, the
anisotropy mode (`& 3`), the two-bit detail class and the target kind. The
image's own header contributes its wrap bits, detail class and target; a
material contributes the record's texture load flags (`Material::texture_flags`,
offset `0x54`) and the texture quality mask, and the loader adds the per-slot
mode bits: normal, gloss and packed specular are data, unpacked specular is
colour, and a material with a subsurface colour loads its diffuse without the
LOD bias. A stored word whose top byte is `0x82`, `0x92` or `0x12` is data
whatever the slot asks.

The sampler follows the word: `0x10`/`0x20` pick linear minification and
magnification, `0x40` samples the mip chain and `0x80` blends between levels;
the wrap bits pick repeat or clamp per axis; anisotropy is
`2^(quality level − mode)`, capped at 16. The LOD bias is
`base · f(ss) / 3` in f32, with `f(ss) = ss` below 2 and `ss − 1` from 2 up;
`TextureSettings` carries the base, the supersampling setting, `TextureDetail`
and the driver's largest texture side (which replaces the routine's static
65536) and its units (clamped to 16). **Bevy's sampler descriptor has no LOD
bias**, so the value is computed and exposed but not applied; a renderer that
wants the original's bias must pass it to the shader. The GL texture-unit and
render-target texture calls (selecting a unit, binding, creating an empty
texture for a framebuffer attachment) have no Exarust item: Bevy owns render
targets and binds per draw.

`TextureDetail` keeps the first uploaded level at
`max(0, class − detail + 1)`, raised while a side exceeds the driver's limit.
A stored chain is uploaded from that level (all of it when mipmapping is on,
the level alone otherwise); a single-level image gets the Kaiser chain from
`exa_render::mipmap`, starting at that level; a compressed image with no
stored chain keeps its base, since it cannot be downsampled. A failed image
gets the original's 1×1 grey placeholder (64, 64, 64, 255) and a warning.

`texture_cache` is the manager: names are sixteen bytes, case-folded, kept in
name order; one record per name and colour space holds the creation word,
alpha flag, image handle and reference count. A second request for a live
name joins the record and replaces its word (the last request wins, as in the
original); the last release frees it. There is no size budget: reference
counts are the only eviction. The original keys by name alone; Exarust adds
the colour space because Bevy needs distinct sRGB and linear images.

## Streaming

`stream` is the exchange the original runs between its main and loader
threads: a ring with a head, a tail, one kept-free slot and growth; requests
in order; a promotion that moves one request to the front (the original's
"Priority item"); removal; and an upload pass that takes only what was ready
and honours a budget. `TextureStream` is the texture loader's queue:
`request` (a duplicate of a queued name is refused), `promote`, `cancel`,
one `decode_next` per pass on whatever executor the caller uses, the loader's
yield while uploads wait, `load_now` for the priority path (promote, decode
and upload one name), and `pump_uploads(budget, upload)` on the main thread
with a per-upload completion callback. The original drains every ready upload each pass; the budget is
Exarust's addition for memory-bounded loaders (unbounded when omitted).

`content_stream` is the stream thread's scheduler: placed level objects first,
then one generic job (its completion queued for the main thread), then one
requested database object, one requested character, and the pending streamed
character nearest the focus point (strict `<`, so the first of equal distances
wins), else idle. Its queues have the original's capacities (16 jobs and
completions, 32 object and character requests/results, 16 attachments, 128
streamed characters); `fetch_object`/`fetch_character` fetch by request key,
`pump` runs completions and collects attachments and scene additions,
`wait_for_level` returns the built object list and missing count, and
`wait_for_characters` drains attachments while the caller pumps other work.
Content builders are supplied by the caller (`ContentBuilders`), as the
original's thread calls into the database, character and item factories. The
queue and loop are implemented; the level-object chunk reader, the database
object factory and the character builders remain in their own components, and
Exarust's level loader does not drive this scheduler yet.


Materials that show wound blood (`docs/RENDERING.md`, Blood): the human
skin, painted and run-time apparel and factory items, any record selecting
dirt mode 4 with a diffuse map, and a bleeding race's body-model materials
whose names start with `s`; each binds the speckle texture `spatter01`
(`ModelLoader::spatter`).

Physique (`morph`): `ModelLoader::morphed_model` / `morphed_job` build a
character's variant of a race body or garment at its `BodyMorph` (race
stem and record physique): each node's share of the race's morph set is
transferred once per race and model (a garment piece from the frame of the
base body part its node names), then the meshes are morphed (normals and
boxes refitted) and the body parts' translations moved with their joints
before skinning; the body's parts are skinned on the morphed shape, a
garment from the unmorphed body. Weights that change nothing, and races
without a morph body, return the shared model. See
[`docs/formats/RFC.md`](../../docs/formats/RFC.md) (Morph sets).

Mesh editing (pure, no Bevy types): `mesh_edit::EditMesh` is a decoded
mesh in the original's editable form (one element index per corner and
channel, the corners of each position element in an `Incidence`), and
`EditMesh::render` converts it back to render form as the original does:
smoothing groups from the triangle flags, runs split by material, texture
seams and mirrored texture winding, then the vertex-cache order of
`vertex_cache` (bit-exact fast logarithm and exponential). `mesh_smooth`
is the original's Loop subdivision (creases, texture folds, weights and
output order as the original's); `ModelLoader::smoothed_model` smooths
every mesh node of a model and refits its bounds (`exarust model … --msm`).
`polyhedra` builds the icosahedron, its cap and their subdivisions for the
effect shells. See [`docs/formats/RFC.md`](../../docs/formats/RFC.md)
(Editable meshes).
