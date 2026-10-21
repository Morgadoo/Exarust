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
