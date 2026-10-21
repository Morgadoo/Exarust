# `exa_assets`

The bridge between `exa_rpk`/`exa_formats` and Bevy: the `Game` resource
holding `GameData`, conversion of parsed data into Bevy assets, and
installation discovery (`--game`, `EXANIMA_DIR`, `.env`).

**Phase:** 0 (started), 2 (asset source `exanima://`, loaders, direct BC upload, streaming).

Skinning (`skin`): the original's seam builder, run on body parts and
garments while a model is built, read instruction by instruction. Body parts
trace wide seam groups whose boundary edges come from neighbours of another
class with a byte-2 marker (the marker class sets the falloff), grow over the
finer classes, and give each joint a group by the original's keep-or-swap
scores; garments run the same groups and then groups of their marked
triangles, each taking its nearest joint. A position belongs to the first
group that reaches it. The weights are per position id: fixed 0.5/0.25/0.125
splits on the groups, and up to three damped distance blends elsewhere
([`docs/formats/RSQ.md`](../../docs/formats/RSQ.md), Skinning). The shipped
human and ogre bodies are checked watertight. The weights run in the model
build on the compute pool; a mesh that ships weights keeps them.

Hair (`hair`): the original's hair colour picker (five decoded hue anchors,
desaturation by the saturated value, a specular grey moved in, the engine's
sRGB chain), with the feral level's lightening, the material's gloss
0.9 − 0.4·f and opacity (1 − 0.65·f)², each mixed-precision step (a value
widened to double and narrowed) as the instructions take it. The character
crate's `hair` system tints the hair materials with it.

Merging (`merge`): `merge_parts` appends the visible body parts and garment
pieces into one skinned mesh as the original's character assembly does:
each part placed by its frame, its vertices, normals, UVs and colours
appended, its weights kept vertex for vertex (a part without them bound to
its bone), and its triangles grouped by material in first-seen order. `culled_indices` drops triangles with a slot's cull bits
(`exa_character::layers::cull_bits`; which triangle flags they test is
not traced) and `occlusion_alpha` writes the generator's per-vertex
occlusion alpha (vertices on the border of the marked triangles fade). The
merge order itself comes from the skin builder, outside the generator, and
is not checked against its instructions here. The loader still draws a mesh per
node; the merged mesh is built by callers, not by the loader yet.

Skin tone (`human_skin`): a character's `SkinLook` selects the built-in skin
material for its record's two tone words (and the women's texture set),
cached per race, tone and sex. The tone multiplier is the original's, bit
for bit: its HSV tone table, grid halves and fast power.

Model nodes carry their rigid body with the physical divisor of their first
mesh part's material (`ModelNode::rigid`, `NodeRigidBody::mobility`), what
the solver registers a prop with.

## Materials

`ModelLoader::exa_material` builds the draw state from a library record with
the original's plug-in-masked feature word (`shader_program::feature_key`):
the record's flags after loading, masked by its type. An unlit record's
`MatDif.rgb` is zero (it draws emission only), every lit record whose blend
source nibble is ONE premultiplies its diffuse by the opacity (not in
refraction mode), and the emissive doubles only when that masked word keeps
`MatEmi` (`0x8`) with the lit/specular bits and the emissive layer. A
transparent record (destination nibble 3) is rewritten opaque when in
refraction mode, or with opacity above 0.99, fall-off below 0.01 and no
opacity flag; the compare widens the stored f32 to f64 as the original's
does; the rewrite starts from the class after the alpha-test rewrite and
reads the loaded flags. Maps bind only where the factory binds them: the
colour slot needs `0x20000` and a lit record (diffuse) or `0x400000`
(opacity), the emissive needs `0x80000`
and `MatEmi`, the specular `0x40000` and `0x100004`, the normal `0x200000`
and a shaded material word (`0x100807`), and a type-2 record keeps only the
colour/opacity slot. Refracting records cast shadows, and a record whose
destination nibble is ONE never does.

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
the slot asks. The target bits are the original's switch: `0x10000000` a 1D
texture, `0x20000000` and every value from `0x50000000` up 2D, `0x30000000`
3D and `0x40000000` the cube. The seven uncompressed formats the original
uploads as they are (`texture::COMPATIBLE_FORMATS`, compared under
`0x800CFFFF`) decide whether a colour word has an sRGB internal format:
ALPHA8 and the two-channel AB8/XY8 do not, so they upload linear.

The sampler follows the word: `0x10`/`0x20` pick linear minification and
magnification, `0x40` samples the mip chain and `0x80` blends between levels;
the wrap bits pick repeat or clamp per axis; anisotropy is
`2^(quality level − mode)` while the mode is below the level and not 3, else
1 (and 1 unless every filter is linear, which wgpu requires). The LOD bias is
`base · (f(ss) · ⅓)` in f32 (the product with the third first, as the
original's instructions order it), with `f(ss) = ss` below 2 and `ss − 1`
from 2 up. A BC4 image is uploaded as `Bc4RUnorm`, the original's
luminance LATC format: one grey channel with no sRGB variant, whose red the
shader repeats over rgb (`material::flags::GREY_DIFFUSE`), so the value is
never decoded. Other BC images keep their formats (BC1 and BC3 with an sRGB
variant, BC5 linear), and the rest decode to RGBA8.
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

`TextureUnitCache` is the original's per-unit bind cache: a texture already
on its unit is not bound again, a new one makes the unit active, binds its
shared sampler when that changed, binds the object and sets the global LOD
bias unless the word has bit `0x200`; unbinding clears the sampler.

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
`promote` (which finds a request queued or held by the loader; one decoded
or uploaded is no longer found, as the loader clears its status bits when it
goes on), `cancel` (a queued request or a
decoded one is dropped, one decoding is marked by its request id so its
upload is skipped and a later request for the same name still loads),
`take_next`/`finish_decode` for an executor that decodes between them (or
`decode_next` for a synchronous one), the loader's yield while uploads are
pending (skipped for a promoted request; pausing only stops the wake-ups),
`load_now` for the priority path (promote, decode and drain every ready
upload, as the original's pump does; no budget, because the original has
none), and `pump_uploads(budget, upload)` on the main thread with a
per-upload completion callback. The pass wakes the loader after it uploads.
The `upload_budget` is Exarust's addition for bounded callers (unbounded with
`usize::MAX`).

`texture_stream` runs that loader on Bevy's tasks. `AssetsPlugin` turns the
manager's streaming path on (`TextureCache::set_streaming`): a new record
gets the original's 1×1 grey placeholder under its handle and a queued load
(`ModelLoader::texture_with_flags` returns at once), and `TextureStreamPlugin`
decodes the queue on the async compute pool, four requests in flight and
only the oldest collected, so uploads keep request order. Each upload
replaces the placeholder's image under the same handle; the plugin touches
the materials that bound the placeholder and the cube shadow cuts rebuild
their bind groups, so they draw the real texture from the next frame.
Consumers whose bindings the upload cannot refresh (terrain and particle
materials) load through `texture_sync`, and the parallax depth is read from
the loaded size. A BC4 image is one grey channel (`TextureRecord::grey`) that
the shaders repeat into colour for diffuse maps, apparel, race skins and the
skin scatter; emissive and unpacked specular maps are not greyed. The loader yields
while uploads wait (the original's rule; the frame uploads before it starts
new decodes), the pump drains every ready upload,
and a record freed at its last reference drops its queued load and its image
(`ModelLoader::release_texture`). `texture_now` decodes on the calling
thread (the original's synchronous path and its priority load); `wait_decodes`
blocks on the in-flight decodes for a caller that must have them. Without a
`Game` (tools) or a task pool the plugin uploads the placeholder, and a cache
built by hand without `set_streaming` keeps decoding synchronously.

`content_stream` is the stream thread's scheduler: while a level's list is
unfinished, its next placed object alone; otherwise one generic job (its
completion queued for the main thread), one requested database object and
one requested character in the same pass, and only when none of those was
queued the pending streamed character nearest the focus point (squared
distance, strict `<`, so the first of equal distances wins), else idle. A
request that builds nothing is dropped. A placed actor of the operative kind
the session's arrival word names (15 after a journey, 18 after a start or a
load), or an object whose marker record's first word is the arrival word,
becomes the player and its position the point the stream orders by; an object that cannot
be built raises the missing count and leaves no hole in the built list. Its
rings have the original's capacities (16 jobs, completions and attachments,
32 object and character requests and results, 128 streamed results);
`fetch_object` always searches the results, and when none matches reports
`Pending` while any object request is still queued (the original's null
node) and `Missing` otherwise (its scene's nil node); `fetch_character` fetches by owner and scene; `pump`
runs completions, then hands over the streamed characters to insert and the
characters to attach; `wait_for_level` returns the built object list and
missing count only when the level had a list, and `wait_for_characters`
drains attachments while the caller pumps other work. Content builders are
supplied by the caller (`ContentBuilders`), as the original's thread calls
into the database, character and item factories. The queue and loop are
implemented; the level-object chunk reader, the database object factory and
the character builders remain in their own components, and Exarust's level
loader does not drive this scheduler yet.

Materials that show wound blood (`docs/RENDERING.md`, Blood): the human
skin, painted and run-time apparel and factory items, any record selecting
dirt mode 4 with a diffuse map, and a bleeding race's body-model materials
whose names start with `s`; each binds the speckle texture `spatter01`
(`ModelLoader::spatter`).

Painting (`paint`): every one of the original's 25 procedural materials is
painted on the CPU from its shade map, masks and library textures, family by
family (cloth, silk, velvet, leather, suede, wood, steel, blade, plate,
painted steel, chain, copper alloy, copper satin, gemstone, resin, painted
planks). The colour map follows the original's sRGB chain; the surface map is
its packed intensity/gloss/metal target; the normal map is the item's normal
plus the material's and damage normals, and the specular pass measures its
creases on that painted normal map. Library textures tile by the original's
`MatTile`/`DmgTile` rules (`paint::Tiling`). A factory region
(`model::FactoryPaint`) paints each mask with its area's material (0 as
SteelGen), lifted colour bytes and finish level, the factory's condition
(damage 2/(2 − d) − 1) and the region's atlas tiling with four offsets from
seed B; apparel takes its palette colour, the material's wear weight and the
loader's condition.

Factory weapons (`weapon`): a `.fty` recipe and an item's seeds choose,
deform (scale, band, taper, bend, radial), place (by the parent's stored
node box, scaled by the parent's recipe scale, turned X, Z, Y) and paint
`Components.rpk` parts, then assemble the striking extent and the stat
bytes. Item quality, condition and stored choices are honoured.

Physique (`morph`): `ModelLoader::morphed_model` / `morphed_job` build a
character's variant of a race body or garment at its `BodyMorph` (race
stem and record physique): each node's share of the race's morph set is
transferred once per race and model (a garment piece from the frame of the
base body part its node names), then the meshes are morphed (normals and
boxes refitted) and the body parts' translations moved with their joints
before skinning; the body's parts are skinned on the morphed shape, a
garment from the unmorphed body. Weights that change nothing, and races
without a morph body, return the shared model. The controller's
registration helpers for numbered target nodes are `morph_part_ordinal`
(the name's second and third characters as a number, which
`bake_morph_parts` sorts by), `numbered_target_deltas`, `control_matches`
(a rotated control triangle against its reference, within 1e-8) and
`nearest_records` (each base point's nearest candidate, 0 by default). See
[`docs/formats/RFC.md`](../../docs/formats/RFC.md) (Morph sets).

Mesh editing (pure, no Bevy types): `mesh_edit::EditMesh` is a decoded
mesh in the original's editable form (one element index per corner and
channel, the corners of each position element in an `Incidence`), and
`EditMesh::render` converts it back to render form as the original does:
smoothing groups from the triangle flags, runs split by material, texture
seams and mirrored texture winding, then the vertex-cache order of
`vertex_cache` (bit-exact fast logarithm and exponential).
`Incidence::build_indirect` is the generic-format incidence that maps each
corner through a reference list. `EditMesh::weld` and
`EditMesh::weld_influences` are the original's welding tools: each stream's
elements sorted by their first component (the original's quicksort), values
within 0.01 cm (positions) or its texture tolerance welded, the incidence
rings spliced in the original's order, the channels compacted and the
streams sharing a channel renumbered with their owner; the character pass
also folds welded-away skin influences into the survivor. The tolerances
are converted from the original's centimetres to metres. `flip_winding`
reverses every face and `rebuild_incidence` rebuilds the master incidence.
`mesh_smooth` is the original's Loop subdivision (creases, texture folds,
weights and output order as the original's; a mesh past the original's
16-bit face limit is only refitted and converted); `ModelLoader::
smoothed_model` smooths every mesh node of a model and refits its bounds
(`exarust model … --msm`).
`polyhedra` builds the icosahedron, its cap and their subdivisions for the
effect shells, and the mind effects' UV sphere with the original's fast
sine, normals, texture coordinates, tangents and cross-strip indices.
See [`docs/formats/RFC.md`](../../docs/formats/RFC.md) (Editable meshes)
and [`docs/RENDERING.md`](../../docs/RENDERING.md) (Effect shells).

Model meshes are built per material group with the original's vertex
attributes: positions, normals, UV_0, split tangents (`tangents::split_tangents`: a per-face unweighted
tangent and bitangent accumulated per vertex, the determinant floor, the
tangent made perpendicular to the surface normal, and the inverted
handedness sign) and skin weights, with the smallest index type that fits
the group (u16 up to 65 535 vertices, then u32; Bevy has no u8 indices). A
mesh's box and bounding sphere are refitted from its positions as the
original does (`exa_formats`' `Mesh::bounds`: box centre, half extents
floored at 0.001 cm and the farthest vertex's radius), its triangles can be
regrouped by material (`Mesh::regroup_by_material`) and two meshes can be
appended (`Mesh::merge`), the render-form equivalents of the original's face
regrouping and assembly. The mesh's shadow-pass cull-front render
bit (`Mesh::shadow_cull_front`) is not carried onto the model node: no
shipped mesh sets it ([`docs/formats/RFC.md`](../../docs/formats/RFC.md),
Mesh body).

## Placement instances

An instance node in a `.rfc` stores no body: it draws the geometry, light or
emitter of the node its instance word names (1-based), at its own transform
and parent. `ModelLoader` follows instance chains to the first stored node
and copies its parts, GI points, light, emitters and shadow bias onto the
instance's model node (`instance_root`).
