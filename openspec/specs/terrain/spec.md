# terrain Specification

## Purpose
Decodes the original terrains and their palettes and shows them as walkable
ground textured like the original.

## Requirements

### Requirement: Terrain decoding
The system SHALL decode every shipped terrain into tiles of heights, hole masks and painted layers, and every palette into materials, blend rules and plants, returning errors instead of panicking on malformed data.

#### Scenario: Corpus
- **WHEN** `exa-tool corpus terrain` runs over the installed game
- **THEN** all terrains and palettes parse without errors

#### Scenario: Truncated tile
- **WHEN** a terrain's tile chunk is shorter than its declared size
- **THEN** parsing fails with an error

### Requirement: Blend rules
The system SHALL resolve painted layers into palette material weights per vertex by evaluating the palette's blend rules, with leftover weight sent to the first rule.

#### Scenario: Height rule
- **WHEN** a rule of kind 0 has bounds 0 m and 2 m and a vertex lies at 1 m
- **THEN** half of the weight goes to each child

#### Scenario: Cyclic rules
- **WHEN** two rules name each other as children
- **THEN** evaluation terminates

### Requirement: Terrain in levels
The system SHALL spawn a level's terrain beside its other content, positioned on the level's tile grid of 48 m tiles, with holes left open and collision for characters.

#### Scenario: Outdoor arena
- **WHEN** `exarust play arenao3` runs
- **THEN** the player stands and walks on the terrain inside the arena

### Requirement: Height pass
The system SHALL displace terrain vertices by the palette materials' height maps, layered with the tile's opacities and blend masks, as the original's height pass does.

#### Scenario: Neutral ground over a dungeon floor
- **WHEN** neutral terrain is painted over a dungeon floor at the same height
- **THEN** the terrain rests just above the floor instead of z-fighting with it

### Requirement: Vegetation
The system SHALL grow grass and plants on terrain where the palette materials' vegetation density allows, choosing, sizing and turning each from the tile's stored seed as the original does, standing on the displaced ground.

#### Scenario: Grassland
- **WHEN** an outdoor arena with grass materials is shown
- **THEN** grass tufts and plants stand on its grassy areas and none grow on bare dirt or rock

### Requirement: Terrain shading
The system SHALL shade terrain with the original's lighting: its lights and shadows, the sun, the GI or environment ambient and reflections, Fresnel F0 0.04, and a gloss taken from the ground's colour.

#### Scenario: Light on a terrain floor
- **WHEN** a level light hangs above a terrain floor
- **THEN** the floor shows the light's pool, lit in the same units as the models around it

### Requirement: Terrain texture orientation
The system SHALL map terrain textures with u along +X and v along −Z of the world, as the original does.

#### Scenario: Cobble layout
- **WHEN** the exanima01 start room is shown at the game's camera
- **THEN** the floor's cobblestones lie in the same rows as in the original

### Requirement: Terrain composite
The system SHALL shade terrain from the original's composite rules. Gloss comes from the layers' gloss textures (roughness 1 − gloss, with no floor, a full gloss being a mirror without a highlight), the albedo is × 0.75 and the specular is masked by the red of that albedo (`min(255·r, 1)`), the normal map and the parallax view vector are taken in the frame of +X projected onto the surface and its cross with the vertex normal, and parallax comes from the bump slot's height at 0.007 per 6 m block.

#### Scenario: Dry dungeon floor
- **WHEN** a torch lights a terrain floor of cobblestones
- **THEN** the stones look matte, with soft relief, not wet

#### Scenario: A sloped tile
- **WHEN** a normal-mapped terrain slope is lit
- **THEN** its normal map's axes stay at right angles to each other and to the vertex normal

#### Scenario: Full gloss
- **WHEN** a sand layer's gloss reaches 1
- **THEN** it shows no highlight and no invalid pixel

### Requirement: Terrain texture detail
The system SHALL read terrain colour and normal layers at their base level, as the original's mip-less composites show them.

#### Scenario: Floor seen from the game's camera
- **WHEN** a cobbled terrain floor is seen from the game's camera height
- **THEN** it shows the stones' full grain, not a mip-blurred surface

### Requirement: Terrain blocks in grid levels
The system SHALL build, in a level with a tile grid, a terrain tile's 6 m block only where its grid cell carries the terrain flag (plane A `0x800000`), the grid offset by (cells − 8 × tiles)/2 halved towards zero on each axis, and SHALL give each block's mesh and plants the room of the block's centre. Without a grid every block is built.

#### Scenario: Rock between rooms
- **WHEN** `exanima01` is built
- **THEN** terrain exists on exactly its 682 flagged cells, and none of the closed cells between its rooms shows ground

#### Scenario: Outdoor arena
- **WHEN** `arenao3` is built
- **THEN** terrain covers its flagged 16 × 15 cell rectangle around the arena

### Requirement: Every terrain material layered
The system SHALL stack every palette material whose share of a tile passes 1/510, up to 16, in palette order: the first opaque, each later one covering what is below by its over opacity sharpened by its blend mask, as the original stacks all the materials a tile uses.

#### Scenario: Dirt over the black fill
- **WHEN** an exanima01 dirt room whose tile uses more than four materials is shown
- **THEN** its floor shows dirt everywhere, with no black patches where the flat black fill lies under it

#### Scenario: Shipped tiles
- **WHEN** every shipped terrain is built
- **THEN** no tile drops a material it uses

### Requirement: Terrain noise maps
The system SHALL build the three 480×480 terrain noise maps as the original's terrain renderer does: value noise of four octaves from the seeds 137, 476 and 7359, with the first map folded into its ridge once after its octaves.

#### Scenario: The first map
- **WHEN** the noise maps are built
- **THEN** the first map starts 137, 128, 117, 131 and its bytes sum to 40 696 892
- **AND** the second map starts 156, 161, 159, 167

### Requirement: Terrain block meshes
The system SHALL build each 6 m block of a terrain tile as its own mesh from one triangle patch shared by every block: 30 × 30 cells in serpentine rows (the first along +X, the next back along −X), each quad split along the diagonal from its low corner to its high corner, the patch's indices flattened into the tile's 241-vertex rows and offset to the block's first vertex. Quads whose low corner is a hole SHALL be left out.

#### Scenario: A full tile
- **WHEN** a tile outside a grid level is built
- **THEN** it has 64 block meshes, each covering only its own 6 m square

#### Scenario: The coarse patch
- **WHEN** the patch is built over 4 × 4 cells with a shift of one
- **THEN** it has two rows of two quads, each two cells wide and two rows deep

### Requirement: Terrain built off the main thread
The system SHALL build a terrain tile's heights, material masks and block meshes on a worker while the loading screen keeps drawing, and SHALL start the next tile's build while the previous one's textures load and its blocks spawn. The level built a step at a time SHALL equal the level built at once.

#### Scenario: A story level
- **WHEN** exanima01's terrain is built through the worker
- **THEN** all 20 tiles, their plants and the height query exist when the terrain stage ends

### Requirement: Terrain height query
The system SHALL answer the ground height under a world point as the original does: the tile coordinate floored toward −∞ from `width/2 + x/48 m` (NaN and out-of-range giving no tile), the tile taken from slot `tz · width + tx` anywhere in the grid's slots, the 6 m block from the fraction times eight and only where the level builds it, the height bilinear between the four vertices around the point, no ground where any of them is a hole by the byte-wise mask, and no ground at or below −100 m.

#### Scenario: A tile centre
- **WHEN** the query is asked 10 cm off the centre of each shipped exanima01 tile
- **THEN** it answers that tile's own bilinear height there

#### Scenario: Past a row's end
- **WHEN** a point lies one tile past the end of a row
- **THEN** the query answers from the next row's first tile

### Requirement: Terrain block priorities
The system SHALL rank a terrain block by round-to-even(6000 − d), d its distance in centimetres, within 60 m and 0 beyond or for NaN, and SHALL set priorities from the lowest and highest ranks of the wanted, visible blocks, found in one pass that lowers the lowest or else raises the highest.

#### Scenario: Falling ranks
- **WHEN** two wanted, visible blocks carry ranks 300 and then 100
- **THEN** the priority base is 6000 and the blocks take half their old rank plus 6000 minus their distance rank

### Requirement: Terrain composite selection
The system SHALL model the original's composite selection: the focus 10 m ahead of the camera snapped to a 6 m block centre; a composite for each block whose squared distance does not compare above 4440² cm² (NaN kept), high resolution below 1500² cm²; at most 189 blocks; 21 high and 43 low sets; and the 3 × 3 tile gather with empty placeholders.

#### Scenario: The focus
- **WHEN** the camera stands at (20, −4) m looking along −Z
- **THEN** the composite focus is (21, −15) m

#### Scenario: The cut-offs
- **WHEN** blocks lie 14.99 m, 15 m, 44.4 m and 44.41 m from the focus
- **THEN** the first is high resolution, the next two low, and the last has none

### Requirement: Vegetation scatter offsets
The system SHALL place each new point of the grass scatter at the cosine of the drawn angle times the drawn radius across and its sine times the radius along, with the engine's sine and cosine (one round-to-nearest reduction by quarter turns, single-precision polynomials).

#### Scenario: A quarter turn
- **WHEN** the drawn angle is a quarter turn
- **THEN** the new point lies straight along, not across, from its centre

### Requirement: Terrain palette versions
The system SHALL read terrain palettes of versions 2 and 3 into the original's runtime material record, a version 2 material gaining the parameters 0.5, 0.3, 0.5 and 1.0 at index 20 (its stored parameters 20–24 moving to 24–28). A level SHALL load `default.ftb` when its terrain's palette is missing, and only then.

#### Scenario: Shipped palettes
- **WHEN** every palette in `Resource.rpk` is parsed
- **THEN** `default.ftb` and `uwa1b.ftb` (version 2) give 29 parameters per material with 0.5, 0.3, 0.5, 1.0 at index 20, and the others (version 3) are read as stored

#### Scenario: Missing and broken palettes
- **WHEN** a terrain names a palette that is not in the pack
- **THEN** "Could not find <name>.ftb" is logged and `default.ftb` is used; a palette that is there but does not parse is not replaced

### Requirement: Version 1 palette upgrade
The system SHALL upgrade a version 1 palette as the original's loader does: the old textures moved to the diffuse, normal and height slots, a mask named from the diffuse name when the old height amount is above `f32::EPSILON`, the old tint times 1.4, the negated offset (1 below epsilon), parameters 20–28 zero, an old shader grass renamed to the `Trn_GrasA01` set, and three default plants.

#### Scenario: Upgraded material
- **WHEN** a version 1 material with a height amount of 0.5 and diffuse texture `Dtex` is read
- **THEN** its mask texture is `DteM`, parameters 2–5 are 3, 1, 3 and 1.5, and the palette's plants are `Trn_GrassA01`, `Trn_SPlntA01` and `Trn_BPlntA01`

### Requirement: Vegetation lookups and drawing
The system SHALL sample a plant point's density from the 240 × 240 mask at texel `round(p × 0.9958333)` (ties to even) as the mask byte times the double 1/255 less 0.2, rounded to `f32`, with −1 where the nearest vertex is a hole. Grass and small plants SHALL be lit as facing up and drawn without a vertex tint. Each 6 m chunk SHALL belong to its own block's room, have no collider, and cast shadows only above shadow quality 4.

#### Scenario: Default shadow quality
- **WHEN** an outdoor arena is shown at the shipped shadow quality 4
- **THEN** its grass and plants cast no shadows, and characters walk through them without colliding

### Requirement: Brush table and limit
The palette's rules and then its materials SHALL be inserted into a table sorted by their 16-byte names with A–Z lowered, by binary search (an equal name met by the search stops it there), and looked up by the same search. A tile SHALL take at most eleven painted brushes; at the next brush found it SHALL report "Maximum brushes exceeded." and read no further layers.

#### Scenario: Underscores
- **WHEN** "b", "_a" and "a" are inserted
- **THEN** the table reads "_a", "a", "b"

#### Scenario: Twelve brushes
- **WHEN** a tile paints twelve known brushes
- **THEN** eleven are evaluated and the limit is reported

### Requirement: Terrain height-rule scaling
Height rules (kind 0) SHALL be rescaled as bound × scale + offset in single precision, without rounding; rules of other kinds SHALL keep their bounds.

#### Scenario: A bound
- **WHEN** a height rule's bound 0.25 is scaled by 100 with offset 5
- **THEN** it is 30

### Requirement: Terrain material bytes and merge
A vertex's material weight SHALL be added to its material's byte at row·256 + column as weight × 255 rounded to nearest even, capped at 255. The used-material merge SHALL run over the 241 × 241 corner, all materials but the first and the last, each byte becoming its share of the running weight times 255, rounded to nearest even and capped at 255.

#### Scenario: A merge step
- **WHEN** a mask byte of 100 merges into a running weight of 100
- **THEN** the weight is 200 and the byte 128

### Requirement: Terrain wet floor
The system SHALL, when the palette's wet-floor source is above 0.4, shade the terrain with roughness `1 − min(G + 8c², 0.85)`, where `c = max((y + 35)·−0.03, 0)²·n.y`, G is the composited gloss and y is in world centimetres.

#### Scenario: A wet floor
- **WHEN** a sewer floor with gloss 0.2 lies at −135 cm with an upward normal
- **THEN** its gloss rises to 0.85, while ground at or above −35 cm keeps 0.2

### Requirement: Terrain composite selection order
The system SHALL sort the selected blocks stably by key: a visible blank block 0, a visible block wanting work `0xffff − (ring − priority)` (1 once the 64-bit difference reaches 0xffff), a hidden block its nearest light's distance rounded to even plus 0x10000. The walk SHALL bake every key-0 block and stop at the first non-zero key once the frame changed.

#### Scenario: One block a frame
- **WHEN** two baked, visible blocks both want work again
- **THEN** one is rebaked this frame and the other the next

#### Scenario: No high set to be had
- **WHEN** a block wants a high set, the high pool is empty and no block holds one at low resolution
- **THEN** the block keeps no set and the frame still counts as changed

### Requirement: Terrain composite set hand-out
The system SHALL hand out a low set from the pool, else from a scan down the list from a persistent cursor that ends at the first low-set holder and takes its set only when that block is hidden; and a high set from the pool, else from the last holder of a high set at low resolution, which keeps a low copy unless it is blank.

#### Scenario: A visible holder ends the scan
- **WHEN** the low pool is empty and the scan meets a visible block holding a low set before a hidden one
- **THEN** no low set is handed out

### Requirement: Terrain composite sets
The system SHALL size the composite pools from the texture detail (8192 at setting 3 and above, else 4096): the high level `floor(log2(size/8)) − 5`, the low level one below, `32 << level` texel sets with a half-size displacement target, 21 high and 43 low sets handed out from the ends of their free lists.

#### Scenario: The default pools
- **WHEN** the level build sizes the pools at the default texture detail
- **THEN** 21 sets are at level 5 (1024²) and 43 at level 4 (512²)

### Requirement: Terrain composite pass keys
The system SHALL key a composite pass as `layers | textured<<4 | masked<<8 | add<<28 | base_normal<<29` over groups of four layers, and give a node at level L (signed; (0.5, 0.5) when L ≤ 0) of a terrain with block level T the `TexPos` `(2^(L−1) − (bx >> s), 2^(L−1) − (by >> s))` with `s = (T − L) mod 32` taken as a wrapping 32-bit difference.

#### Scenario: An additive group
- **WHEN** a normal pass covers six layers
- **THEN** its second group of two layers carries the additive bit

#### Scenario: A node deeper than the block level
- **WHEN** a level-4 node at column 7, row 5 of a block-level-3 terrain takes its `TexPos`
- **THEN** the shift count is 31 and the coordinate is (8, 8) with third component 16/4800

### Requirement: Terrain tile streaming bits
The system SHALL keep each tile's streaming bits: the reader's 7, the worker's `(state & ~0x10) | 0x828`, and a frame hook that walks the tiles with bit 0x800, marks a ready, inserted tile's blocks wanted, and handles the first reload (0x1000), insert (`(state & 0x309f) == 0xf`) or upload (`(state & 0x4080) == 0x4080`), passing idle tiles.

#### Scenario: An idle tile is passed
- **WHEN** the first streaming tile is already inserted and the second is built
- **THEN** the frame hook inserts the second tile

### Requirement: Terrain wireframe overlay
The system SHALL draw every terrain block's triangle edges as lines when `EXA_TERRAIN_WIRE` is set, as the original's debug wireframe hook does.

#### Scenario: The overlay
- **WHEN** a level with terrain loads with `EXA_TERRAIN_WIRE` set
- **THEN** each block has a line-list child with three edges per triangle

### Requirement: Terrain live frame hook
The system SHALL spawn a built terrain tile hidden, mark it built only once its entity exists, show it when the frame hook inserts it, insert every tile before the level shows, and after the load run the frame hook and then the block selection once a frame, skipping both while the refresh delay counts down; an unloaded tile's blocks SHALL leave the selection and hand their sets back.

#### Scenario: Every tile shows after the load
- **WHEN** a level with terrain finishes loading
- **THEN** every tile is inserted and visible

#### Scenario: An unloaded tile
- **WHEN** the frame hook unloads a tile whose blocks hold composite sets
- **THEN** the tile is hidden, its blocks become wanted and blank, and their sets return to the pools

### Requirement: Terrain light distance
The system SHALL measure a hidden, in-reach block's light distance as the largest non-negative clearance, from 0, over the light planes the centre lies beyond: the plane value `(z + (y + x)) + w` less the support, the normal taken into the box's frame by its dot products with the box's axes, the support `(z + (y + x))` of that vector times the sign-copied half extents; the sort key SHALL be the nearest light's value (65000 with none) rounded to even plus 0x10000.

#### Scenario: A sheared box
- **WHEN** a box whose x axis is (1, 1, 0) with half extents (10, 20, 30) lies 500 cm along +X from a light box of half side 100
- **THEN** its light distance is 390

### Requirement: Terrain low-set hand-out failure
The system SHALL leave the frame unchanged when a low-resolution block gets no low set, so the walk goes on to later blocks, while a failed high-set hand-out still counts as a change.

#### Scenario: No low set
- **WHEN** the first block wanting a low set gets none and a later block has a non-zero key
- **THEN** the later block is still baked this frame

### Requirement: Terrain priority slot
The system SHALL rank only the selected blocks in the priority slot, `round(6000 − d)` within 60 m, and set the ring to half the spread of the wanted, visible blocks' old priorities plus 6000; the slot SHALL NOT run every frame, so the ring otherwise advances by one on each frame that bakes.

#### Scenario: A quiet frame
- **WHEN** a frame bakes nothing after a frame that baked
- **THEN** the ring keeps its value

### Requirement: Terrain composite pass bindings
The system SHALL bind a normal, colour, gloss or displacement pass's textured lane at unit `2·lane + 1` and masked lane at `2·lane + 2`, a mask pass's masked lane and a height pass's textured lane at `lane + 1`, the blend mask at 0, and give `TexPos` the third component `2^level / 4800`.

#### Scenario: A two-layer height pass
- **WHEN** a height pass covers two layers with height maps
- **THEN** `Tex0` binds at unit 1 and `Tex1` at unit 2

### Requirement: Terrain composite set hosting
The system SHALL, when `EXA_TERRAIN_SETS=1`, create a composite set's colour, normal, gloss and half-size displacement render targets at `32 << level` the first time the live selection holds it, at most one set a frame, and drop them when the set returns to its pool; otherwise no target is created.

#### Scenario: Allocation off by default
- **WHEN** a level with terrain runs without `EXA_TERRAIN_SETS`
- **THEN** no composite set target is created

### Requirement: Plant instance matrix
The plant baker SHALL place each vertex through the row-vector rows `(cos·s, 0, −sin·s)`, `(0, s, 0)`, `(sin·s, 0, cos·s)` and the ground point, with the engine's sine and cosine of the stored angle, summing each coordinate as `(x·r0 + y·r1) + (z·r2 + t)`. Every variant's own normals SHALL turn by the same rows without the translation and be scaled by `1/s`, since the loader marks every variant so.

#### Scenario: Angle zero and a quarter turn
- **WHEN** a small-plant vertex at +X is baked at scale 2 onto the ground point (10, 1, −3)
- **THEN** angle 0 places it at (12, 1, −3) with normal +X, and a quarter turn places it within 1e-5 of (10, 1, −5) with a normal within 1e-6 of −Z

#### Scenario: A large plant's normal
- **WHEN** a large-plant vertex with normal +X is baked with a quarter turn
- **THEN** its normal is within 1e-6 of −Z

### Requirement: Terrain normal under a plant
The terrain normal at a point in vertex steps SHALL blend the four vertex normals of its truncated cell corner `(i, j)` with weights `fx·fy`, `(1−fx)·fy` for `(i+1, j)`, `(1−fy)·fx` for `(i, j+1)` and `(1−fy)(1−fx)` for `(i+1, j+1)`, in that order, and normalise by the approximate reciprocal square root (a zero sum stays zero); a point past the grid SHALL give +Y.

#### Scenario: Blended corners
- **WHEN** a 3 × 3 grid holds +X at (0, 0), +Z at (1, 0) and +Y elsewhere and is sampled at (0.75, 0.5)
- **THEN** the sum is (0.375, 0.5, 0.125) normalised by its reciprocal square root

### Requirement: Vegetation density threshold in double
Nothing SHALL grow at a point whose density, widened to double, is below the double 0.35; a NaN density SHALL take the growing branch.

#### Scenario: The f32 nearest 0.35
- **WHEN** every point reads the f32 nearest 0.35
- **THEN** nothing grows, while the next f32 up grows plants

### Requirement: Terrain composite bake
The system SHALL, when `EXA_TERRAIN_SETS=1`, bake at most one held composite set a frame, in selection order, running the compositor's normal, colour, gloss and displacement passes in that order into the set's targets, each a full-target quad over the block.

#### Scenario: One bake at a time
- **WHEN** two blocks hold hosted sets and neither is baked
- **THEN** only the block first in the selection is queued, and the second is queued once the first bake is reported done

### Requirement: Terrain composite bake content
Each bake pass SHALL write the per-pixel layer stack at its texel — normal xy as xy·½ + ½, colour before the terrain's 0.75 diffuse factor, gloss, parallax height — clamped to the 8-bit target; the displacement target SHALL be half the set's side and the colour target SHALL store sRGB.

#### Scenario: Matching the per-pixel draw
- **WHEN** `arenao1` is drawn with a fixed scene seed, once from the baked sets and once per pixel
- **THEN** the two frames differ by about as much as two per-pixel runs do

### Requirement: Terrain composite draw
The system SHALL draw a block from its set only after its bake has run, reading the set at the tile-local position in centimetres times `TexPos.z` plus `TexPos.xy`, shifted by the set's height in four accumulating parallax steps of `vt.xy·0.007·(4 + 3·vt.z)·(h − ½)` and eased back by min((h₀ − h₃)·2, 1); a block that loses its set, or whose set's targets are rebuilt, SHALL return to the per-pixel draw.

#### Scenario: The composite coordinate spans the block
- **WHEN** block (2, 5) of a 48 m tile centred at (100 m, −40 m) is drawn from its set
- **THEN** its composite coordinate is (0, 0) at world (88 m, −34 m) and (1, 1) at (94 m, −28 m)

#### Scenario: A lost set
- **WHEN** a block drawing from its set no longer holds it
- **THEN** the block draws its tile's per-pixel material again

### Requirement: Terrain room light rank
The system SHALL rank a hidden, in-reach block in a room against that room's light list only — each light joining the room of the cell it stands in, then every room that owns a light passing its whole list through its portals once, in room order — leaving 65000 cm when the list is empty, and SHALL rank a block in no room, or in a level without rooms, against every light.

#### Scenario: A room without lights
- **WHEN** a hidden block's room lists no light though a light in another room overlaps it
- **THEN** its light distance is 65000

### Requirement: Terrain pass mask window by node level
Each compositor pass SHALL upload and sample a mask window of `256 >> L` texels holding `240 >> L` cells, starting at texel `(by·256 + bx)·30`, where `L` is the level of the node the pass draws: the block level 3 for the composite set passes, whatever the set's level, and the tile's root node (0) for the tile's mask and height passes.

#### Scenario: A low set's block
- **WHEN** block (7, 5) is baked into a level-4 set
- **THEN** its mask window starts at texel 38 610, is 32 texels wide and holds 30 cells

#### Scenario: The tile's height pass
- **WHEN** a tile's height pass is planned
- **THEN** it samples a 256-texel window out to 241/256 and its vertices reach 241/244 of a 244² viewport

### Requirement: Terrain set copy draws
A set copy SHALL draw one quad per destination target the features keep (colour always), sampling the source's same target at unit 0 with the vertex and texcoord at the same 0/1 corner. When the selection gives a non-blank holder's high set to another block and hands the holder a low set, the system SHALL record a copy from the holder's high set into that low set and, where the holder draws its finished high set, SHALL blit it before the taker's bake into the high set.

#### Scenario: Normal off
- **WHEN** a set without the normal feature is copied
- **THEN** no normal-target draw is planned

#### Scenario: The high pool runs out
- **WHEN** twenty-one blocks hold every high set and the focus moves onto a twenty-second block that wants one
- **THEN** the last holder takes a low set, state 0x500, and one copy is recorded from its old high set into that low set

#### Scenario: An unbaked holder
- **WHEN** the holder never drew its high set
- **THEN** no blit is queued and the holder's low set is rebaked from its layers

### Requirement: Terrain extension deactivation order
The static terrain's deactivation SHALL run only while the terrain is active and SHALL deactivate its extensions in attach order.

#### Scenario: Two extensions
- **WHEN** a terrain with extensions attached as grass then a second one deactivates
- **THEN** grass deactivates first, and a second deactivation does nothing

### Requirement: Plant ground normal correction
The plant ground query SHALL take the point relative to the cell corner's right neighbour, test `(−x + z)·0.05 − cross.y·0.0025` (with `cross` the relative point crossed with the corner-to-neighbour edge) against 1, use the far diagonal when the test is above 1 or NaN, and add (far) or subtract (near) `0.0025·dot(other edge, cross)` to the height, summing the dot as `(x + y) + z`.

#### Scenario: Both diagonals on a flat block
- **WHEN** a flat 3 × 3 block of 20 cm cells is queried at (0.5, 0.5) with the point 5 cm above the plane at z = 20 and at z = −20
- **THEN** the tests are 2 and −2 and both corrected heights are 0

### Requirement: Grass hook chunk tests
The grass main draw SHALL draw a chunk only when its block word has `0x80`, the view mask meets the chunk node's mask, the node word has no `0x4` and the chunk has instances; the sun-shadow draw SHALL need block `0x80`, instances and the node mask's `0x80000000`; the omni-shadow draw SHALL need block `0x10000` and `0x20000`, instances and the light's mask meeting the chunk's room mask.

#### Scenario: Sun shadow bit on the mask word
- **WHEN** a visible block's chunk with instances has node mask `0x80000000`
- **THEN** it is drawn into the sun's shadow map, and with mask 0 it is not

### Requirement: Plant ground height
Each plant SHALL stand at the height the plant ground query returns for its cell's four vertices, in centimetres on the original's 20 cm cells; the query's diagonal test SHALL pick the plane even where it is not the triangle under the point.

#### Scenario: Raised corner
- **WHEN** a 20 cm cell has only its first corner raised 1 m
- **THEN** the point (0.6, 0.2) stands at 0.2 m and the point (0.3, 0.3) at 0 m

### Requirement: Vegetation scatter grid
The grass point set's draws SHALL be the 32-bit step widened to double times 2⁻³², narrowed once. Its neighbour grid's cell side SHALL be the radius times the original's double for 1/√2, narrowed once; the grid side SHALL be the tile side over the cell rounded to nearest even plus one, and a point's cell each coordinate over the cell rounded to nearest even. A cell SHALL list at most four points, and retiring an active point SHALL move the list's last point into its slot.

#### Scenario: The cell side
- **WHEN** the radius is 1.3
- **THEN** the cell side's bits are 0x3F6B533C and the grid is 262 cells a side

#### Scenario: A half-cell coordinate
- **WHEN** a point lies at 2.5 and 3.5 cells
- **THEN** its cell is (2, 4)

### Requirement: Terrain rule walk
At a rule, an ordered input at or below the low bound SHALL send the whole weight to the below child, one at or above the high bound to the above child, and any other input (in range or NaN) SHALL split it, the above child taking w·t first and the below child (1 − t)·w.

#### Scenario: Outside the bounds
- **WHEN** a rule's input is below its low bound and its above child is a rule with a NaN input
- **THEN** the whole weight reaches the below child and no total turns NaN

### Requirement: Terrain layer remainder
A painted layer's weight SHALL be its byte times the single-precision 1/255. A running remainder SHALL start at 1 and lose each painted layer's weight in turn; each layer and then the remainder (sent to the first rule) SHALL be walked only when its weight widened to double is above the double 0.001.

#### Scenario: Remainder order
- **WHEN** painted bytes 12 and 25 are walked
- **THEN** the remainder is (1 − w₀) − w₁ in single precision, 0.85490197

### Requirement: Terrain used materials and covers
Each leaf SHALL add ROUND(w·255) to its material's byte, comparing the low 32 bits of the 64-bit sum as a signed integer with 255. The tile SHALL use, in palette order, material 0 and every material whose summed leaf weight is above 1/510; their merged channel bytes SHALL be the tile's covers.

#### Scenario: Half a byte
- **WHEN** a material's summed leaf weight is exactly 1/510
- **THEN** it is not used

#### Scenario: A NaN leaf
- **WHEN** a leaf's weight is NaN
- **THEN** its material's byte is unchanged
