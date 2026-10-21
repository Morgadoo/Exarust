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
anisotropy mode (`& 3`), the two-bit detail class and the target kind. A
material passes the record's texture load flags (`Material::texture_flags`,
offset `0x54`) or-ed with the texture quality mask (`texture::material_word`),
each slot adds its mode bits (normal, gloss and height maps, and the
specular map of a material whose kind has bit 0, are data with no bias; a
diffuse whose subsurface red, green and blue are not all zero skips the
bias), and the image's header flags from bit 16 up (wrap, stored chain,
detail class, target) join the word. An uncompressed format word with bit 16
set, or a compressed one whose top byte is `0x82` or `0x92`, is data whatever
the slot asks.

The sampler follows the word: `0x10`/`0x20` pick linear minification and
magnification, `0x40` samples the mip chain and `0x80` blends between levels;
the wrap bits pick repeat or clamp per axis; anisotropy is
`2^(quality level − mode)` while the mode is below the level and not 3, else
1 (and 1 unless every filter is linear, which wgpu requires). The LOD bias is
`base · f(ss) / 3` in f32, with `f(ss) = ss` below 2 and `ss − 1` from 2 up.
`TextureSettings` carries what `TextureQuality` writes (`with_quality`: the
mask, the anisotropy level and the bias base, −0.75 at quality 5),
`TextureDetail`, the supersampling setting, the device's largest texture
side and its units (clamped to 16); `fallback` is what the original forces
on a generic (software) pixel format. **Bevy's sampler descriptor has no LOD
bias**, so the value is computed and exposed but not applied; a renderer that
wants the original's bias must pass it to the shader. The GL texture-unit and
render-target texture calls (selecting a unit, binding, creating an empty
texture for a framebuffer attachment) have no Exarust item: Bevy owns render
targets and binds per draw.

`TextureDetail` keeps the first uploaded level at
`max(0, class − detail + 1)`, raised while a side exceeds the device's limit;
in the shipped data 21 of 8 839 images (class 3) start at level 1 at the
retail detail 3. A stored chain is uploaded from that level (all of it when
the quality mask lets the mip bit through, the level alone otherwise); a
single-level uncompressed image gets the Kaiser chain from
`exa_render::mipmap`, starting at that level; a compressed image with no
stored chain keeps its base alone and loses its mip filter. An image that is
shipped but cannot be read gets the original's 1×1 grey placeholder (RGB
64, 64, 64, repeating) and a warning. One the game does not ship is no map:
the original would draw the placeholder, but Exarust has no registry of the
images the original builds at run time, so such a name may be one of those
(`model::tests::shipped_materials_missing_images` lists them).

`texture_cache` is the manager: a name is its first sixteen bytes, cut at the
last dot among them and with `A`–`Z` folded, kept in the original's order
(four little-endian words compared in turn); images are looked up by that
stored name. One record per name and colour space holds the creation word,
alpha flag, image handle and reference count. A second request for a live
name takes a reference and leaves the record as it is (the first request's
word stays, as in the original); a duplicate insert does the same; the last
release frees it. There is no size budget: reference counts are the only
eviction. The original keys by name alone; Exarust adds the colour space
because Bevy needs distinct sRGB and linear images.

## Streaming

`stream` is the exchange the original runs between its main and loader
threads: a ring with a head, a tail, one kept-free slot and growth; requests
in order; a promotion that moves one request to the front and lets the last
fill its place (the original's "Priority item"); removal; and an upload pass
that takes only what was ready and honours a budget. `TextureStream` is the
texture loader's queue: `request` (a duplicate of a queued name is refused),
`promote` (which also finds a request decoded and waiting for its upload),
`cancel`, one `decode_next` per pass on whatever executor the caller uses,
the loader's wait while uploads are pending (skipped for a promoted request;
pausing only stops the wake-ups), `load_now` for the priority path (promote,
decode and upload one name), and `pump_uploads(budget, upload)` on the main
thread with a per-upload completion callback. The original drains every
ready upload each pass; the budget is Exarust's addition for memory-bounded
loaders (unbounded with `usize::MAX`).

`content_stream` is the stream thread's scheduler: while a level's list is
unfinished, its next placed object alone; otherwise one generic job (its
completion queued for the main thread), one requested database object and
one requested character in the same pass, and only when none of those was
queued the pending streamed character nearest the focus point (squared
distance, strict `<`, so the first of equal distances wins), else idle. A
request that builds nothing is dropped. Its rings have the original's
capacities (16 jobs, completions and attachments, 32 object and character
requests and results, 128 streamed results); `fetch_object`/`fetch_character`
fetch by request key, `pump` runs completions, then hands over the streamed
characters to insert and the characters to attach, `wait_for_level` returns
the built object list and missing count, and `wait_for_characters` drains
attachments while the caller pumps other work. Content builders are supplied
by the caller (`ContentBuilders`), as the original's thread calls into the
database, character and item factories. The queue and loop are implemented;
the level-object chunk reader, the database object factory and the character
builders remain in their own components, and Exarust's level loader does not
drive this scheduler yet.

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
