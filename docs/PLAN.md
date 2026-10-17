# Exarust — reimplementation plan

Goal: an open-source runtime, written in Rust on Bevy and wgpu, that plays
**Exanima** from the user's own installation, staying as close as possible
to the original in looks, feel and rules. Exarust ships no game data. It
reads the retail packs in place, the way OpenMW reads Morrowind or IW4L
reads Modern Warfare 2.

This file describes the plan as it stands today. Update it when the plan
changes. Each work item becomes an OpenSpec change (`docs/WORKFLOW.md`).
Behaviour that exists is specified in `openspec/specs/`. The record of
individual work sessions lives in `context/` (see `CONTEXT.md`).

---

## 1. What the original is

Findings from analysing the installation (details in `docs/formats/` and
`docs/research/`):

- **Data:** 12 `.rpk` packs, 7.8 GB in total, plus `Exanima.rfp`, which
  names the mounted libraries. The packs are uncompressed tables of contents,
  so they can be memory-mapped. About 22 000 entries in 15 formats.
- **Engine:** Bare Mettle's in-house engine, written in **Free Pascal 3.2.2**
  (Win64). Rendering is **OpenGL 4.x core** with block-compressed textures,
  shaders generated from fragment libraries, point-light cube shadows and
  **voxel global illumination**. Audio is **OpenAL Soft + EFX**. There are no
  third-party physics or animation middleware libraries: rigid bodies, cloth
  and articulated characters are all custom code.
- **Gameplay core:** characters are **physically simulated bodies** that try
  to follow a small set of motion targets (about 48 clips in total in
  `humanoid.rsq`). Combat comes from colliding weapon and body physics, with
  damage resolved through **layered armour** (cloth, padded, mail, plate). Items
  are **procedural**: weapons are assembled from parts by factories, and
  materials (steel grades, iron, copper alloy, leather, wood) generate their
  own textures.
- **Content:** Story mode (Underworld levels `exanima01..08` built from
  modular wall and floor tile sets, plus terrain levels), Arena mode (arenas,
  a hub, companies, fighters, seasons, a shop), Practice mode, thaumaturgy
  (four power domains), dialogue and journal.

## 2. Principles

1. **Read, never redistribute.** Assets are loaded from the user's install.
   The tracked tree, issues and releases never contain game files or
   anything converted from them.
2. **Original data as ground truth.** Formats are documented from evidence,
   meaning a parser that reads every shipped instance. Behaviour (physics
   constants, damage rules, AI timing) is measured against the original
   running side by side. A guessed value is labelled as a guess.
3. **Engine-independent parsing.** `exa_rpk` and `exa_formats` know nothing
   about Bevy. This keeps them testable and reusable by `exa-tool`, by
   exporters and by anyone else.
4. **Deterministic simulation.** Physics, combat and AI run in a fixed-step
   schedule with explicit state, so runs can be replayed and compared. This
   also leaves room for netplay later (not a goal for now).
5. **Bevy-native, not Bevy-shaped.** Use ECS, assets, states and the render
   graph where they fit. Write custom passes and solvers where the original
   depends on behaviour Bevy lacks (voxel GI, the character solver, cloth).
6. **Pinned Bevy.** Stay on one Bevy release (0.20 today) and upgrade
   deliberately, in its own change.

## 3. Architecture (summary)

Full description: [`ARCHITECTURE.md`](ARCHITECTURE.md).

```
            exa_rpk ──► exa_formats ──► exa_assets ──► exa_world / exa_render / exa_audio / exa_ui
                                            │
                            exa_anim ──► exa_physics ──► exa_character ──► exa_combat ──► exa_ai
                                                              │                │
                                                         exa_items ◄───────────┘
                                                              │
                                     exa_script ──► exa_game (modes, states, saves) ──► exarust (binary)
exa_tool: CLI over exa_rpk + exa_formats (inspect, extract, export)
```

## 4. Phases

Each phase ends with a **demo milestone** that anyone with the game can run.
Phases overlap where dependencies allow. The order follows the longest
dependency chain: data → world → bodies → combat → everything else.

### Phase 0 — Foundations ✅ (this commit)

- Cargo workspace, crate layout, docs, Makefile, toolchain, licence.
- `exa_rpk`: pack reader (mmap, case-insensitive lookup, nested packs),
  `Exanima.rfp` parser, `GameData` over an installation. Verified on every
  retail pack.
- `exa_formats`: format detection by magic, chunk-tree framing, `.rdb`
  index, `.rfi` decoding of every texture format (BC1/3/4/5, RGB8, RGBA8,
  L8, LA8, A8, raw and RLE-packed storage), checked visually.
- `exa-tool`: `info`, `ls`, `stats`, `chunks`, `rfi`, `png`, `extract`.
- `exarust`: a Bevy window that opens the install, lists packs and shows a
  texture decoded from the retail data.

**M0 "first light":** `make run` shows the human skin atlas from
`Resource.rpk`.

### Phase 1 — Complete asset decoding

Goal: every entry in every pack parses into typed data. The result can be
checked outside the engine through exporters.

| work item | crate | notes |
|---|---|---|
| ✅ Static mesh (`3D03`): vertex streams, index maps, triangles, material groups, node transforms | `exa_formats::rfc` | done: 23 065/23 065 retail meshes decode; cm → m, right-handed Y-up confirmed |
| Materials (`BA01`, `.rml` libraries): ✅ texture layers and slots; ✅ parameter block colours, opacity, gloss, specular level, render class, flags, kind and displacement shadows; subsurface colour, opacity fall-off and refraction still open | `exa_formats::material` | map the rest of the 184-byte block to the plug-in classes seen in the binary (`TMaterialPlugIn`, `TRTMatPlugIn`, `TProcMaterial`) |
| ✅ Skinned meshes: weights from seams (`add-skinning`), every race's body (`add-race-bodies`), morph sets (`*morph.rfc`, `morph-the-physique`) | `exa_formats::{rfc,morph}`, `exa_assets::{skin,morph}` | |
| ✅ Skeleton and motions (`.rsq`) | `exa_formats::rsq` | 18 bones, 48 clips |
| ✅ Node keyframe tracks (`3AC1`) | `exa_formats::key_track` | one track ships (the secret switch's brick); played by `exa_anim::tracks` |
| ✅ Level grid (`3DED`), level sections (`3DE10000`, `DBC0`, …), placed objects and characters, starts | `exa_formats::{level,characters}` | wall/floor sets `gews_*` / `gefs_*` |
| ✅ Terrain (`.rft`) and terrain tables (`.ftb`) | `exa_formats::terrain` | heightfield, layers, grass and plants |
| ~~Packed `.rfi` storage~~ | `exa_formats::rfi` | done in Phase 0 (RLE) |
| Databases: ✅ `races`, `objdb` (apparel, weapons, shields, doors, operative objects, layer table, worn model, occupancy), `locales` (level list); `charroles` and `objstrings` still open | `exa_formats::{rdb,races,objdb}` | record schemas |
| ✅ Rigid bodies of models (`3DD0B001`: spheres, links, sound slots) and the cloth marker (`3DD0C001`) | `exa_formats::{rigid_body,rfc}` | every body of every pack decodes |
| Factories ✅ (`add-weapon-factories`); actors, powers, dialogue (`.rcd`) still open | `exa_formats::{factory,actor,power,dialogue}` | |
| Shader libraries (`.flb`) | `exa_formats::flb` | optional: capturing the running game with RenderDoc/apitrace gives the plain GLSL directly |
| Exporters: ✅ glTF for static meshes with textures; skinned characters and assembled levels open; PNG ✅; WAV | `exa_tool` | check results in Blender |
| Corpus test: parse every entry and report coverage | `exa_tool corpus` | ✅ `corpus meshes`; more kinds as formats land |

**M1 "asset viewer":** `exa-tool export-gltf` opens any prop, weapon,
character or level in Blender, and `corpus` reports full coverage.

### Phase 2 — Static world (in progress)

Done (change `add-level-viewer`): level props and grid header decoded;
models, materials and BC textures converted to Bevy; `exarust level <name>`
shows a level's props and lights with a fly camera; `exarust model`.
Done (change `add-grid-builder`): walls, floors and ceilings from the tile
grid; `exanima01` and the arenas are recognisable.
Done (change `add-terrain`): terrain heightfields with holes, the palette's
blend rules (height, slope, the original's noise maps) and masked material
layering in a WESL shader; outdoor arenas render and are walkable
([formats/TERRAIN.md](formats/TERRAIN.md)).
Done (change `add-level-lighting`): the level environment record (ambient
colour, extent, sun) and the original's per-level presets: sunlight with
shadows and fog on the outdoor arenas.
Done (change `refine-grid-and-terrain`): wall pieces' own floor/ceiling
strips and occluders, the terrain height pass. Done (change
`add-vegetation`): grass and plants grown as the original grows them.
Done (change `add-outdoor-sky`): an open sky over the outdoor arenas
(Bevy's atmosphere stands in for the original's sky shader, which is not
decoded).
Done (change `load-behind-a-loading-screen`): levels and play mode load over
frames behind a loading screen, the long parts on the async compute pool,
so the window keeps answering the desktop ([UI.md](UI.md)).
Done (change `add-room-visibility`): the grid's rooms and door links (the
original's flood fill), each room's black occluder and lid merged from the
pieces' `[…` and `+R…` strips, ceiling caps, the original's active,
adjacent and hidden rooms around the player with their 1 s fades, and
terrain limited to the grid's terrain cells: the space between and inside
walls, the rooms the player cannot see and the rock between rooms are
black as in the original ([RENDERING.md](RENDERING.md), Rooms and black
fill).
Done (change `play-node-keyframe-tracks`): the scene nodes' keyframe tracks
decoded after any node kind's body and played as the original plays them:
a start (using the object, in the original) swaps once and reverse and
puts the tracks on the playing list; every frame the node's translation
follows the original's Catmull-Rom curve at its clock's time until once
or reverse ends. The one shipped track is the brick of the secret switch
in `exanima03`, which sinks 5 cm and comes back in 3 s; until objects can
be used, F7 starts every track ([formats/RFC.md](formats/RFC.md),
Keyframe tracks).
Remaining: portal clipping of adjacent rooms, doors' open state, the lid in
the lights' cube faces, terrain gloss textures, fine displacement, the
original's sky shader, time of day and weather of the outdoor arenas,
`exanima://` asset source, streaming.

- `exa_assets`: `exanima://<pack>/<entry>` Bevy asset source, asset loaders
  for meshes, materials and images, direct upload of BC textures with mip
  chains (fallback: CPU decode), async streaming on Bevy's task pools.
- `exa_world`: level assembly (tile grid → wall/floor set pieces → `Objlib`
  props → terrain), sectors and portals for visibility, level transitions.
- `exa_render`: materials on `StandardMaterial` first, then custom `Material`
  + WGSL where the look requires it (apparel tinting of BC4 diffuse, detail
  maps, alpha-tested foliage). Point lights with shadows, a fog/ambient term.
- Free-fly and top-down cameras, level picker.

**M2 "walkable world":** load any arena or story level, fly around, and get
correct geometry, textures and light placement.

### Phase 3 — Bodies: skeleton, physics, control (started)

Done (change `add-apparel-painting`): object database items and the
apparel painter (palette, cloth/leather/suede/velvet/mail/plate/painted
finishes); dressed characters show their colours
([formats/OTHER.md](formats/OTHER.md)).

Done (change `keep-humans-in-underwear`): apparel is worn as the model its
item's body names, so placed characters wear every stored item (103 of 103
placed humans covered); the player and the character view's human body are
kept in the original's underwear, pale grey linen braies, by its rule
([formats/OTHER.md](formats/OTHER.md), Underwear).

Done (change `paint-in-the-originals-colour-space`): painted apparel colour
follows the original's chain (inputs read as stored, the finishes' values
kept as sRGB texels and decoded when lit), now stated at the painter and
pinned by a test; painted colour mips average in linear light. The braies
still read darker than the skin because the skin lacks the original's
`chargen_*` composite (Phase 5, character surface)
([formats/OTHER.md](formats/OTHER.md), Colour space).

Done (change `light-skin-and-torch-as-original`): human and ancient skin is
drawn as the original's built-in skins, from the race's own textures, with
the diffuse half of its `chargen_*` composite (the race texture tinted by
the middle skin tone in sRGB space), rougher, with the subsurface program
([RENDERING.md](RENDERING.md), Skin);
the character view is the original's creation scene, two lights, no sun,
the floor fading to black ([RENDERING.md](RENDERING.md), Character views);
the carried torch is raised in the right hand with the "Torch" moveset by
the original's grip, and practice and arena levels start without it
([PLAYER.md](PLAYER.md), The carried torch). Skin tones per character
remain.

Done (change `morph-the-physique`, M18 of the master plan): a race's
morph set (`humanmorph.rfc`, `ancientmorph.rfc`: 18 landmarks, 2 286
positions, 11 targets) is decoded and every character's record weighs
its targets as the original's builder does (physique picker, height, age,
feral level, the female bit); its body parts and the apparel fitted to
them follow the body's surface (the transfer, cached per race and model),
are morphed before skinning with normals and boxes refitted, and the parts'
origins move with their landmarks, so the skeleton, the skin weights and
the physical body's joints follow the physique (a height of 1 drops the
pelvis 11.2 cm). Placed characters take their records' physique; the
player, the duellist and the character view keep the model's body unless
`--physique` sets one; races without a morph body (skeletons, golems, …)
keep theirs ([formats/RFC.md](formats/RFC.md), Morph sets). The limb
spheres are still not refitted to the morphed meshes (C20).

Done (change `add-motion-playback`): skeleton and motion set decoded;
clips play on the human body (`exarust character Default/Walk`).
Done (change `add-player-movement`): rapier physics with level colliders,
kinematic player capsule, camera-relative movement, chase camera
(`exarust play <level>`).

Done (changes `add-ragdoll`, `add-muscles`, `add-balance`): the
original's own body solver, not rapier joints. Each part is a rigid
tetrahedron of four particles with sphere colliders, joined by the
skeleton's links, under Verlet at 83 Hz. The dead and the knocked out go
limp. Living bodies follow their clips through muscles (pose drive, tone,
daze). Blows shove and daze; a body that loses its balance falls and gets
up ([formats/RSQ.md](formats/RSQ.md)). Done (change
`keep-bodies-out-of-level`): level contacts as the original makes them
(one-sided faces, swept tests, four stored contacts per part held through
the next tick, its friction), so shoved bodies stay on the floor. Done
(change `step-bodies-in-the-fixed-tick`, C1 of the body-controller plan):
bodies tick in the fixed step as one world in the original's order (every
character tick, the player's last, then the solver over every body). Clips
are sampled every tick, blows are queued for the next tick, parts are drawn
between ticks, and `exarust --trace` writes the body state per tick in the
retail probe's columns. Done (change `decode-race-body-fields`, C2): the
race factors the body controller reads (support pins, balance bonus,
stance factor, arm relaxation, limb damping) and the gait table are decoded
from `races.rdb` for every race, with the original's fallback for an
unknown race, and printed by `exa-tool races`
([formats/OTHER.md](formats/OTHER.md), Races). Done (change
`decode-race-stat-fields`, M40 of the master plan): the rest of the race
record the original reads is decoded for all 18 races, with its built-in
fallback: the clip-rate base (1.5 for the ghouls), size, scale (capped at
1.5 at spawn) and mass divisor; the gait rows' brake distances and the
feral sprint blend toward the jog row; the second flag word and the kind
word (red regeneration, claws, hovering, quadruped); the unarmed reach,
the weapon swing-turn and recovery factors, the AI attack bonus and
perception, the held-item offset, the footstep set and the voice; each
character's stats are derived from them at spawn (M41, below). Done (change `read-character-records-fully`, M39
of the master plan): character records are read whole, versions 5–7, in
the level databases and `actors.rpk` (every shipped record parses to its
end): physique, height, age, the feral level (every living human of the
story levels is undead, 0.4–1.0), the saved health and focus pools (full in
every living placement), combat skill, traits, experience, the six skills
with the technique learnt at each level, inventory, roles, saved state and
powers; the learnt techniques give the perks combat reads (Remise, Feint,
Fend, Riposte, Security, the six shield perks and the thaumaturgy ones),
the Manoeuvring in Armour tier and Deflection; `exa-tool characters`
prints them ([formats/RFC.md](formats/RFC.md), character records). Done
(change `derive-character-stats-at-spawn`, M41 of the master plan): every
character spawns with the stats the original's builder and init derive
from its record and race: body scale and size from its height (0.95 to
1.05; the race's scale capped at 1.5), standing pelvis and leg reach, the
most energy (1 − 0.1 × the feral level), step reach, support pins' factor,
the head's pose gain and muscle factor and the walking arms of the
undead, the sprint gait, perception and voice, maximum health with the
race-byte rules (the manipulator 0.375) and the saved pools (a living
record that saved no red health, as the arena's opponents, spawns full),
and the body rule bits. The body now reads the most energy, the head, the
walking arms and the scale: exanima01's zombies recover only to 0.9, are
dazed under 0.85 and hold their heads by their weakened muscles alone (the
daze, its pose hold, the swing clock and the attack gate measure energy
against the character's most); the player and the duellist spawn from a
new human record; `exa-tool characters` prints each record's stats
([formats/RFC.md](formats/RFC.md), Stats derived at spawn).
Held weapons and fists are held by the character's size, scale and
encumbrance (M21's holder), and the step reach is the weapon record's
one rule; red regeneration reads the rule bits (M43, Phase 4); the
support pins, gait and damage bits read the stats when they land. Done (change
`join-parts-at-their-origins`, C3): every part is pinned at its origin to
its place in the parent, as the original registers it, so joints are ball
joints and the knee a hinge; the skeleton's links meet the parent at the
stored point plus the model's translation, unturned; corrections are shared
by the parts' mobilities; living pivots, limp pivots and slack limits follow
the original's laws; the leg limit strengths and the limp smoothing are
kept ([formats/RSQ.md](formats/RSQ.md)). Done (change `damp-limp-joints`,
C4): a limp joint is damped inside its range through each link's smoothed
length, set from the pose the body is built in and reset when it goes
limp, so the limp character lies curled on its side instead of kneeling;
velocity retention is the original's default environment (0.426 / 0.25 /
0.05 per second, banded by each limp part's speed; living limbs keep the
fast value). Done (change `add-link-muscles`, C5): every slack link is a
link muscle that turns its joint toward the clip's angle (damped by the
joint's own motion, saturated, relaxed toward a held pose), with the gains
the original's character tick sets: the race's stance factor rising with
the gait, part factors, arm stiffness states and the leg limit's blend; the
pose drive is left only on the pelvis, spine, head and feet
([formats/RSQ.md](formats/RSQ.md)). Done (change `sleep-by-countdown`, S1):
a limp body sleeps by the original's countdown (256 ticks, two off each
tick its parts move under 20 cm/s, held at 30–32 between 20 and 40 cm/s,
back to 256 above), its first tick asleep takes its velocity away and then
it lies in deep sleep; blows and going limp wake a body, and living bodies
never sleep. All seven exanima01 corpses now sleep 3–5 s after going limp
(3 of 7 before). Done (change `measure-balance-and-support`, C6): balance
is measured as the original's at the chest (the thorax pins) against the
ankles (where the original's muscle setup moves the foot contacts), with
its lean score, counter (floor 80 standing still) and recovery; each leg
asks the level for its ground with the original's Height Query, and
support S = grounded × balance feeds the link muscles; Exarust's falls
follow the lean, decided in the fixed step from last tick's balance
([formats/RSQ.md](formats/RSQ.md)). Done (change `add-body-debug-tools`): debug tools for
bodies ([DEBUGGING.md](DEBUGGING.md)): an overlay of every body's
particles, links (coloured by how far past their slack) and muscle
anchors; a readout of one body's joints (turn from the rest, swing and
twist, distance from the clip, range used, pivot gap); a per-part joint
trace; pause, single tick steps and slow motion, with scripted pauses at a
tick or on a stretched link. Done (change `match-the-pose-drive`,
C7): the pose drive is the original's: it turns the pelvis, spine, head
and feet toward the clip's orientation about each part's own origin, with
no position target, by G·o·(1 − c) (a one-sided damper of the particle's
velocity), G scaled by t²·T·E⁴·(0.5T + 0.5)·S², only with support; a fall
keeps the tone and a fallen body is left to its link muscles
([formats/RSQ.md](formats/RSQ.md)). Done (change
`carry-the-body-on-its-feet`, C8): the body stands on its feet through the
original's carry: every limb particle falls by g(1 − a) + carry·a (a the
tone times its support share, or times its energy above the legs), and a
lift servo takes g·f·S off the carry to hold the pelvis at a target height
over the ground under the feet, from the clip root's height over each
ankle, capped at the leg's reach (an idle human's pelvis at 101.5 cm); the
capsule carries the body only across the ground and turns it, and holds
its pelvis horizontally (Exarust's own until the support pins, C14). A
get-up no longer hops (the end of `Get Up 2` overshoots 3–5 cm and settles
in 45–100 ticks, from 10.7 cm ringing 90). Done (change
`climb-steps-until-feet-step`, Exarust's own stopgap): until the
original's walking feet and leg IK (C13, C19), the lift reads no foot's
ground lower than the grounded controller's floor less 6 cm, so a body
climbs the steps its capsule climbs and stands on them (both exanima01
stair starts reach the landing without a fall; C8 alone fell 1–5 times),
while level ground is unchanged (the capsule floats 2–5 cm there; the
arena walk's target is the same every tick). C13 and C19 remove it.
Done (change `anchor-the-character-frame`, C9): the
pose is put on the original's character frame, whose origin is the pelvis
less the clip root's offset across the ground: it rides on the pelvis
while the body moves, follows it exactly without support, within 5 cm
while a standing body's support is under 0.95 and not at all from 0.95,
within 5 cm while down and 10 cm as a get-up starts. The capsule's
horizontal hold now pulls toward the frame's pelvis (Exarust's own until
the support pins, C14), so a steady body keeps its stance spot and an
unsteady one stays where it is pushed; the capsule follows the frame
through the controller (a tenth of the way a tick, at most 2 cm), so it
stays with a shoved, falling or rising body, and a get-up no longer moves
the capsule in one go. Done (change
`blend-clip-starts`, C10): clips play at the original's clocks (a race's
base rate × 0.6 a tick standing, 0.75 off balance, 0.9 getting up, the
weapon's swing rate attacking, the gait's rate × (0.8 × speed + 1)
walking), loop by their last key with a blend back to the first, and
start from where the body is: a capture of its root, its legs' targets
and its upper body's physical turns (a get-up from the floor: the whole
lying body), blended to the clip's first key over (first key + blend)
frames, eased; a clip played once ends into its stance with the
original's blend table (2 × the weapon's stance blend after an attack).
Clips keep their root's travel but for the walk, run and backpedal. A
dazed or unsteady body holds its physical pose by the original's hold
weight. The end of `Get Up 2` no longer jumps (36 cm in one tick before;
its pelvis target now moves under 1 cm a tick), though the pelvis still
overshoots its target height by about 4.7 cm while the lift catches up;
attacks take about 1 s (1.4 before)
([formats/RSQ.md](formats/RSQ.md), Motion playback). Still to do: the
support pins, the original's falls and get-ups, recovery steps, foot
planting, the locomotion clock's stride terms and the run overlay,
collisions between bodies (and their wake of a sleeper).

Done (change `decode-object-bodies-and-mechanisms`, data only): models'
rigid bodies are decoded as the original stores them (class, mobility,
rest tetrahedron, point masses, collision spheres, links with the
slider's travel stored squared, scrape and roll sound slots; 1 163 of the
1 253 `Objects.rpk` models, 65 of them hung from the world by links), with
the cloth marker of the long hair, four banners and the wraith's robe
([formats/RFC.md](formats/RFC.md), Rigid body); the object databases' door
and operative sections (keys, levers, buttons and plates, fire and light
jobs, relays) and the body class a record gives its model
([formats/OTHER.md](formats/OTHER.md)). `exa-tool bodies`, `mechanisms` and
`corpus bodies` print them. Next: props as bodies in the solver (M32), then
world links, the hand's drag, doors, levers and plates (M33–M35).

Done (change `simulate-props-as-bodies`, M32): the free rigid bodies of a
level's props, placed objects and own model are bodies in the 83 Hz
solver, registered as the original registers them: asleep (countdown 0)
where the level placed them, mobility over their first material's physical
divisor (decoded: iron 2–3, wicker 0.6, most 1), gravity at the centre of
mass, the stored spheres against the level with friction 0.75, articulated
children joined by their links and an origin pivot; a blow wakes them and
they fall, settle and sleep again; below −50 m they are removed. Props
placed static (flag `0x80000`, 3 883 of 79 935) and level nodes marked
static (the same node flag: `arenao1`'s tavern table) stay level geometry. Their
meshes stay as stand-ins moved with them, so capsules and character bodies
meet props as before ([formats/RFC.md](formats/RFC.md), Rigid body,
Materials, Props). `exanima01` registers 4 591 trees of 5 000 bodies.
`--shove-props` shoves the props near the player. Body–body contact came
with C17 (below). Not yet: world links (M33: lamps, chains and hanging
braziers stay static), doors, levers and plates (M12, M35), the drag
(M34), impact sounds (C22).

Done (change `follow-the-pelvis-with-the-camera`, M3 of the movement plan): the
chase camera looks at the pelvis as it is drawn between body ticks, and
shows its own 17 ms ticks blended by how far the frame is into the last
(nine ticks a frame at most), and rings a kick of the camera down as the
original's shake does ([PLAYER.md](PLAYER.md), Camera).

Done (change `track-character-condition`, M38 of the master plan): every
fighter keeps the original's condition word (normal, walking, attacking,
combat idle, weak, down, getting up, dormant, collapsing, dead, shocked,
kicking, switching weapons), written once a fixed tick by a posture stage
in the original's order (the daze first, then the off-balance test at 0.6
and 0.3, the attack, tone 0.85, the clip), and by the get-up's start, its
clip's end, a killing blow and a level's corpses; the cursor, the player's
steering and attacks, the duellist, the daze's stance and hit detection
read it by the original's masks instead of a separate down flag;
`--trace` logs it (`--shove 60`: normal while falling, down under 0.3,
getting up, normal) ([formats/RSQ.md](formats/RSQ.md), Condition). Next:
the collapse before death (L5), the falls and get-ups' own rules (C11).

Done (change `stage-the-character-tick`, M0 of the master plan, no
behaviour change): the fixed step's tick is named in the original's order
(pre-step, character ticks, solver, post-step). Each living character's
tick runs nine stages (upkeep, readiness, anchor, behaviour, posture,
attack, movement, sampler, carry) before the next character's, the player
last; Exarust's work sits in posture, movement, sampler and carry, one
module each (`exa_game::stages`). The solver calls registered stages at
the original's points (after integration, the pair pass and links between
bodies in each of the 20 iterations, sim objects, body–body detection,
sim-object contacts, the pair friction of each of the 8 passes, after the
post-step), so pairs, props' mechanisms, world links and cloth join
without editing its loop ([formats/RSQ.md](formats/RSQ.md), how a tick
runs). `exarust --fixed-frames` makes runs repeat tick for tick so traces
compare between builds ([DEBUGGING.md](DEBUGGING.md)). The stance, the
condition and the collapse's dying tick (M38, L5), falls, attack stamp and
player input still run ahead of the character ticks, over every body, and
the AI once a frame; each joins its stage with the change that rewrites
it. The condition's clip-end writer runs in the sampler stage.

Done (change `collide-bodies`, C17 of the master plan): body parts collide
with each other as the original's pairs do, for every body (characters,
corpses and props). After the iterations every
awake part meets the parts near it, awake or asleep; two parts of one body
skip each other by the original's group masks (legs, pelvis, spine and
head never meet; arms meet the lower body; hands meet almost everything
but their own arm), parts of two bodies by its layer masks (two
characters' living lower legs pass through each other until one collapses
or dies), and a character's feet never meet other bodies. The more mobile
part brings its spheres
against the other's (spheres against spheres until the parts' spheres are
fitted to their meshes, C20): each hit pushes both at once by their
mobility shares and how concentrated the contact is on each one's
particles, keeping their momentum, and is kept (up to four points per
pair) for the next tick's iterations, where a point over 0.01 cm deep is
corrected with the original's reversed pair friction, and a pair needing
nothing is dropped; the pairs rub before the level in the 8 friction
passes. A contact wakes a sleeper with the toucher's countdown, so a
corpse walked into wakes. Props meet bodies through the pairs (their
mobility, the group masks of their tree), so a cup rests on a table that
is a body and a character pushes a prop; their stand-ins in the physics
world are the character capsules' only. `--pair-trace` writes every
contact and `--corpse` lays a dead body down in play mode
([DEBUGGING.md](DEBUGGING.md)).
Weapons, shields and torches as parts with their hit events (C21), impacts
(C22), the placed state and the AI's overlap queries remain
([formats/RSQ.md](formats/RSQ.md), Contacts between bodies).

This phase carries the most risk and defines how the game feels.

- `exa_anim`: skeleton, clip sampling and blending, procedural layers
  (aim, look-at, stance, idle noise) that produce **target poses**.
- `exa_physics`: fixed-step simulation at the original's 83 Hz. Rigid bodies and
  collision for props and level geometry. **Articulated character bodies**
  driven by joint motors toward the target pose (an "active ragdoll"), with
  balance (`Rebalance`, falls, get-ups), momentum and joint limits.
  - Decision: **rapier3d** for level colliders, props and the player
    capsule. The character solver is in-house behind `exa_physics` (done,
    see above), so it can be tuned to match the original.
- `exa_character`: race body plus morphs (done, `morph-the-physique`),
  skinning, hair, apparel meshes fitted to the body (the original uses skin
  pivots and edge splitting, see `TSkinBuilder` and `TEdgSplt`).
- Player control matching the original (done, `original-controls-and-torch`,
  see `docs/PLAYER.md`): W walks the heading and A/D steer it, the right
  button walks to the cursor, Tab toggles combat mode (on at every
  practice and arena fight start) where the body faces the cursor and the
  mouse steers attacks. Camera at a fixed 20 m with a
  field-of-view zoom. Still to do: crouch, interact.

**M3 "standing man":** a fully dressed character walks around a level under
player control, stumbles when pushed, falls and gets up.

### Phase 4 — Combat and items (started)

Done (change `add-combat-prototype`): held weapons, attacks steered by the
mouse, hits between blade and body parts, knock-downs and a duellist.
Done (change `add-weapon-factories`): weapons assembled from the factory
recipes with the original's seeded choices, deforms and painted parts.
Done (change `add-damage-model`): weapon stats, apparel protection and
races' natural armour decoded, and every blow resolved by the original's
model: the zone along the weapon, the speed of the struck point, two armour
layers per part with the coverage roll, impact, crush, slash and pierce.
Done (change `start-no-clip-on-a-wound`): a blow that does not kill starts
no clip on the struck fighter, as in the original (Exarust used to play
Head Pain, a story clip, for any wound to the chest, head or arms, and
Rebalance below); a fighter struck mid-attack stopped it and returned to
its stance at once, Exarust's stand-in until the swing stall
(`run-the-swing-clock`); the
shipped elbows and knees are shown to be hinges bounded by their links
(elbow straight to 120°, knee 10° to 130°)
([formats/OTHER.md](formats/OTHER.md), Damage;
[formats/RSQ.md](formats/RSQ.md)).
Done (change `choose-the-moveset-from-held-items`): what a character holds
picks its moveset, stance class and weapon class as the original's rule
does, in and out of combat mode (the fists the race's own moveset, a
one-handed weapon "One Handed", two-handers by their flags, polearms,
shields, a torch in either hand), for the player and the duellist; the
player's left click picks the original's attack (the main side at random
without repeats, the other side with the cursor more than 20° left of the
facing, an overhead on a double click), so unarmed it throws the "Default"
punches, which cannot land yet; in combat mode the heading turns to the
cursor beyond 30° and the body turns toward it
([PLAYER.md](PLAYER.md), Combat mode and Moveset from held items).
Done (change `take-the-combat-stance`): every fighter keeps the
original's stance, the looping clip it returns to; in combat mode the
player stands in the Defence stance (unarmed, the fists-up guard of the
reference shots) and drops to Cmbt - Idle with its target within 3.5 m or
for 3 s after its attack, and stays there when the target leaves, as the
original's body tick never switches away from Cmbt - Idle; practice and
arena fights start in combat mode once the level shows; the duellist
enters combat in its guard and guards at four reaches or more; standing
3 s in Cmbt - Idle halves a blow's wound (readiness); `--still-opponent`
shows the stances at a chosen distance ([PLAYER.md](PLAYER.md), The
combat stance).
Done (change `draw-world-cursor`): the original's world cursor in place of
the system pointer, from `cursors.rfi` at the GUI scale's size (30 px in
a 900-pixel window): the red ring in combat mode, the arrowhead after
death or with Ctrl, amber chevrons walking to the cursor and the
arrowhead or cross outside combat; the cursor's point on the 80 cm aim
plane, found through the supersampled camera's viewport
([UI.md](UI.md), Interface scale and World cursor).
Done (change `draw-the-health-bars`): the HUD's health bar (yellow, brown
for damage that will heal, red for lasting damage) and focus bar (full blue:
no powers yet) at the bottom centre, painted as the original draws them
(gradients, inner and drop shadows, frames tinted by `guitex.rfi`) and
fading in with combat mode and out 200 ms + 2 s after; the stamina gauge's
rule (no shields yet); the low-health heartbeat (a pulsing dark red border
and its sound, story levels only); the interface laid over the game's view
premultiplied, which fixed every translucent interface pixel
([UI.md](UI.md), Health bars, Heartbeat, Compositing).
Done (change `complete-the-weapon-record`): a held weapon's whole record
as the original builds it: swing speed and shaping, recoil, stance
blend, arm stiffness, second-hand place, swing sound and AI turn factors,
reach and mass, held by the holder's encumbrance and size and refreshed
on equipping with the race's factors; its length along the model's
rigid parts; the built-in fists and claws by race; the shield record and
its factors; the step reach. Held weapons and the duellist's reach use it;
fists that strike, the swing clock and weapons as bodies build on it
([formats/OTHER.md](formats/OTHER.md), Weapon record).
Done (change `keep-the-combat-idle-until-the-player-walks`): the
user's reference shots of the original's torch arm (a compact carry before
the chest) are its combat mode, the "Torch" Cmbt - Idle; exploring, both
games raise the torch beside the head with "Torch" Idle, and the grip,
overlay, arm muscles and tracking were measured equal. The exploration
steering now starts only the gaits, as the original's, so a player that
turns combat mode off in Cmbt - Idle keeps that carry until it walks;
tests pin both carries to the installed clips ([PLAYER.md](PLAYER.md), The
carried torch). Next for the torch's look: the torso twist and head look
toward the cursor (M8) and the open hand.
Done (change `strike-with-fists`): the fists strike, as the original's
empty hands are held items with the race's fist or claw records, each a
striker across its hand's box, the motion's hand striking (the right
straight, the left straight, the right overhand); the original's attack
executor for the player and the duellist: 80 % of the most energy, the
cooldown from the last attack tick on the 12 ms game clock (400 ms with no
primary striker, else 300 ms × its recoil + 300 ms: 330 ms for the fists);
the player's left button kept as a standing request each behaviour tick,
so holding it repeats the attacks, a held double click's overhead follows
the attack, and Alt stabs with a held weapon; the race's weapon fields
from its decoded stats; `--pause-on-hit` for pictures of a landed blow
([PLAYER.md](PLAYER.md), Combat mode; [formats/OTHER.md](formats/OTHER.md),
Weapon record).
Done (change `blend-stance-and-daze-starts`, after C10 and L2/L3 met):
every stance change starts its clip with the original's blend and ease
(the set-stance helper 30 frames smooth, the posture switch 30 linear, a
duellist's stance 40 or twice its primary's stance blend smooth); a dazed
fighter (energy more than 0.05 under its most, in double precision) takes
the Defence stance, its clip starting with round((1.05 − E) × 300) frames
of blend unless it is down or attacking, and the player's stance rule
gives Cmbt - Idle only out of the daze, so a dazed body's targets blend
back instead of jumping to the clip when its energy returns; the other
side's attacks start with round((20 − W19) × 0.6) frames
([PLAYER.md](PLAYER.md), The combat stance;
[formats/RSQ.md](formats/RSQ.md), Motion playback).
Done (change `sum-armour-encumbrance`, M42 of the master plan): armour
weighs. Every character spawns with the original's encumbrance, summed as
its armour pass does from what it wears (0.0025 per unit of each item's
byte, less 0.025 for each articulated plate piece touching another, less
0.3 per Manoeuvring in Armour tier, an unpowered golem suit 1) and the
shield it holds (its bulk, less with Mobility), at most 1: the duellist's
jacket and helm 0.175, a tourney plate set 0.625, Elestan of the story
0.77; its step reach shrinks 20 % per unit and the player's run blend tops
at 1 − 0.3 × it (3.2 m/s in the plate set instead of 3.6). The armour
layers read each item's protection as stored (made items store it scaled
by their condition; Exarust scaled it twice) and Deflection widens
coverage. The weapon record's holding pass reads it too (in the plate
set a sword's recoil and swing shaping rise, its blend by a frame); the
turn, sprint, steps and get-up clock read the same value when they land;
`exa-tool characters` prints it ([formats/OTHER.md](formats/OTHER.md),
Armour and Encumbrance).
Done (change `face-the-practice-duellist`): in `play arena`, which has no
opponent's start, the duellist stands 2.5 m in front of the player, facing
it, instead of behind it, where its blows fell 15–21 cm short of the
player's back and none landed. A duellist behind a player who turns away
still falls short: its striking distance (1.15 m between the characters)
stands in for the original's reach and footwork (C16, L11).
Done (change `assemble-movesets-by-the-tree`): every clip comes from the
character's clip table as the original builds it from `humanoid.rsq`'s
moveset tree (parents are the nearest earlier moveset of their name): the
race's own moveset chain from the root, the weapon moveset the held items
ask for found under each of its ancestors, then the "Torch" overlay. The
held items give the original's config word instead of a moveset name, so
a torch beside a shield and the creatures' own weapon movesets (the
golem's, the ogre's, the demon's "One Handed") now resolve; a human's
clips are unchanged. Tables are built once per race moveset and config;
`exa-tool clips` prints one ([formats/RSQ.md](formats/RSQ.md), A
character's clip table; [PLAYER.md](PLAYER.md), Moveset from held items).

Done (change `collapse-before-death`, L5 of the master plan): a lethal
wound no longer kills at once. The fighter collapses as in the original:
its condition collapsing, Fall Backward from where it is with
round((1.2 − E) × 300) frames of blend, off its controller; each tick its
heading wanders, its tone falls 0.003 and its pins' gain 0.0003, its frame
follows its pelvis, its lift fades by the tone squared and its arms go
loose (0.2); it throws its arms wide with Rebalance once its balance is
under 0.8; at a tone of 0.25, 250 ticks (3.0 s) after the blow, it dies a
plain ragdoll that keeps its motion (no invented velocity). The cursor
keeps the ring through it ([formats/RSQ.md](formats/RSQ.md), The
collapse); `--pause-after-collapse` for pictures of it. The corpse lies
as the original's (on its back, arms out, a knee up), but the body
squats on its feet longer before it falls (Rebalance 1.6–1.9 s after the
blow; live 0.9–1.4 s at like energies). Still missing from it: the
stagger steps (C15/C16), the dropped weapon (L13).
Done (change `regenerate-health`, M43 of the master plan): wounds heal as
in the original. Each fixed tick yellow health regains √red × a rate that
ramps up over 40 s to 0.01/83 a tick (0.005 a second for a human with
full red), held at 0 while the fighter attacks or Cmbt - Idle plays and
set back to 0 by every blow and weapon contact; a heal pool (nothing
fills it yet) and, for the ogre, Sir, the harvester, the demon and a
cored golem suit, red regeneration (1.25e-5 a tick, dead or alive) refill
red; yellow stays within red, red within the maximum, and death empties
yellow. The rule fits a retail trace to 0.0012 over 46 s; in game, the
player's yellow refills on the HUD once combat mode is off, the bars
showing while it rises ([formats/OTHER.md](formats/OTHER.md),
Regeneration; [UI.md](UI.md), Health bars). Not yet: bandages, potions
and the second wind (M46), burning (M45), the saved heal pool (M47).
Done (change `apply-damage-as-the-original`, M29 of the master plan):
blows take health by the original's damage call, its order and its
double-precision arithmetic: yellow −0.1·Y (half that under a potion),
then red −0.2·R with yellow scaled by the red left, except that feral
humans, skeletons and ancients (rule bit 0x8: 248 of the story levels'
451 living characters) keep their red and lose the red share from
yellow; only vital damage (every weapon blow; impacts once C22 lands)
kills, when it leaves yellow under 0.01 in double precision
(so 0.01 in single precision dies), costing red 0.1 × the maximum in the
story levels; anything else that leaves yellow under 0.05 runs the
low-health check, which now starts the player's heartbeat (and later the
second wind, M46). The "no death" byte and the player hook the original
also tests are never set in its executable (RB48), so Exarust has
neither. `--feral f` makes the player feral, for a duel in which its red
stays full ([formats/OTHER.md](formats/OTHER.md), Applying damage;
[UI.md](UI.md), Heartbeat).
Done (change `fall-to-death`, M44 of the master plan): a character that
falls into a pit dies as in the original. Each tick a living body off its
balance and still moving checks its fall: sinking faster than 5 cm a tick
it screams once (a heavy groan); with no ground within reach under its
pelvis and its feet (25 m) it is lost; the player hears the pit 1 s later,
its screen fades to black over 3 s and it dies at 4 s, anyone else at 3 s,
and the character leaves the world with what it carries. Unsupported
bodies now measure the ground under their pelvis for it. Dead bodies that
fall below −50 m are taken out, the player's kept
([formats/RSQ.md](formats/RSQ.md), Falling to death; [UI.md](UI.md),
Death fade). A drop from 200 m in the arena: lost about half a second
after the body loses its balance, the pit 84 ticks later, gone 334 ticks
after the loss. Not yet: the pause and a death menu at the fade's end,
the voice sets (the scream picks a human voice), the retail chasm
capture (RR18).
Done (change `run-the-swing-clock`, M22 of the master plan): every
attack tick runs the original's swing routine. The clip clock runs at the
primary striker's rate (now with the feral level's ramp); a swing
strength scales it while the swing stalls, and the stance returns when
the stall ends: a dropped request releases the swing (a click checks a
punch in 5 ticks, a held button swings through: 67 ticks for a punch), and
a blow dazing an attacker stalls its swing, replacing the stop-at-once
stand-in. The strike window opens at 0.3 of the clip (stab 0.4, overhead
0.5) with the motion's hand, the overhead's and stab's close late; a
released swing's next press within 300 ms throws the overhead at once;
the record's perks reach the fighters: Remise chains a held side attack
past 0.8 (a punch after 55 ticks, a greatsword's Attack Mid after 78: the
same rule gives the live trace's two-hander its 828 ms within its weapon's
spread), Feint renews a released swing; the stab's and overhead's start
blends; the attack turn by the weapon's handling. `--perks` and the
trace's swing columns ([PLAYER.md](PLAYER.md), Combat mode;
[formats/RSQ.md](formats/RSQ.md), Motion playback). Still missing from
it: the swinging arm's stiffness (needs per-arm stiffness in the body
controller), the swing sounds, the AI's own release.
Still to do: inventory and containers, injuries beyond damage, blocking,
parrying, grabs, practice mode, the knockback step, the striker's recoil
and follow-through (C21, M26), the collapse's stagger steps, the
balance's Defence stance and Rebalance (C11); next in the combat timing:
the unarmed parries (L10).

- `exa_items`: item database, item instances, inventory and containers,
  equip and wield, weapon assembly from factories, mass and centre of
  balance from geometry, procedural material grades and their textures.
- `exa_combat`: hits from contacts between weapon and body parts, impact
  energy and contact area (edge, point, blunt), armour layer coverage per
  body part, cut/pierce/blunt penetration through layers, injuries
  (bleeding, limb impairment, stun, knock-down, unconsciousness, death),
  blocking and parrying with weapons and shields, grabs and tangles.
- Practice mode: a dummy and a sparring partner.
- Minimal UI: inventory, equipment, health.

**M4 "first duel":** fight a passive or scripted opponent in an arena.
Hits land where the weapon actually touched, and armour visibly matters.

### Phase 5 — Visual and audio fidelity (started)

Done: the original's render pipeline (legacy and PBR programs, the
environment cube, parallax, supersampling without MSAA, sharpening),
voxel GI with the voxel lights of glowing items, emissive surfaces drawn
as the original's (additive glows, opacity fall-off), uncompressed
textures in the original's channel order (red embers, green foliage, RGB8
normal maps), fires and their particles, point lights that stop at walls
with cached cube shadows, and basic audio (music, ambience, impacts,
footsteps).
Done (change `shade-and-paint-blood`): wound blood as the original keeps
it, one byte per vertex in the colour stream's alpha, painted by the
original's wound painter (a ball behind the centre, a smear along the
blow's follow-through, speckled edges) on a character's skinned meshes and
the meshes under its head, each shared mesh copied for the struck
character on its first wound; dirt mode 4 shading against the speckle
`spatter01` for the human skin, painted apparel, factory items and a
bleeding race's skin (`races.rdb` flag bit 0), and `--debug-wound` to paint
test wounds.
Done (change `bleed-from-wounds`): blows bleed as the original's do: on a
race that bleeds, when the wound's size R (the lasting damage on the part
before the red bar's 0.75) exceeds 0.05, the wound waits on the victim
(seven at most) anchored to the striking blade and is painted seven body
ticks later, smeared along the blade's follow-through, with radius
30·R + 10 cm, on corpses too.
Done (change `stain-weapons-with-blood`): weapons stain as the original's
do: the weapon that deals a bleeding blow draws half the wound's R of
blood (up to 1, never washed off; fists none), and once it holds more
than 0.01 a second, multiplying pass draws the blood gradient `BldGr02D`
over its model's first mesh (every part of an assembled weapon), keyed on
the height up the blade so that the stain runs down √(80·L·blood) cm from
the tip, densest at the tip and speckled at its edge; `--weapon-blood`
shows one without a fight.
Done (change `spray-blood-from-wounds`): bleeding blows spray as the
original's do: one jet per bleeding blow, fixed to the struck part at the
contact point and pointing against the blade's swing, sending 36·R
droplets (60 at most) at 40 a second; each droplet flies off at up to
twice its jet's velocity, falls at a quarter of gravity for its 62 ticks
and never touches the floor, drawn as a camera-facing diamond of the
`BldPrt02` spatter that multiplies the frame (white leaves it unchanged);
`--debug-spray` fires test jets and `--pause-after-jet` stops a fight just
after the first. Next: blood across saves and respawns, creatures' own
blood (their striking body parts), the 8-bit clamp before the multiplies.
See `docs/RENDERING.md` and `docs/PERFORMANCE.md`.

- Materials: one `ExaMaterial` whose parameters follow
  [OpenPBR](https://github.com/AcademySoftwareFoundation/OpenPBR) names
  (mapping in `research/openpbr.md`), with two shading modes: **Original**
  (default, reproduces the captured GLSL lighting model) and **Enhanced**
  (an OpenPBR subset in WGSL: base, specular, coat, fuzz, approximate
  subsurface).
- Lighting: cube-map shadows for many point lights (torches, candles),
  and the original's voxel light-propagation GI rebuilt as a compute pass
  (done: one block per level; see `docs/RENDERING.md`).
  Ambient occlusion, bloom, tone mapping, colour grading against reference
  screenshots.
- Effects: particles (fire, sparks), blood sprays and spatter decals,
  bloody weapons, thaumaturgy effects (force waves, barriers, tendrils).
- Character surface: GPU generation of procedural skin, injury and decal
  textures (`chardecomp_*`, `chargen_*`).
- Cloth on capes, tabards and padded armour (simplified, physics-driven).
- `exa_audio`: positional playback of `Sound.rpk`, reverb environments
  per area, sound sets per material and event (impact, footstep, voice), and
  music. Its modules (`bank` with the one sound-set picker every event
  uses, `music`, `hits`, `footsteps`) take each lane's new sounds as their
  own module; the original's set probing (01 to 16, no repeat) and its
  audio environment are still to do.

**M5 "looks and sounds like Exanima":** reference shots taken from matching
camera positions in the original and in Exarust are hard to tell apart.

### Phase 6 — AI (started)

Done (change `build-the-navigation-grid`): every level gets the original's
navigation grid when its load ends, a 30 cm grid of voxel columns built
from its terrain, its meshes' GI sample points and its tile-grid floors,
reduced to two layers with the cost bytes the planner reads; `--debug-nav`
draws it and `exa-tool nav` dumps it ([NAVIGATION.md](NAVIGATION.md)).
Done (change `build-nav-terrain-only-where-blocks-are`): the grid's
terrain comes only from the 6 m blocks the level builds, the renderer's
(in a tile-grid level the flagged ones), so the rock between exanima01's
rooms is no longer floor. Next: the grid queries and the original's
look-ahead, then A\* and NPC paths.

- Perception (sight, hearing), navigation on tile grids and terrain
  (the original's 30 cm two-layer grid and its A\*, no navigation mesh),
  activities and idles.
- Combat AI that drives the **same control inputs** as the player
  (movement, facing, attack direction, stance), so NPCs obey the same
  physics. Skill-dependent timing and aggression.
- Companions and hirelings, arena opponents, monsters (golems, demons,
  wraiths, ogres, skeletons).

**M6 "fair fight":** NPC-vs-NPC and player-vs-NPC duels that behave like
the original's.

### Phase 7 — Game modes and content

- **Arena:** company creation, roster, hiring, training, skills and
  experience, matches (1v1, team), seasons and rewards, shop, emblems.
- **Story:** level progression `exanima01 → 08` with variants, scripted
  encounters, dialogue (topics, choices, phrases), journal, map discovery,
  portals, thaumaturgy powers (displacement, energy, force, mind).
- Character creation (race, body, face, hair, colours, class, skills).
- Saves in an Exarust format. The original's save files are not a
  compatibility goal.
- Full UI using the original GUI skin, icons and manual pages.

**M7 "arena season", M8 "story act 1", M9 "full story".**

### Phase 8 — Polish and release

Performance (streaming, culling, GPU timings), settings, controller
support, Linux/Windows/macOS packaging that auto-detects Steam
installations, crash reporting, a modding hook (loose-file overrides that
take precedence over packs), regression tests based on recorded inputs.

## 5. Method: how the original is studied

| tool | use |
|---|---|
| `exa-tool` + hex inspection | format structure, corpus statistics |
| REA (Ghidra provider) on `Exanima.exe` — see `RE-TOOLING.md` | readers (`TRFCReader`, image loader, packed-image decompressor), constants (physics, damage), rules. FPC RTTI names classes and virtual tables. Decompiled code is never copied: rules and constants are rewritten in Exarust's own terms |
| RenderDoc / apitrace on the running game (Proton/Wine) | GL state, plain GLSL as compiled, draw order, light counts, shadow resolutions |
| Side-by-side capture | identical scenes and camera paths in both runtimes; screenshot and video diffs |
| Official sources | [baremettle.com](https://www.baremettle.com/) news archive (`/news/`), forums and Steam devlogs: release notes, combat demo videos and design statements, used as behavioural reference |
| Community references | the community format spec Structs.txt; the Rust [Exanima Modding Toolkit](https://codeberg.org/ExanimaModding/Toolkit); [ExanimaREP](https://github.com/Horse4Horse/ExanimaREP) (IDA database with ~330 named classes); translation projects that solved the image RLE; Jango's Blender RFC editor. All collected in `docs/research/online-research.md`. They show where to look; they are not evidence. Respect each licence: some are source-available with no redistribution, so read them for facts only. |

Facts, offsets and dumps stay in `context/` (untracked). The tracked tree
holds the runtime and the documentation of formats and behaviour.

## 6. Risks

| risk | impact | mitigation |
|---|---|---|
| Physics feel diverges from the original | the game stops being Exanima | own character solver; fixed timestep; tuning harness that replays recorded inputs; compare against videos of the original |
| Scripts or shader libraries resist analysis | missing story logic or exact shading | RenderDoc capture for shaders; story logic can be re-authored from observed behaviour; the community spec covers part of the script language |
| Format drift: the game went 64-bit in 0.9.5 and community specs predate it | wrong field layouts | every claim re-checked by a corpus parse of the installed build |
| GI and many shadowed lights cost too much on wgpu | looks flatter | baked GI per level as a first step; budgeted shadow updates |
| Level sections only partly understood | missing props or lights | corpus statistics plus Ghidra on the level reader |
| Bevy API churn | upgrade cost | pinned version, thin integration crates |
| Legal | takedown | no assets, no decompiled code, no keys; the user must own the game |

## 7. Immediate next steps

[`COMPONENT-MAP.md`](COMPONENT-MAP.md) sets every component of the original
against Exarust and lists the largest missing ones. In order:

1. The support pins, the original's falls and get-ups,
   recovery steps, foot planting and leg IK (stairs), clip blending, and
   parts' spheres fitted to their meshes (Phase 3).
2. Inventory, equipment and containers, then blocking and parrying
   (Phase 4).
3. The original's NPC brain: perception, targets and the combat state that
   drives the same inputs as the player (Phase 6).
4. Portal clipping and doors' open state, and level transitions
   (Phase 2).
5. `add-static-mesh-decoding` is still open, waiting on a visual mirroring
   check against the running game.
