# `exa_world`

Levels in the Bevy world.

**Phase:** 2. Status: props, level scene nodes and lights spawn
(`spawn_level`, `spawn_model`); walls and floors from the tile grid, terrain
and the grid's rooms; portal clipping is next (see
[`docs/PLAN.md`](../../docs/PLAN.md)).

- `spawn_model(commands, model, placement)`: a cached model with its node hierarchy, its lights, fires and voxel lights (`exa_render::gi::VoxelLight`) and its nodes' keyframe tracks (`exa_anim::tracks::NodeTracks`); a placement replaces the root transform, as level props do. Placed items dim their voxel lights and glow meshes by their condition byte. `spawn_model_nodes` also returns the node entities.
- `db_objects`: the object manager's scene decisions: the scene tree (a model, a factory weapon, the invisible container volume
  with its radius, or the 50 cm placeholder of an operative record without
  a model name), the managed node flags (`0x4000`, children `0x5000` and
  the parent's hidden bit), the door and operative kind words, the
  material rules (a weapon record hides `*`, an item-body record hides
  `*` and paints `&`), the impact-sound class, masters kept per model
  unless painted, and the release that logs a mismatch and goes on. Only
  the placeholder is spawned; the rest is not wired.
- `bodies`: which rigid bodies of a level register in the solver, as the original registers them (each body tree from its top body down through its body children), and their registration: the free and hung trees of props not placed static (`Prop::STATIC`; nodes marked static, `rfc::Node::STATIC`, never register), of placed objects that are no door, lever or push surface, and of the level's own model, asleep; a hung tree's root keeps its stored links as world links anchored in its parent node (`exa_physics::body::WorldLink`), an origin pivot unless the first is a slider, and no level detection; mechanisms stay static until their steps drive their bodies (`LevelInfo::bodies` counts each). A body's stored scrape and roll sound slots travel with its registration (`PendingScrapes`) and become `Ragdoll` sources on their parts; `exa_audio` does not play them yet.
- `mechanisms`: the door and operative records a placed object carries (`MechanismRecords`, under its tagged placed id), converted for `exa_physics`'s door, lever, plate and signal rules; a signal can be dispatched by them. Nothing ticks a mechanism yet: its body stays static level geometry; the hung props' world links are in the solver.
- `player_markers`: a level's transition markers, read from the volume sections of its placed objects (`exa_formats::objdb::Volume`): the entrance, the id of the level they lead back from and their world position. 54 ship across the levels; `exanima01` has the box that leads back from `exanima02`.
- `spawn_level(commands, loader, name)`: the level's own meshes and omni lights plus every prop from `Objlib.rpk`, with the placement's node overrides (a named node's transform or mesh positions, `exa_formats::level::PropAttachment`) applied before its bodies register; placed objects take the same overrides.
- `LevelLoad`: the same level built a step at a time within a time budget per step (grid, terrain, level model, props, placed objects, lights), for a loading screen.
- `level_lifecycle`: the loaded level's lifetime. Every root a level owns (the level root with its props, grid and terrain, each placed character, each generated scene body, the room shells and the sky) carries `LevelEntity`. A `LevelChangeRequest` despawns those subtrees, empties the physics world (`exa_physics::Physics::clear`), removes the old capsules so the fixed step makes new ones, and drops the level's GI blocks and textures, navigation grid, room and wall resources, terrain ground, fitted sun map, sky and fog. The host then loads the destination through `LevelLoad`; `LevelChangeReady` carries its name and spawn. The player, camera and interface are not level entities, so the session state survives.
- `PlacedCharacter`: a level's placed character resolved for spawning (position, facing, dead or alive, race, worn and held items) with its whole record, from which play mode derives its stats, and the encumbrance its worn items and held shield give it (the level's or the game's items). A dead placement also carries its saved pose (a bone tree or the legacy flat frames) and a flag for the command line's synthetic corpse, the only dead body built standing and left to fall. A world-database character is kept only in its own locale (`locales.rdb`), as the original creates them.
- `teleporters`: the teleporter and reconstruction device rules. A device's first `[` node is its crystal slot; a scene object within 1 cm of it is what it holds (NaN is never near), and a use needs the player within 100 cm of the point 16 cm above the device (NaN passes, as the original's unordered compare does) and no possession of another body. The teleporter takes a Power Crystal A (431, below 0x55 spent) or its large-crystal adapter (1423), compared by the whole record id, and lays the destination screen's rows out as the original walks them (a zero entry, or one past the free rows, goes to row 0; the shipped 7, 10, 12, 13 show as 13, 7, 10, 12; flag bit 2 shows one row) with the `prtlsmbla.rfi` atlas. The reconstruction device takes a Power Crystal L (500) whatever its charge, starts the scan 1600 ms later with the 3.0 flash, glows over a 1500 ms rise, 3800 ms hold and 1500 ms fall (`pulse_step`, in double precision) and raises the reconstruction latch with the player record's identity. `session_end` turns a story death (red below 0.01 in double) of that identity into a reconstruction, stripping only the first time: an unshared record is copied first and the dressed copy (one use, mark 0x40, identity 0, roles 1 and 0xC2, no health) is the corpse, while the player keeps the stripped record (hands and worn list cleared, one empty action, health and stamina 0x7FFF of 0xFFFF). An object let go within 5 cm² of the slot point powers the device down. The destination ids are the locale list's (7, 10, 12 and 13 are Exanima05, 07, 08 and 67); `rooms::enter_level` turns a chosen id into the level it loads. Placed objects carry `PlacedItem` (record id and placement condition) and the two devices `TeleportDevice`; `exa_game::teleporters` drives them.
- `rooms`: the grid's rooms and door links (the original's flood fill), the cell a point stands in (`cell_of`, and `cell_of_node` clamped to the grid as nodes are sorted), `open_side` (the flood's "open and not a wall" side test), each link's portal (plane B's side nibble and plane C's special type, half widths and tops of 82/245, 115/290 and 200/340 cm, from −50 cm up), the door a placed node stands on, the original's light reach through open doors (2 m from the ¾ door point), its control-side door query (no room checks, so two points in no room reach; otherwise the first link of the first point's room into the second point's cell decides, with no distance), its light spill (a light within 2 m of its cell's centre reaches the rooms behind that cell's open doors), `RoomLights` (the room manager's per-room light aliases; a room that owns a light passes its whole list, deduplicated, to the rooms its portals lead into, in room order; modelled, not read by the renderer), and `RoomTracker`, the original's active, adjacent and hidden rooms around an anchor with their 1 s fades, its reset and its active-room flag (faded in, or wanted again mid fade-out).
- `scene_nodes`: the original's scene-node unit as a pure model, not yet on the draw path (`room_view` still draws from the room states). The scene is its own first sector; nodes outside every scene hang from `_World` (owner `None`). Registration by kind (meshes, lights unless world lights, auxiliary nodes per sector; geometry per scene; the owner always; `IN_SCENE` only for the listed kinds), batch insertion (leading sectors appended, the first other entry registers the whole batch in its owner), locked sectors, change bits and the membership rebuild, subtree moves (local position plus the old origin less the new), child validation (the last child fills an empty slot), frames in sector space, a sector's two mesh boxes (seeded at the first split mesh, the second carrying on from the first, extents capped at each radius, the union radius about the world centre) and the scene's bounds written into its own sector, the 6 m cell and spill mask (one byte, `1/600` multiplier, −1 cm/601 cm; the cell handler runs when the cell changes), and the per-frame visibility: the scene sector's full mask, active sectors against the main views, adjacent sectors through their doorways to active sectors (clipped by the first main view, the scissor from the faces' first vertices rounded ties-to-even), the hidden pass (no view bit and no such doorway), the 20 cm doorway test against the sector beyond, the inherited frame bits and the cut-away fade (0.003 per ms; 0.001 and 0.999 in double precision). Its cut-away detector (`detect_cut_away`) probes the meshes of the sectors the first view draws, in each sector's space: the camera's segment (from beyond the eye to a metre short of the player) against each candidate's bounding sphere, or a 10 m segment straight down onto the player for an overhead one, by the kind bits of the scene word; marks carry to the parent and the mesh children, and a miss clears a marked mesh. It shares [`scene_nodes::cell_spill`] with `light_flood`.
- `light_flood`: a directional light fills `width²` cells with an empty window (cells that list it already keep theirs); the mesh cell and spill test is the scene-node unit's `cell_spill`.
- `room_view`: the rooms in the Bevy world: `LevelRooms`, `RoomAnchor` (the player, or 20 m ahead of the fly camera), `InRoom` on meshes, rooms of props and characters, and the rooms' black shells (`RoomShell`: occluder, lid, fade overlay) shown by state; `RoomView { all }` shows every room.
- `grid`: walls and floors from the wall and floor sets, and each room's occluder and lid merged from the pieces' `[…` and `+R…` strips and the ceiling slots (never spawned as pieces); ceiling caps. Special tile masks turn with the cell, a cell past the last piece of its type reuses it (and one with none of its type takes the set's first), types 6–9 keep their centre floor, a floor set named `Terrain` is not loaded, and pieces and characters look their room up by the cell they stand in. `Strip::ray_hits` is the engine's one-sided mesh ray test (front faces only, determinant at least the f32 epsilon, 0 < t < limit), and `grid::ray_blocks` / `segment_blocks` walk the active rooms' occluder or lid shells in order, answering no at the first active room that has none, as the room manager's queries do (their picking and segment callers are not ported).
- `grid_sets`: the builder's set catalog: the wall and floor set names registered once each (whole-name comparison, the original's 16-byte name), the loaded flag, and the `Terrain` floor set marked `0x7E01` instead of loaded by the load pass, so it never brings floor pieces (the original's floor registration loads a new set at once, before that pass; see `docs/formats/RFC.md`). `GridBuild` registers every name it loads and indexes a set shared by name once.
- `scene_env`: the original's scene environment as a pure model: the sun and moon on their 45° path and the daylight factor, the time-of-day records and their pick-and-blend, cloud dimming, the fog's span and per-frame near/far, the clear colour and sky gradient, the ambient scale, the automatic sunlight rule, the sun mode and sky switch, rain and lightning with the original's generator, and the save unit's three presets: the indoor/underground scene, the seeded outdoor day and weather, and the fog variants. `environment` turns it into the level's `Lighting` (ambient, sun, linear fog, clear colour and sky) and Bevy resources; a level load calls `level_lighting_preset`, which applies the preset the level's name selects (indoor for `exanima*` and the first seven arenas, the seeded outdoor preset for `arenao*`, the record for everything else) with a seed from the process clock, as the original seeds it when it creates the scene. The weather tick and the time of day are modelled but no frame system drives them yet.
- `terrain`: tiles, blend rules, height pass and plants; the palette's
  name-sorted brush table (the original's binary search), the
  eleven-brush limit (wired: later layers are dropped), and, not wired,
  the height-rule scaling, the byte channel accumulation, the
  four-per-plane mask packing, the Gaussian blur kernel and the
  used-material merge; the palette falls back to `default.ftb` as the original does; a tile's geometry
  and masks are built on the async compute pool (the original's terrain
  worker), one mesh per 6 m block (`terrain::patch` is the original's
  shared serpentine triangle list), each in its block's room and with the
  original's `TerrainBlock` frame and flat 10 cm slab bounds, and in a grid
  level only the blocks its grid flags; `terrain::ground` is the
  original's world height query (`TerrainGround`), built from the same
  displaced heights the navigation grid reads, with the bilinear sums in
  the instruction order (the +X corner onto the low corner, then +Z, then
  the high corner); `terrain::frame` is the
  lit program's normal-map tangent frame; `terrain::stream` holds the
  original's streaming decisions (60 m block priorities, the 44.4 m / 15 m
  composite selection and its pools, the 3 × 3 tile gather) and the tile
  streaming bits with the one-tile-a-frame hook as a tested model;
  `terrain::composite` is the original's compositor as numbers: the 21 high
  and 43 low set pools and their 1024²/512² targets (colour RGB8, normal
  RG8, gloss R8, half-size parallax R8), the four-layer pass keys
  (`AddPass`, `Textured`, `Masked`, `BaseNrm`), every per-layer value the
  passes upload, the block palette and mask quads, `TexPos`, the
  wet-floor gloss term (wired into `terrain.wesl` through the palette's
  wetness) and the selection's look point, cut-offs, light-distance sort, set stealing
  and one-block-a-frame rule; `terrain::wireframe` draws each block's
  triangle edges as lines under `EXA_TERRAIN_WIRE` (the original's debug
  wireframe hook). Grass and
  small plants take the original's upward normals, grass is drawn at 0.75
  size, the density mask is sampled on its
  240 × 240 texels, and the baked chunks have no collider and cast shadows
  only above shadow quality 4; wind is not ported.
- `voxeliser`: the GI sample-point generator's parameters: a 40 cm lattice spacing, the 4.0 and 0.4 tuning values and one work pass.
- `gi_volume`: the voxel GI occupancy block, built on the async compute pool for the block the camera is in; each grid cell's floor meshes are merged and get the original's floor heightfield points (`exa_physics::voxel`) once per level, in the cell's own space, and `GiState` tells a loading screen where it stands. `GiBlockLayout` lays the level's blocks out once (the built cells' bounding box rounded to eight-cell blocks and slid inside the grid; the synthetic 8 × 8 grid and its block at cells (3, 3) for a level without one), the camera picks the block its cell falls in, and the last three built occupancies are kept for reuse. Thin and centred GI meshes splat their quarter weight or half-voxel height; `Occupancy::update_dynamic` takes a moving mesh's old points out and puts its new ones in; `VoxelRegion`/`sub_upload` carry the dirty bytes. Every frame it marks the cells within the reach that are built and in view, then the built cells next to a marked one, in the original's two passes, and lists each sector's lights from the 6 m cells' flood lists (`exa_render::gi::GiSectorLights`).
- `point_light(light)`: the original omni light approximated as a Bevy `PointLight`.
- `light_source`: the state a light node keeps beside its record (base and current intensity, scale, the flicker term, the record's or the constructor's amplitude, speed and emissive share, the shadow-quality offset), the lever, and the once-a-frame flicker and fade at the scene's frame time with the engine's generator, in the original's double-precision steps; a flickering light drives its point light and its parent mesh's first part's emissive (on a material copy of its own).
- `node_parts`: a model node's part entities in the model's part order (the original's sub-object list).
- `particles`: the original's CPU particle emitter (`TParticleSystem`): its template record and size scaling, derived per-tick values, ring of Verlet particles, ageing (the fire's distance cap), emission with its two-thirds accumulator, light and sound walks, quad building (diamond, stretched, billboard) with the atlas variants and colour keys, and body contacts; `particle_uv_table` builds the particle unit's 128-entry UV lookup (four atlas cells of eight symmetries of four corners) and is tested against the per-particle variant arithmetic. The draws come from the engine's integer LCG and the run-time library's Mersenne twister (`Rng`, `Mt19937`), and the arithmetic follows the original's (double-precision steps, half-to-even rounding). Plain state and rules at the 83 Hz tick; no Bevy `App` needed for a test.
- `fire`: the fire presets: the six templates and three beds, the size tiers (small, medium-small, medium and large at half size), `CreateFire`'s light (including the ×1.25 radius above size 0.2 in double precision), the ring sources, burning items' sizes and the marker import.
- `flame`: the fire emitters in the Bevy world: their fixed ticks (four catch-up ticks at most), the fire's light density and softness, and each frame's particle mesh with the `_FirePrt` material, skipped while the flame is hidden; a finished one-shot fire is removed. `CarriedFire` on a model root (the carried torch) makes its fires ride the item as the original's body-attached emitters do.
- `arc`: electric arcs in the Bevy world, over `exa_render::electricity`'s rules: the jittered tendrils' camera-facing ribbon with the `_ElecMat01` material and the preset's light (a voxel light, or a device's point light). Nothing spawns an arc yet.
- `sun_shadow`: the sun's one fitted orthographic shadow map in a level with a terrain: each frame it clips every active camera's frustum (out to the original camera's 125 m far plane) at the level's box and the record's floor and ceiling on the sides the camera looks towards, fits it with `exa_render::sun_fit` (200 cm radius steps with hysteresis, texel-snapped centre, ±8000 cm depth) and writes the matrices into the sun's single Bevy cascade before Bevy derives the shadow frusta; a frame that fits nothing keeps the last map. Other levels, and `--bevy-sun-cascades`, keep Bevy's two cascades ([`docs/RENDERING.md`](../../docs/RENDERING.md), Sun shadow fit).
- `cloth`: the level's cloth-marked meshes (long hair, four banners, the wraith's robe) become a simulated cloth (`ClothInstance`) over their vertex positions, pinned at the shipped meshes' rest points with the loose 0.1 weight (the pin rule is a labelled guess), stepped once per 83 Hz body tick with gravity and written back into the mesh's positions (each cloth takes a copy of the mesh asset, since one asset is shared by every copy of a model, and a node whose mesh has no triangles is marked `ClothSkipped`); the two-face draw passes come from `cloth_draw`.
- `nav`: the original's navigation grid (30 cm cells, two layers, cost bytes), built for each level when its load ends from its terrain (the 6 m blocks the level builds), its meshes' GI sample points and its tile-grid floors, on the async compute pool (`NavGrid` resource, `NavState`, `nav::finder::InstalledPathfinder`: the level's grid or the 64 × 64 abstract fallback, whose queries answer nothing); its queries (`nav::query`: the line walk, the straight walk, the obstacle test; `nav::walkable`: a point's cost and the nearest walkable point by the original's flood; `nav::path`: the planner — the 256-cell window with its blocked border, the two-layer step rule, its LIFO open list and unsigned strict-total pop, the relaxation rule, the goal-height heuristic, the node cap, the closest-node answer with the 100 cm rule and the rough cut, and the corners' budget line walk; `nav::mover`: dirty rectangles, the sixteen-slot ring, the scene-wide countdown and the refresh from the floors as built, which the grid keeps beside the current ones; `nav::stamp`: a line stamped on the penalty bytes, as a barricade stamps its wall — its cells, their ring and the cells beside the ring, on the layer whose height band holds them, and the stamp taken back); `--debug-nav`/F6 draws it, `nav::headless` builds it without a window ([`docs/NAVIGATION.md`](../../docs/NAVIGATION.md)).
- `presets`: the environment unit's six-preset preparation: the sRGB
  linearisation of the twelve colour words (a double-precision division
  below the knee, the engine's fast logarithm and exponential above it, so
  the curve is close to the exact one but not equal) and the separately
  converted ambient word (only when it is above zero), the 0.75 scale of
  five sky words, and the 25 % blend that fills an empty ambient colour
  (only when its first channel equals zero); applied by
  `PresetTable::configure`, linearisation only when linear lighting is on.
  The shipped six records stay with the level data; Exarust's level lighting
  still chooses Indoor/Outdoor/FromRecord.
- `scene_nodes` also carries the live scene's bookkeeping, none of it on a
  draw path yet: the timed transition list (an add skips a repeated node and
  controller and flips the controller's 1/2 state; the advance passes the
  signed seconds since the start, overwrites finished entries with the last
  one and clears the moved word of the node under the sector), the
  256-entry node-removal queue (255 usable; a full queue still marks the
  node and the sector's removal bits), the despawn (a payload carrying both
  climb bits stands for the node under its sector; a character payload
  reports its character data, a live object-database word its slot), the
  detail level's 0.01 scale with the shadow size, the renderer and light
  lists (the added hook runs before a light is listed, the removed hook and
  swap removal after; the cell lights of a scene that keeps them are
  cleared first), and the sector destructor, which only empties the sector's
  lists. `flush_removals` is Exarust's own consumer of the removal queue.
- `terrain::manager`: the manager's construction numbers (a 48 m tile of 240
  cells of 20 cm, 241 vertices, 244 apron rows; detail size 4096/8192 from
  the texture-detail setting, quality tables from a separate quality level,
  the integer palette scale and the height scale), the tile record (heights,
  centred positions and normals as the two halves of one vertex buffer, the
  0x7f-filled channel read with a 244 stride, packed normals and opacity
  bytes), the Sobel normal with the processor's approximate reciprocal square
  root, the vertex-height pass, the 16 × 16 height bounds, the block
  descriptor (centre, half extents, radius) and the 8 × 8 translated block
  kd-trees. Only the detail displacement is shared with the running terrain
  build; its own normal keeps the exact square root.
- `terrain::plugins`: the terrain extension list (attach order, activate
  forward and deactivate backward) and the static terrain's create-or-reuse
  set-up: the scene is marked only when the manager is created, one shadow
  receiver per tile and block outside room mode, a pool sized in room mode.
- `grid` keeps the set-piece loader's rules: a fully transparent part (alpha
  below 0.01) marks its triangles' vertices in an added channel, the load's
  node flag words, and the child skip for the 20 cm subdivision (`#`, `+` and
  `[` children are not subdivided, but are still built).
