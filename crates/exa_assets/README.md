# `exa_assets`

The bridge between `exa_rpk`/`exa_formats` and Bevy: the `Game` resource
holding `GameData`, conversion of parsed data into Bevy assets, and
installation discovery (`--game`, `EXANIMA_DIR`, `.env`).

**Phase:** 0 (started), 2 (asset source `exanima://`, loaders, direct BC upload, streaming).

Material bookkeeping (`material_manager`): the original's per-scene
material manager as a generic table (own provider in slot 0 with the global
plug-in providers behind it, slot reuse from a search hint, the loaded and
waiting-to-free lists, the `_Default` slot, get-or-load with the fallback to
`_Default`, release with the negative-count error). It is not driven by the
loader yet; `ModelLoader` shades an unknown material name with the built-in
`_Default` record (`exa_formats::material::Material::builtin_default`), one
shared handle for all such names.

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
build on the compute pool; a mesh that ships weights keeps them. The
builder's group stack (sixteen frames, an empty frame for a node without
groups, a cleared top frame that keeps its depth) is `skin::GroupStack`;
`skin::auto_skin_with_context` matches a garment's groups against an
enclosing part's joints as the stack's reader does, and the per-node model
build passes no context.
`model::underscore_skinned_child` and `model::skinned_mesh_under` are the
wound and effect lookups (the `_`-named child, then the deepest skinned
mesh in reverse child order).

Power tree sheets (`Game::load_tree_sheets`): the domain icon sheets, the
frames, mods and flamelets, and the domain wheel, which the tree's load
resamples to 32 GUI units a side through `exa_render`'s resample-to-size
path in its own format word and flags (`wheel_texture`; 384 → 128 pixels at
the four-pixel unit); `TreeSheets::resample_wheel` does it again when the
unit changes.

Hair (`hair`): the original's hair colour picker (five decoded hue anchors,
desaturation by the saturated value, a specular grey moved in, the engine's
sRGB chain), with the feral level's lightening, the material's gloss
0.9 − 0.4·f and opacity (1 − 0.65·f)², each mixed-precision step (a value
widened to double and narrowed) as the instructions take it. The character
crate's `hair` system tints the hair materials with it.

Merging (`merge`): `merge` appends the visible body parts and garment pieces
into one skinned mesh as the original's character assembly does: each part
placed by its frame, its vertices, normals, UVs and colours appended, its
weights kept vertex for vertex (a part without them bound to its bone), its
triangles grouped by material in first-seen order, and each mesh's own
position-stream ids kept and moved past the meshes before it (groups of one
mesh share its stream). The final renumber pass is the original's: each
material's vertices in order, each run starting right after the previous
one's last vertex. A part's `vertex_masks` and `cull` drop the vertices
whose slot mask meets the cull bits and the triangles that needed them; the
dressed-mesh pass passes a slot's cull bits (`0x200000`/`0x400000`,
`exa_character::layers::cull_bits`) as that argument, and the append writes
nothing on the triangles. A part whose material has the occlusion mark
(`occlude`) fades the vertices on the border of the marked region
(`occlusion_alpha`). `culled_indices` drops triangles by per-triangle flag
bits (which triangle flags the original's draw tests is not traced).
`Merged::subobjects` states each material run's vertex and index ranges,
and `model::scene_merged_mesh` merges a scene's skinned nodes group by group
from their body-space frames. The merge order and the per-slot cull bits
come from the generator (`exa_character::dress` and `layers::cull_bits`).
The running game merges through `model::merge_dressed`: the caller gives
each built material group with its body-space frame, weight bone and cull
bits, the built mesh is turned back into file streams
(`positions`, `normals`, UVs, joint indices and weights), and each merged
material run becomes a drawable sub-mesh (`model::run_mesh`) bound to the
character's skeleton with the root part as joint 0
(`exa_character::dress::assemble` builds the plan). The per-node draw stays
as the fallback for a character without its body model. The built meshes
keep no vertex slot masks and no triangle flags, so on shipped data the
cull argument and the occlusion fade do nothing; both remain tested rules.

The editable-form assembly is `merge::merge_editable`: the original's merge
read instruction by instruction. Groups join by material (the destination's
face count grows, a new group carries the joined count and no split flags),
streams by their whole format word, channels by their low key; a channel's
entries move past the largest destination element count over the streams that
share it, so a part's positions, colours and weights start together, and a
gap is filled with zero or 255. Only the position stream is placed (by
the rigid `destination⁻¹ · source`, in the original's float order) and the
normal stream (low key 2) is skipped, as the original does. The face → group map the
merge keeps is `EditMesh::face_group`; `merge::regroup_by_material` sorts the
faces into material runs and permutes every channel, the flags, the edge bits
and the extra per-face word (`EditMesh::face_extra`), then drops the map.
`EditMesh::duplicate` is the original's mesh copy: every array is its own.
`merge::assemble_editable` appends several parts into the first one's space
and regroups once, the sequence every original caller runs.

Skin composite (`skin_composite`): the original's generated skin maps.
`colour_texel` is `chargen_d`'s colour times the tone, `specular_texel`
`chargen_s`'s packed intensity/gloss/metal, and `normal_texel`
`chargen_n`'s base plus each physique layer's `(an − bn)·weight`. The
beard's three overlay programs are `beard_colour` (squared alpha, mip-3
alpha, the 0.1 mix and the 1.5 alpha), `beard_normal` and
`beard_specular` (`a³·0.4`). `compose` runs the whole job per texel into
the sRGB colour target and linear surface and normal targets, its rows
split over the machine's threads (up to eight; the same texels on every
machine). The race maps are read unshifted: the look seed's two offsets
(`UVOfs`) belong to the decomposition programs only (`decomp`). `SkinCompositeJob` is
the job as data: `ModelLoader::composited_skin` decodes the race, layer and
beard maps on the CPU, composes and uploads the three maps as an
`ExaMaterial` (white diffuse, packed surface, normal, subsurface, blood),
cached by the job's key (stem, tone, side, beard and layers);
`skin_compose_inputs`, `SkinComposeInputs::generate` (pure, for a task
thread) and `finish_composited_skin` split it for the async dressing pass.
`run_mesh` rebuilds each run's tangents with the splitter, as every drawn
group's are. The running game feeds it to each character's merged skin run; a job whose
textures do not decode falls back to the race maps plus tone in
`human_skin`.

Skin tone (`human_skin`): a character's `SkinLook` selects the built-in skin
material for its record's two tone words (and the women's texture set),
cached per race, tone and sex. The tone multiplier is the original's, bit
for bit: its HSV tone table, grid halves and fast power. The creator's
picker grid is `tone_grid_linear` (nine linear colours), the grid the
unit's init precomputes; the three scales it derives after the grid (the
darkest column over the base skin's linear colour, divided in single
precision and times 0.8 in double) are `tone_blend_target`.

The decomposition pass (`decomp`): the original's `chardecomp_d`, `_s`
and `_n` programs evaluated per texel for a corpse's or undead's skin —
the skin scaling and decay mixes, the green specular sheen and the decay
normals with the height-map cavity normal — from shader texels, the skin
colour and the amount; `DIFFUSE_MAPS`, `SPECULAR_MAPS` and `NORMAL_MAPS`
name the maps the passes bind (read from the unit's binding loops).

The character-creation sliders (`morph_controller`): a centred slider reads
its position as `(position − 0.5) · −1`, a direct one as itself; `morph_slider`
writes that float into one morph target's weight and returns it for the
caller's apply, and refuses an index past the targets instead of writing
past the array.

Model nodes carry their rigid body with the physical divisor of their first
mesh part's material (`ModelNode::rigid`, `NodeRigidBody::mobility`), what
the solver registers a prop with. A node with a body also keeps its mesh
triangles in the node's frame (metres, `ModelNode::triangles` from
`model::mesh_triangles`: the file's index list over its positions, a
trailing index that starts no triangle ignored, an index past the positions
giving none), which registration hands the solver as the collision mesh the
pair stage tests the other side's spheres against. A node's `_null` groups
are geometry the original never draws but its cursor pick meets (the hub's
shop counter is a `_null` box): they are built into `ModelNode::collision`
as bare position and index meshes kept in the main world (never uploaded;
none for a skinned mesh), spawned hidden as pick-only colliders, and do not
appear among the node's drawn parts. A cloth-marked node also keeps what its
cloth is built from (`ModelNode::cloth_mesh`, `ClothMesh`): the mesh's
position elements, its triangles over them with their face flags (sub-chunk
`0x3DC3`, whose top bits mark the pinned faces) and, for each drawn part,
the element of each of its vertices, so the cloth makes one particle per
element as the original does (a seam does not split it).

Object libraries: `ModelLoader::object_pack` searches the project's object
libraries in their order and `object_model` loads the first match, as the
original's `TObjLibManager` does; the master model is cached and every
placement shares its meshes and materials. A node flagged `0x8000` is a
placeholder: the master keeps its data, but a spawned clone draws nothing
and registers no body at its transform (`ModelNode::placeholder`), as the
original's clone replaces it with a plain node. No shipped model carries
the flag. The worker unit's weighted three-plane sum is
`noise::weighted_planes` (not called yet).

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

A height layer is switched off by its texture: a map one texel tall clears
the material word's `0x800000` and the shader word's displacement bit, and
stores zero parallax counts; a taller map keeps the layer and gets its
texel counts. Only the height takes part in the test, so a one-texel-wide
map still displaces. The parallax counts, the displacement step and the
bias are widened to double, scaled and narrowed, as the writer's
instructions order them.

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
3D and `0x40000000` the cube. A cube map uploads as six faces, each from the
detail level the word keeps with its levels back to back, and a cube view;
the three shipped cubes (`genref01r`, `genref02r`, `genref02rm`) take that
path. A block-compressed cube is refused (none ships). No shipped image asks
for 1D or 3D; those still upload as 2D. The seven uncompressed formats the original
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
bias**, so each material carries it in its parameters and the shader adds it;
a settings change rewrites every loaded material's bias
(`exa_render::material::set_texture_bias`), as the original sets it on every
bind. The GL texture-unit and
render-target texture calls (selecting a unit, binding, creating an empty
texture for a framebuffer attachment) have no Exarust item: Bevy owns render
targets and binds per draw.

`TextureUnitCache` is the original's per-unit bind cache: the texture object
already on a unit (compared by identity, not by GL name) is not bound again; a
different one makes its unit active when it is not (the active unit starts at
0), binds its target and name, and sets the target's LOD bias parameter to
zero unless the word has bit `0x200`. Binding nothing, or an object with no
GL name, only clears the unit's entry. The original has no sampler object
here, and the cache is not wired: Bevy binds per draw.

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
`image_format` is the loader's reader dispatch: a borrowed name goes first to
a registered provider (Exarust's pack lookup is that provider) and then to
the file reader its extension names. `.rfi` is the only reader registered; a
name with another extension (or an empty one: a trailing dot, or more than
the original's 16 bytes) reports the original's "Unrecognised file
extension: …" message, titled "File format error"; a bare name stands for a
provider's image, and one whose bytes carry no image magic fails like the
provider's reader. Both paths end in the grey placeholder, as the
original's failed load does. `texture::decode_entry` runs that dispatch
before the `.rfi` decoder; the streaming task and the material loader use
it.

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
texture loader's queue: `request` (a duplicate of a queued name is refused).
A request made through the manager also keeps the pack and exact entry the
image was found in (`TextureRequest::from_entry`, the original's provider and
image indices), so a later decode cannot drift to another pack;
`promote` (which finds a request queued or held by the loader, decoding it
or waiting with it for the upload pass; one already in the upload list or
uploaded is no longer found, as the loader clears its status bits when it
goes on), `cancel` (a queued request or a
decoded one is dropped, one decoding is marked by its request id so its
upload is skipped and a later request for the same name still loads),
`take_next`/`finish_decode` for an executor that decodes between them (or
`decode_next` for a synchronous one), the loader's wait (`wants_to_yield`:
it decoded a request while earlier uploads were queued and the request was
not promoted, so it starts no other decode until an upload pass; pausing
only stops the wake-ups),
`load_now` for the priority path (promote, decode and drain every ready
upload, as the original's pump does; no budget, because the original has
none), and `pump_uploads(budget, upload)` on the main thread with a
per-upload completion callback. The pass wakes the loader unless it is idle
with an empty ring.
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
Consumers that need the real image before they bind it (the parallax
depth, read from the loaded size) load through `texture_sync`. A BC4 image is one grey channel (`TextureRecord::grey`) that
the shaders repeat into colour for diffuse, emissive and unpacked specular
maps, apparel, race skins and the skin scatter. The loader waits for
the upload pass only when it decoded a request behind waiting uploads; the
frame uploads before it starts new decodes, so that wait never idles it, the
pump drains every ready upload,
and a record freed at its last reference drops its queued load and its image
(`ModelLoader::release_texture`). `texture_now` decodes on the calling
thread (the original's synchronous path and its priority load); `wait_decodes`
blocks on the in-flight decodes for a caller that must have them. The
terrain material path's two routines are `texture_existing` (the manager
lookup that takes a reference and creates nothing) and `texture_stream_first`
(a live record first, else the placeholder and a queued load whatever the
general streaming switch says, always with status 1; a name the game does
not ship gives nothing). The terrain build asks for each palette material's
diffuse and normal map this way as soon as a tile names the material and
spawns the tile only once `texture_waiting` says both arrived, so its splat
material and composite bake never bind a placeholder and the decodes run on
the loader rather than the load step; a caller that waits for the whole
level in one call finishes a waiting record on the spot with
`texture_finish_now` (the priority wait: the record's own word, no new
reference). `add_texture_image` adds an image the caller already has under
a name (the manager's named-image inserts): a name already present keeps its
live record with one more reference, and the queued insert also logs
"Added texture already exists." under "Texture management";
`add_texture_file` reads and decodes the named file first, adding nothing
for a file it cannot read unless the insert asks for the grey placeholder.
The manager's fast-load flag (`TextureCache::set_fast_load`)
makes the material path's queued load the loader's priority item (status 3
rather than 1, so its decode never waits for the upload pass); the shipped
game reads the flag and never sets it, so it is off. Without a
`Game` (tools) or a task pool the plugin uploads the placeholder, and a cache
built by hand without `set_streaming` keeps decoding synchronously.

`content_stream` is the stream thread's scheduler: while a level's list is
unfinished, its next placed object alone; otherwise one generic job (its
completion queued for the main thread), one requested database object and
one requested character in the same pass, and only when none of those was
queued the pending streamed character nearest the focus point (squared
distance, strict `<`, so the first of equal distances wins), else idle. A
request that builds nothing is dropped. `StreamThread::new` holds the
thread's two empty data streams, as the original's constructor creates
them (unused so far); the loop above runs on the executor that drives it. A placed actor of the operative kind
the session's arrival word names (15 after a journey, 18 after a start or a
load), or a placed object of the marker class whose record (type byte
`0x40`) holds the session's arrival variable as its entrance word and the
id of the level the session came from as its second, becomes the player and
its position the point the stream orders by; an object that cannot
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
the character builders remain in their own components. Exarust's level
loader drives the level pass: `exa_world::LevelLoad` builds its placed
objects through `step`, one entry a pass in file order, names the player by
the session's two words, and ends with `wait_for_level`, logging
`missing_message` ("N missing database objects.") under `LOG_SOURCE`
("Content management") when the count is not zero. `LevelHandoff::of` is the
wait's choice of where the finished objects go: to the scene's grid
environment, which sorts them into its rooms, when the scene has one and its
room-mode word has either bit of `0x3` (every shipped scene is created with
`0x73`), else appended to the caller's list after what it holds. The
request rings and the streamed characters have no driver yet: the
character spawner and the database object requests build directly. `saved_chunks` reads the saved
sub-chunk blob a streamed character carries (the builder scans from byte 4
for the flat pose, motion state, pose tree and AI home while the next record
header lies inside the stored payload length, keeping the last of a repeated
tag), and `stored_size_scale` is the
character builders' size rule: the stored 16-bit word times the double
constant 1/65535, times the body's base scale, narrowed to `f32` last; the
streamed builder first defaults a pair whose second word is zero to
`0xFFFF`/`0xFFFF` unless the body is dead (`streamed_size_words`), the
request builder scales the words as stored.
`root_node_flags` is the placed-object root rule: only the object's flags
under `0xB80000` replace that field of the node word, and the placement's
bit `0x10` adds the node's static bit.

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
seed B. An area's emblem adds a colour-only layer over the same mask
through its `emblembkg` picture placed by the factory's `PtrnPos`
(`paint::Pattern`), and blade layers sample their mask, shade and region
normal stretched by the part's `UVScale` (`Tiling::uv_scale`); apparel takes its palette colour, the material's wear weight and the
loader's condition. The region quad's width crop (`1 − texture_parameter`)
scales the source textures' u (`Tiling::crop`), and below texture detail 3
a job paints at half the region's size while its tiles double
(`paint::paint_target_size`); `paint::RegionDraw::uniforms` reproduces the
`MatTile`, `DmgTile`, `ShdTile`, `MatCol`, `Damage`, `Dirt`, `Quality`,
`UVScale` and `PtrnPos` values the original's six passes bind.

The painter's GPU-side bookkeeping is modelled in `paint_jobs`: the
256-slot job ring (a full ring flushes all but the newly pushed job), the
64-slot surface cache with its 4-slot extra-layer pool and the original's
cursor arithmetic, the per-size shade scratch targets and the render-target
formats. Exarust paints on the CPU and on Bevy's task pool, so these queues
have no GL thread; their index arithmetic and formats are pinned by tests.

The procedural material's compiled program variants are
`exa_formats::procedural::programs`: family names, pattern/alternate-normal
defines, the sampler counts and the uniform lists each pass fetches.

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
and copies its parts, GI points and GI flags (centred, thin), light, emitters and shadow bias onto the
instance's model node (`instance_root`).
