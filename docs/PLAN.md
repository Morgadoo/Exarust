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
Done (change `clip-portals-and-cross-levels`): a shut door does not load
the room behind it; an open doorway shows a point only when the view
crosses its opening (82 × 245 cm, 115 × 290 cm, or 200 × 340 cm); a
passage loads the named level.
Done (change `pull-a-hinged-door`): a hinge creaks once its smoothed handle speed is past a squared 0.015. A velocity of 1 gives pitch 0.711. A shut door's pull count rises by 1 up to 20 and falls by 4 while it is held; from 20 the seventh pull unlatches it. Opening shoves the handle 3 cm along the facing.
Done (change `motor-a-hinged-door`): a toggle or an open command aims at (1 − fraction) · range · sign + closed. A plain step is 0.0013 rad when the error and the range are both 1. A fast motor steps 0.005. An open command 0.05 rad short of its aim arrives and ends. A countdown below 2 ends the command after that tick's drive.
Done (change `slide-a-door`): a sliding creak at stored energy 0.25 has volume 0.1 and pitch 0.75. A motorised open at fraction 0.015 drops to 0 with smoothing 0.1. A close at 0.95 latches at 1. Shut and not held, the fraction stays 1 with smoothing 0.5. Held at 0.9, the seventh pull from 20 unlatches. A plain open jumps to 0.6 for 80 ticks. A frozen door keeps its parked fraction. A hand point at 100 cm is pulled back to 60.
Done (change `pair-a-double-door`): a single shut door stays shut at exactly 0.05 rad short of closed and leaves once it is farther, while still being held. A paired door whose partner is not within 0.1 rad is not held, and it leaves when its own angle is short of closed. Exactly 0.1 rad short is not near. A paired door adds the pair bits to its layer.
Done (change `unlock-a-door`): an unlock clears the saved latch and waits 1000 ms, on the door and on its partner, while both stay latched. At 1000 ms they are still latched. At 1001 ms both latches open. Which key fits the door is not in.
Done (change `step-a-lever`): a lever given arm velocity 1 creaks at volume 0.1 and pitch 0.75. From 0.2 rad the drive is 0.18 toward the first detent. A return from 0.5 rad steps to 0.52 toward the second. A forced lever at 0.4 rad steps to 0.38 toward the off detent. Arriving at the first detent from the second signals 0. Arriving at the second arms a timed return and signals 1. Forced and inverted, the on detent signals −1. A linked light fades out without a signal. The return's later tick is not in.
Done (change `step-a-plate`): a pressed plate at travel 0 rises to 0.02 and grinds at volume 0.81, pitch 1. A travel of 0.68 stays up. A travel of 0.69 clicks and signals 1. A down plate falling from 0.51 stays down. Falling from 0.5 releases and signals 0. A one-shot stops being pressed once a travel passes 1. A body standing on the plate is not in.
Done (change `dispatch-a-signal`): a missing target does nothing. A door closes on 0, opens on 1 or 2, and toggles on −1: an unlatched leaf closes and a latched leaf opens. An inverted door opens on 0 and closes on 1, and −1 still toggles. A door wins over a fire job on the same object. A fire of size 200 starts only when the value is not 0. A light whose stored word is 16 becomes 17 on 1 and 16 on 2. A relay of kind 4 is ignored. A plate is pushed exactly when the value is not 0. The relay bodies are not in.
Done (change `combine-relay-inputs`): a combiner sets bit 0 on a value of 1 and signals 1 when the state equals the code. The silence flag writes the bit and sends nothing on a mismatch. The invert flag sends 0 on a match. The second combiner forwards the original value to the source's second target when the state's low bit is clear. A pair sends the same value to both targets. Seven sequence steps signal the second target. Eight clean steps signal the first. A repeated mark flags a wrong order. The broadcast across every sector is not in.
Done (change `broadcast-a-relay`): a broadcast signals a tagged door whose link equals the relay, including the source. A door link that does not match is skipped even when the operative target matches. Without a door link, an operative is signalled when its target equals the relay, except the source itself. An untagged id is skipped. The two hits keep list order and carry the original value. Carrying out the close is not in.
Done (change `close-and-lock-a-door`): a motorised hinge at fraction 0.2 is set to 1 and driven shut for 300 ticks, or 100 when fast. A shut hinge that is not motorised stays idle and becomes latched. An unlatched sliding leaf plays the lock sound and does not start a drive. A shut fast slider drives for 100 ticks and becomes latched. The lock sound's random pitch is not in.
Done (change `signal-open-a-door`): a signalled open plays the unlock sound once when there is a door under the node, and plays nothing when there are none. A latched door and a motorised door each wait 120 ticks. A door that is neither does not wait. The 120th tick ends the wait. The open that follows is not started by this wait.
Done (change `open-when-the-wait-ends`): the 120th tick of a signalled wait opens the door. The latch, the shut flag and the pull count clear. A plain hinge goes idle and is shoved 3 cm along local Z. A motorised hinge at fraction 0.4 is set to 0 and drives open for 100 ticks, with the same shove. A plain slider jumps to 0.6 for 80 ticks and wakes. A motorised slider keeps fraction 0.4 and drives open for 300 ticks. The unlatch sound's random pitch is not in.
Done (change `toggle-a-door`): a shut hinge toggles open, aim fraction 0, for 100 ticks, or 60 when fast, and wakes for 256 ticks. An angle 1 rad short of closed still opens. An angle 1.1 rad short closes. An open slider closes for 60 ticks. A slider at travel 0.3 opens for 60 ticks and clears the shut flag. A latched hinge is shoved 3 cm and then aims open. A latched plain slider jumps to 0.6 and then opens for 60 ticks.
Remaining: the lid in
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
capsule carries the body only across the ground and turns it. The
support pins (change `pull-the-support-pins`, C14) pull the two thorax
pins and the pelvis pin toward the frame in each iteration. A
get-up no longer hops (the end of `Get Up 2` overshoots 3–5 cm and settles
in 45–100 ticks, from 10.7 cm ringing 90). Done (change
`bend-the-legs-to-the-ground`, C13): a standing leg bends onto its bend
target by the law of cosines (30° when the cosine is above 0.866, 180°
when it is below −1). The correction from the clip advances at most 0.02
rad a tick (0.2 while a recovery step is on) plus its stored rate, and
the muscles follow it on the next tick. An unanchored target is the
anchor lowered by twice the lift term stored before clamping (2.25 cm
when a standing body is at its target height). While stepping, the thigh
turns 75% of the hip-to-target angle and an anchored foot takes the step
lift. The controller-floor stair coupling is gone: the lift reads each
foot's own ground, so a controller 12 cm up does not raise the target,
and a synthetic walk up four 18 cm steps keeps balance at or above 0.9
with its feet on the landing. Gait plants are not in yet (C19).
Done (change `anchor-the-character-frame`, C9): the
pose is put on the original's character frame, whose origin is the pelvis
less the clip root's offset across the ground: it rides on the pelvis
while the body moves, follows it exactly without support, within 5 cm
while a standing body's support is under 0.95 and not at all from 0.95,
within 5 cm while down and 10 cm as a get-up starts. The support pins,
not a pelvis spring, hold a steady body on its stance spot; an unsteady
one stays about where it is pushed, within 2 cm of where the frame puts
its pelvis. The capsule follows the frame
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
([formats/RSQ.md](formats/RSQ.md), Motion playback). Done (change
`blend-the-run-over-the-walk`): forward locomotion plays Walk, and each
bone is slerped toward the Run sample at the same keys by 0.7 times the
run blend before the pose hold. The root stays the walk's, and the jog
clock rate is unchanged. Done (change `weight-the-sprint-overlay`):
Shift with the cursor further than 3 m keeps the sprint latch, and the
overlay uses the run blend itself when that latch is kept, the stance
level is above 0.9 and the speed is above 3.5 cm a tick. A full sprint
meets the run bone; level 0.9 and 3.5 cm a tick stay at the jog weight.
Done (change `clock-the-sprint`): while sprinting, the forward clock
blends from the walk rate toward the feral sprint row by √run, then
multiplies by 1 − 0.1 × encumbrance. A feral level of 0 at run blend 1
and 4.337 cm a tick gains 0.727 frames; feral 1 gains the jog's 0.894;
encumbrance 0.5 gains 0.691. The capsule still moves at the jog speed.
Done (change `step-the-gait`): the capsule's sprint speed is the jog
speed times the feral sprint row's thrust over the jog thrust; backing
plays Walk with the negated backpedal clock; a walking foot plants when
its phase is 0.2 to 0.4 and lifts at 0.55; a blow whose impact is over
0.5 starts a knockback step of 2·(20·strikes + 10)·J cm along the swing
plus 20 ticks of chest velocity, capped at the step length.
Done (change `tighten-the-run-cap`): encumbrance 0.5 with Shift and the cursor 4 m ahead tops the run blend at 0.8 and keeps the sprint latch. Shift with the walk key and no cursor tops it at 0.85 and spends the latch. Still to do:
the locomotion clock's stride terms, and collisions
between bodies (and their wake of a sleeper).
Done (change `step-the-cloth`): a cloth particle keeps 0.98977 × 0.5^(1/83)
of its velocity, dragged 5% toward the mean, then falls by the body's
gravity. Wind, a pin and one edge pass follow.
Done (change `collide-and-sleep-the-cloth`): a sphere or a level plane
pushes a vertex out to 2 cm of padding, sleep counts down while the cloth
is still and never below its owner, rustle plays when the squared speed
sum exceeds 400, and the mesh vertices are the particle positions.
Done (change `sweep-cloth-collisions`): a body sphere swept through a cloth
triangle pushes the vertices it reaches, and a vertex swept through a level
triangle stops 2 cm short of it. A sweep that misses leaves the cloth where
it was.
Done (change `build-a-cloth-mesh`): a triangle mesh becomes one particle a
vertex, a link on each edge and a bend link across each shared edge, and
both windings. The 16×16 wind tile is sampled in 2 m cells. A collision
returns the opposite of the vertex pushes to the body.
Done (change `rank-cloth-pins`): each vertex stores its distance along the
mesh from the nearest pin, and the draw hook culls the front and then the
back. The distance is not the straight line.
Done (change `submit-cloth-draws`): each submitted pass becomes a material
whose pipeline culls that face, the front and then the back. A single-sided
cloth submits one material that culls the back. Spawning a mesh for every
cloth marker is not in.

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
compare between builds ([DEBUGGING.md](DEBUGGING.md)). The stance (M38)
and player input still run ahead of the character ticks, over every body,
and the AI once a frame; each joins its stage with the change that
rewrites it. The condition, the collapse's dying tick and the falls (C11)
run in each character's posture stage, the swing routine (M22) in its
attack stage, and the condition's clip-end writer in its sampler stage.

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

Done (change `fall-and-get-up-as-the-original`, C11 of the master plan):
bodies lose and regain their footing by the original's posture block, from
last tick's balance. Off balance (under 0.6) an upright body takes the
Defence stance (still) or Rebalance (moving); leaning further it staggers
(a get-up started partway, which holds the clip's first lying key until
its clock reaches it, as the original's does) or falls forward or
backward when the lean is sideways, fast, lost or far; under 0.3 it is
down, and lying still it gets up once its feet give support or 100 ticks
after they lost it, with Get up from its front or Get Up 2 from its back
(by its pelvis's front axis, with ±0.2 of hysteresis, picked again while
the get-up is under 40 %), turned the way it rises when more than 18° off,
blending 20 frames from its whole physical pose; getting up its clip runs
(1.5 − 0.5·encumbrance)·0.6 a tick and its energy recovers faster. Unsteady
on its feet (0.6–0.8) it takes the Defence stance or Rebalance, whose wait
holds the posture switch off; under 0.8 it turns toward its fall. Exarust's
always-fall into Unconcious 1, its 2.5 s, energy and speed gates and its
first-pose matching are gone. The condition, the collapse's dying tick,
the falls and get-ups and the fall check now run in each character's
posture stage, the health bookkeeping in its upkeep and attack stages, and
a body that dies in its posture stage skips the rest of its tick
([formats/RSQ.md](formats/RSQ.md), Falls and get-ups). `--shove 60`: the
dazed player Rebalances, falls on its back, gets up with Get Up 2 1.3 s
later and stands 1.9 s after that; `--shove 90` staggers into Get Up 2.
The recovery step (C15) starts when the chest leaves the feet, moves one
foot by the stance score, and ends by its own rules. The thorax lead follows the horizontal error from the pin to the posed
target plus the step offset: 0.7 (0.4 while Idle plays) times
(direction + 0.5) times (2 − 25·s) times 1.2 times the square root of
body scale times the stance level, with direction
(h²/60 + 0.25)·0.36·forwardness. A still 40 cm sideways error moves the
target 33.6 cm. The standing leg bend is in (C13). Gait walking is not
(C19). The support pins (C14) are in.

Done (change `plant-the-feet`, C12 of the master plan): the feet stand on
the ground as the original's do. Each leg's foot contact (its ankle) has a
state (touching, anchored for a step, reached, lifted, no ground), an
anchor on its ground plus the sole height, a grip that rises to full over
0.6 s and its lower leg's ramp; a standing body's feet are planted where
they meet the floor and gripped there by the original's PD grip in each
iteration (a walking body's by a velocity servo; a lifted foot is only
pushed up), the toe held over the floor, the lower legs' and feet's
friction set by state (0, 2.0, 0), and a foot whose stance spot drifts
5 cm off steps there, one at a time. Getting up or unsupported the feet
stand on their stance spots, lying no foot touches, and the get-up's start
clears them. Corrections to the notes: condition 1 is walking, so standing
and combat feet take the PD grip and a standing probe's sphere is 5 cm.
`--trace` fills the foot contacts' columns and adds the ankles; the body
overlay draws the anchors ([formats/RSQ.md](formats/RSQ.md), Feet). The
footwork's stance scores and timers (C15) choose the foot. The standing
leg bend is in (C13). Not yet: the walking gait's plants and the closing
step (M16, C19), the locomoting bend, and the sole's orientation drive.
Standing feet keep within 0.5 cm of their anchors; a
heavily dazed body now sinks onto its planted feet instead of toppling.

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
Done (change `classify-a-click`): a free hinge is a door, a dead character a corpse and a living one a person. A prop lighter than 0.08 is ignored, and a heavy one is ignored below strength 0.9. A point past 70 cm is turned toward first. Reach is 245 cm high and 300 cm away, or 300 cm high and 350 cm away for a door.
Done (change `limit-a-drag`): a drag pulls along the cursor direction by the nearest hit inside 1000 cm. Hits of 1200 cm and 400 cm pull 400 cm. A box centred at 270 cm with 10 cm of half-height is held, and 20 cm of half-height is refused because a corner passes 280 cm. A corner on 280 cm or on −1000 cm is kept.
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
blow; live 0.9–1.4 s at like energies). The stagger step's length and
target are in. The footwork that walks that target is in, and the
dying tick calls it. The held weapons are dropped when the collapse
starts and again on the first dying tick if they are still held.
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
Done (change `hold-weapons-by-the-grip-table`, L12 of the master plan):
every held item sits under its hand by the original's grip table,
replacing Exarust's invented 90° turn: the hand class from the held slot
and the weapon class (fists, one-handed and the first of two weapons,
two-handed, polearm, claws, the off-hand item and the second of two
weapons), the grip's hand (a polearm in the left), its offset times the
character's size (the ogre's own, unscaled) and the race's held offset
(the demon's 9 cm), its Euler angles (about X, then Z, then Y of the
hand) and a flipped item's half turn; all 208 shipped weapons have a
grip. The one-handed sword is held low, forward and across the hips; the
two-hander upright before the chest as in the live retail capture; the
torch keeps its grips (now shared) ([PLAYER.md](PLAYER.md), Held items
and their grips). Still missing from it: the grip links and modes (with
weapons as bodies, M24/M25), the record's flip bit, second weapons.
Done (change `equip-and-block`): a worn item takes off whatever overlaps
its occupancy; displaced braies are dropped and return when the groin is
bare; a container and a pack exchange an item by moving its id. A parry
stops the blow. A shield that is not parrying deals yellow
0.05·(1 − stamina)·(0.4 + 0.4·impact)·leak and no lasting wound.
Done (change `place-items-in-a-pane`): an inventory pane places icons at
pixels. A drop from outside adds the icon and its bulk and clamps it; a
drop from the same pane or the linked pane only moves it, and a linked
move is not refused for capacity. Ending the drag removes the icon. The
hit is the top-most rectangle.
Done (change `mark-a-container-drop`): a shared reject bit is refused. A required mask must match. An own-only pane takes only an id it already lists with the low bit set. A limit of 0 does not refuse for size, and a capacity of 0 does not refuse for bulk.
Done (change `choose-a-card-page`): an id below 1, a container, a missing model, and a mark of 0 have no card. A map has no stats page. Seven coins print 1, and the high bit keeps 7. A withheld weapon of armour 6 is apparel, armour 0 is a shield, and armour 7 is plain. Three coins print 0.
Done (change `show-a-weapon-bar`): slash 5 and pierce 0 are absent. Crush 6 is 0.3. Thrust 20 is 1.
Done (change `show-an-apparel-bar`): coverage 5 is 0.25. Impact 5, crush 0, and encumbrance 5 are absent. Slash 6 is 0.3. Pierce 20 is 1.
Done (change `place-a-card-icon`): a plain card of span 268 and content 160 places an icon of 160 at 54, 16 with a unit of 4. Content 100 on a span of 269 places it at 54, −14. A stats picture of 40 or 44 stays at −28, 4. A picture of 45 is −27, 4. A picture of 160 is 88, 4.
Done (change `show-a-shield-bar`): weight 5 is 0.05 and impact 20 is 1. Encumbrance 0 is absent. Encumbrance 1 is 0.05. Flags 0x02000000 store weight 1, impact 0, and encumbrance 8.
Done (change `show-rank-in-the-arena`): menu 0 entering arena 8 turns rank and points on. Arena entering story turns them off. Story entering practice turns them off. Staying in practice, leaving mode 6 for practice, and leaving arena for mode 1 keep them on. Skill 0x21 is Aspirant and 0.5. Skill 0xc0 is a dash. Skill 0xe0 has no name. Skill 0x1f is 15.5. A closed gate shows neither.
Done (change `pick-a-description-phrase`): value 11 misses a row of 0 to 10 and keeps roll 7. Value 5 turns "fine" into " fine." and the roll into 1. Roll 16 chooses " beta." and becomes 0x80884051. "fine x" and "Fine x" add nothing before "fine work". "fin x" and a bare "fine" become " fine work.". A comma phrase becomes ", but torn.". "the x" becomes " the item here.".
Done (change `wrap-a-weapon-sentence`): "ab" stays "ab". "sword" is "A well sword sharp." at roll 0x08088406. "edge." is "An edge edge sharp.". Quality 0 on "Sword [x]" is "SwordIt is well, but sharp.". Quality 131 is "SwordIt is well and sharp.". "Blade [c]" is "BladeIt is sharp." at roll 1. "-blade" is "A prime blade keen.". A missed flawless row uses "ok".
Done (change `edit-a-fixed-name`): "ab" stays "ab". "Sword" stays "Sword". "axe" is "An axe". "Zed" is "A Zed". "zed" is "A zed". A mark between "ab" and "xy" becomes "An aby". A mark at the end of "ab" becomes "An ab". A leading mark before "cde" becomes "A de". "Za" and a mark before "bc" becomes "A Zac". The roll does not step.
Done (change `wrap-an-apparel-sentence`): "ab" stays "ab". "axe" is "An axe". "coat" is "A well coat sharp." at roll 0x08088406, and material 1 names the side "Wool line" without changing that text. "edge" is "An edge coat sharp.". Quality 0 on "Coat[p!]" is "CoatThey are well, but sharp.". Quality 131 is "CoatThey are well and sharp.". Both high marks stay ", but". "Coat[x!]" is "CoatIt is well, but sharp.". "ax" plus a mark plus "e[pc]" is "ax" plus a mark plus "eIt is sharp." at roll 1, and the side stays. "a coat" is "A well a coat sharp.". "coat." is "A well coat sharp.". "A coat. lined" is "A well coat sharp. lined". Material 9 on a mark between "ab" and "xy" is "A well aby sharp.". Material "wool" is "A well abwool y sharp." with side "Wool line". "oak" before "cde" is "A well oak de sharp.". Fourteen missed rows hide a fifteenth "yes", leaving "A  coat ok." at roll 1.
Done (change `stick-a-weapon`): an unarmed bite sticks nothing, and a depth of 0.1 sticks nothing. A depth of 0.25 uses flags 0x11000, strength 3, smoothing 0.6, count 60 and span 41.5. A float depth of 0.4 uses flags 0x211000, count 99 and span bits 0x4284cccd. A depth of 1 uses count 249 and span 166. A depth of 3 matches that count, span and strength, with smoothing 1. A unit of 2 at depth 0.11 uses count 54. The weapon layer is 0xC0. A part layer of 3 with a parent of 0 and children 1 and 0x40 becomes 0x43, 0x40, 0x41 and 0x40.
Done (change `parry-unarmed`): a mid swing at clock 0.5, in front and within reach, is parried left-down with blend 20. Clock 0.2 is refused unless the combo flag is set. Yellow 0.009 is refused. Size 1.3 is refused unless the defender has a shield. A blow behind, and one past reach plus 150 cm, are refused. A down swing whose table height is at or below the chest difference is left-down, and one above it is left. A bite of 0.25 holds the two parts together in the solver. A bite of 0.1 opens no grab and the parts stay apart.
Done (change `arm-a-parry-grip`): a cleave is refused. Handling 0.5 inside 80 cm has speed 0.5, blend 10, and strength bits 0x3ecf5c29. At 165 cm the strength bits are 0x3e4f5c29. At 250 cm and beyond the strength is 0. A passed blow has speed 0.25, blend 5, and flags 0x100000. A shield of weight 0.8 and stamina 0.5 has speed bits 0x3f4ccccd, blend 16, strength bits 0x3df5c290, and flags 0x800. Slot 1 is used only while attacking outside motions 8 to 10 with two weapons held.
Done (change `recoil-a-striker`): a recoil sets strength and the stall rate to 0.5 and clears the released bit. A class word of 0x90 with the continuation bit, or with clock 0.4 and a counter, stops there. A released swing with the window bit and drive 1 sets flags 0x27, ends at 1240 from a clock of 1000, uses blend 40, and sets both arm stiffnesses to bits 0x3e4ccccd. Drive 0.013 ends at 1200. A follow-through with that same skip writes nothing. Gain, recoil and speed 1 at zone 0.25 set strength bits 0x3d99999a and end at 1060. An edge sets strength bits 0x3eb9999a. Strength caps at 1. A negative gain ends at the clock. Speed 0.009 ends at the clock.
Done (change `stagger-a-collapse`): a body that is already stepping takes no stagger. Reach 80 at balance 1, tone 1 and scale root 1, with chest velocity 3 and 4, has length bits 0x42a00000 and target bits 0x42d7faf0, 0x42be0000 and 0x430ffca0. Reach 0.01 at balance 0.4 has length bits 0x3b83126f. The next lower balance has length bits 0x3b83126e. A zero velocity leaves the chest. Landings and the step kind are 0.
Done (change `walk-a-stagger`): a target 2 m ahead pulls the stance origin to z bits 0x3f6147ae and anchors the first foot there, state 4. A target at 0.5 m stays at z bits 0x3f000000. A target on the frame moves no foot. The stagger target from reach 80, balance 1 and velocity 3 and 4 pulls the origin to bits 0x3f072b02 and 0x3f343958 and anchors the first foot at bits 0x3f53f7cf, 0xbe4ccccc and 0x3f343958.
Done (change `walk-the-dying-tick`): the dying tick walks that stagger from the measured chest. A chest recorded at the origin, then a metre along z, then there again, anchors the first foot on the first tick and leaves it on the second.
Done (change `drop-weapons-at-death`): the dying tick drops both held weapons. Mobility 1.25 becomes drag bits 0x3e600000, friction bits 0x3f400000, and the striker flags clear. A velocity of 1.5 stays. The hands, the class word and the primary are empty. A second tick adds nothing. An empty pair stays empty.
Done (change `own-a-dropped-weapon`): the dying tick makes weapon 3 its own owner and copies that owner onto child 7 and grandchild 8. The parent list 5, 3, 8 becomes 5, 8. Roots are 3, iterations are 4, links are 0, grip 4 becomes -1, and the awake byte is 0. Child 7's mark is -2. The child's iterations stay 11. Weapon 4 keeps grip -3, mark 4, parent list 9 and iterations 2.
Done (change `step-a-dropped-weapon`): the dying tick puts a solid weapon of mobility 1.25 into the solver, awake at 256, with friction bits 0x3f400000 and its sphere. Velocity 1.5 is kept. One solver tick leaves the first particle at bits 0xbe3a7bb7, 0x3f7fa2b5 and 0xbe4ccccd. A flat shape spawns nothing. A second tick adds no body.
Done (change `meet-a-dropped-weapon`): that weapon's sphere, starting on another body's sphere, meets it in one shipped body step. The contact names both bodies, has one hit, and its deepest bits are 0x3ec39cd9. The weapon's first particle is at bits 0xbda3ae15, 0x3f7e43be and 0xbe492d0c, not the place one step alone leaves.
Done (change `notice-a-deep-wound`): a lasting wound of bits 0x3daaaaab is not noticed. The next float is. A non-number is not. A chest cut through the hit path records the notice.
Done (change `grant-a-second-wind`): a granted wind whose ready clock is 11, at game clock 12, fires, waits until 180012, and writes no heartbeat message. Volume bits are 0x3dcccccd and pitch bits are 0x3f800000. Ready clock 12 waits, and the heartbeat message is written.
Done (change `heal-a-second-wind`): an attacking fighter with yellow bits 0x3d000000 and an active wind ready at 180012 gains yellow bits 0x3d013a93 at game clock 12 and at 3012, and stays active. At 3024 it gains the same bits and the wind ends. A quiet wind and a dead fighter keep bits 0x3d000000.
Done (change `name-a-practice-partner`): a novice fight at count 0 becomes count 1 named opponent01 and is not last. Red bits 0x3e4ccccd with maximum 0.25 become 0x3e666667, and yellow copies them. Count 7 becomes 8, named opponent08, and is last. Count 8 stays. An expert count 0 is named opponent09. An expert count 7 is count 8, named opponent16, and is last, and red bits 0x3e75c28f become 0x3e800000.
Done (change `knock-a-fighter-out`): flags 0x80000150 at clock 1000 with delay 40 wake at 1040, are down, and have flags 0x80000130. The statue clears, gait is 0, tone is 0, and the pin bits are 0x3da3d70a. A countdown of 300 stays. A lower countdown rises to 256. The maximum clock plus delay 1 wakes at the minimum clock. A blow does not call this. No limb stores an impairment. Still to
do: spawning the practice partner.

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

Done (change `sense-a-contact`): a contact is seen out to 12 m (18 m once
tracked, 6 m asleep). Hostility with neutral traits settles near
0.0375 times the size of a negative attitude, and that heat chooses
notice, watch, engage or flee. A duellist that engages takes the guard.
Done (change `choose-the-fight-blow`): once the fight has started, an unarmed fighter inside eight tenths of reach can swing overhead and outside it can take the other side. An armed fighter stepping forward thrusts when its preference wins, and takes the overhead at age one half. A facing score above 4 takes the other side. Energy under eight tenths of the maximum refuses the swing. The duellist starts that blow.
Done (change `score-a-fighting-place`): reach 50 cm, size 1 and a step of 80 cm score a strike of 0.25 at 25 cm and 1 from 120 cm through 130 cm. At 600 cm the strike term is 0 and the fighter keeps closing. A spot 1 cm past reach plus 150 cm has a close term of 0. Outside a fight the close term doubles and the strike term is 0. A large opponent's ideal is 50 cm longer, so 100 cm scores 1. Wanting melee doubles a strike of 0.25 to 0.5. The duellist stands at 1.20 m and walks at 2 m. The attack-arc danger is not in.
Done (change `score-an-attack-danger`): a side swing on its arc end at phase 0.5 scores danger bits 0x40000000 at skill 0 and bits 0x40800000 at skill 1. Phase 0.35 scores bits 0x3ff28f5c. A thrust inside its arc at 100 cm scores bits 0x3f000000, and one outside it scores bits 0x40800000. At 200 cm, at phase 0, and when the side is not sweeping, danger is 0. A held weapon keeps the side-swing danger past that limit. Outside a fight the strike term is 0 and that danger stays. Feral 1 halves the close term and scores no danger. A dull feral fighter keeps the close term. The approach term and the recovering close tweak are not in.
Done (change `approach-a-fighting-place`): at 25 cm and at 200 cm, reach 50 cm and skill 0, the approach is bits 0x3ec00000. A wide fighter at 25 cm scores bits 0x3f88b000. At 400 cm, and outside a fight, it is 0. A recovering swing at 25 cm scores close bits 0x403c0000 and approach bits 0x3fc00000. Action 5 at phase 0 publishes kind 1, start bits 0x3fdf66f3, end bits 0xc01c61aa and look bits 0xbeb2b8c4. At phase 0.5 a hand on the side sets the end to bits 0x3fc90fdb. A recovering swing of that action publishes kind 4. The facing scale and the idle-action clock are not in.
Done (change `scale-a-facing`): a fighter who is not attacking, with the candidate beside it, scores approach bits 0x3f400000 at 25 cm. Straight ahead keeps bits 0x3ec00000. A press with the opponent facing aside scores strike bits 0x40300000 and approach bits 0x40840000. An armed action outside a swing publishes kind 4 when the idle stamp is 5 and the clock is 4, with both edges on the heading. Stamp 4 against clock 4, and stamp 0 against the maximum clock, publish kind 0. A dead body is knocked out at clock 50 with no delay and is down. A walking body stays walking. A blow does not call that entry. The part rewrites that precede it are not in.
Done (change `rewrite-a-dead-part`): a dead entry sets a held part's mark to 0x2001 from 1, its gate to 6 from 15, its span bits to 0x40000000, and its identity turn to 0, 0, 0, 1. An unheld part keeps mark 3 and span 9 and still takes gate 6. A negative basis takes turn 0, 0, 1, 0. A link of count 1 becomes kind 4 and bits 0x30. Count 0 stays. The marker clears. Loose 2 times 0.5 is bits 0x3f800000. Posed and the behaviour mark become 1. A step base of 4 becomes 2. The body is then down at clock 50. A walking body keeps its parts and stays walking. The race setup and the three calls after the part loops are not in. The characters section stays 27%.
Done (change `finish-a-dead-entry`): when both pair words are 0, class 0x800 and body kind 0x100 set the hands to firm and open, leave the bare mark on, clear the class word, and set the primary to -1. Class 0x500 marks the hands and clears the bare mark. A pair word of 1 leaves the hands. Feral 0 and feral -1 with a base of 1 score reach bits 0x42a00000. Feral 1 scores bits 0x42600000. Empty slots fill. An offset of 3, 4, 0 faces along x. A walking body skips this. The virtual call and the weapon pass inside the step are not in. The characters section stays 27%.
Done (change `hear-a-disturbance`): a disturbance above 0.3 is walked to. An alert of 0.2 or more faces a target more than 45° off, and otherwise looks around after 0.7 to 3 seconds. A quiet cell stays on the idle.
Done (change `idle-roam-patrol-sentry`): a roam waits, walks to a wander point, waits 500 ms plus a draw of 2000 ms, walks toward home, then waits 1000 ms plus a draw of 4000 ms. From a generator at 0, an arrival at 2000 ms waits until 2500 ms and a home arrival at 3000 ms waits until 4125 ms. A patrol only repeats the wander. A sentry holds the guard pose. A disturbance during a wait is walked to and the phase stays.
Done (change `pass-a-door-on-the-path`): a walker with no path, a cursor at the end, or no door keeps walking. A slider at travel 0.29 is more than 0.7 open and is left alone. A shut hinge holds the walker and is toggled open for 100 ticks. A slider at travel 0.3 holds the walker and opens for 60 ticks. A hinge already toggling is held without another toggle. The rest of the path is not in.
Done (change `give-up-a-door-wait`): a walker that is not waiting at a door stays as it is. At exactly 10 seconds a shut hinge is toggled and the wait still holds. One millisecond later the walker seeks again, the avoided node is cleared, and cursor 3 skips bit 3, while the search stays held and the hinge is not toggled again. A slider at travel 0.29 moves the walker past the door and lets the search run. One millisecond after 10 seconds that slider seeks, the search is already allowed, and cursor 32 skips bit 0. A start near the top of the clock does not give up at a small now. The heading sway is not in.
Done (change `arrive-at-a-span`): a walker that is not approaching a span stays as it is. A busy approach at its stamp stays there. Exactly 250 cm from the midpoint starts the door wait. A missing span seeks and sets only the high skipped bit. A later miss 251 cm out also skips cursor 3. The midpoint one millisecond later starts the door wait at that clock. The aim snap is not in.
Done (change `sway-a-wander-look`): a look that is not wandering stays as it is. At the clock, from extracted angle 1, the look eases to 0.91 and the offset stays. One millisecond later a unit draw of 1 sets the offset to 1.8, steps the roll to 134775814, and sets the timer to 5126. A heading of 4 at posture 1 eases to about −0.0685. The flag call is not in.
Done (change `snap-an-aim`): an aim snap copies the body's place. A heading whose height is 9 and whose forward part is 1 has yaw 0. Roll 1 steps to 134775814 and the timer becomes 3094 at clock 2000. The pace word and the body's marker are cleared. A defence clip starts Idle over 30 frames. A walk clip stays. Idle already playing does not restart. A wander look at that clock keeps its offset.
Done (change `pick-a-wander-point`): a walker with no room does not pick. The only exit from the origin toward a room point 100 cm ahead aims 25 cm forward, at height 0, with a stop of 20 cm and a radius of 150 cm. An exit 100 cm deep toward the origin aims 75 cm deep. An exit that leads back is drawn again, and that second exit 200 cm ahead aims 150 cm forward. The only exit that leads back is kept. Follow and sleep are not in.
Done (change `follow-a-leader`): no leader stands when there is no activity or panic is at least 1, and otherwise runs the activity. A leader facing the follower closes inside 150 cm and does not at 150 cm. The band is 400 cm, 280 while following, and 1200 while busy. Urgency is 0 at 500 cm, 1 at 801 cm, and 2 at 1601 cm or when the leader sprints. A planned spot 100 cm aside does not replan. 101 cm does, and the plan stops at 50 cm. A hidden follower who is not walking stands. Sleep is not in.
Done (change `lie-dormant`): a walking body whose flags are the shock stagger and whose tone is 1 sleeps into the dormant condition. The stagger flag stays and the dormant flag is set. Tone and the tone target become 0. It is a statue with gait 2. The tick after that changes nothing, and a second sleep keeps it. Waking when a target is seen is not in.
Done (change `wake-a-sleeper`): a sleeper whose flags are only the dormant flag stays asleep until it sees a contact. At clock 2500 that contact drops role 44 and knocks it out: down, the dormant and waking flags, no statue, gait 0, tone 0, pin gain 0.08, and the sleep countdown raised to 256. A sleeper that also has the shock stagger stays dormant. The tone rise after the knock-out is not in.
Done (change `rise-after-a-knock-out`): through 1500 ms the tone stays 0, the clip base is 0.4, and the condition is down with both flags. At 1501 ms the tone is 0.001 and the dormant flag clears. At 1502 ms it is 0.002. Getting up at phase 0 adds 0.002, and at phase 1 adds 0.001. A held body keeps the dormant flag. At 8000 ms the clip base stays. At 8001 ms a clip base of 0.995 finishes at tone 1, the target becomes the stance factor, and the flags clear unless held. A body that is not waking is unchanged.
Done (change `awaken-and-refresh`): the tick that drops the dormant flag tells the one person who can see, and not the person who cannot or the non-person who can. The next tick tells nobody. When the clip base reaches 1, a 40 cm weapon and an 80 cm weapon reach 80 cm and prefer thrust 6. With no hand bit, a weapon whose slash, crush and pierce are 0 uses the race reach of 50 cm and prefers 10. The witness memory update is not in.
Done (change `remember-an-awakening`): at 1501 ms and second 100, a seeing person who needs two hits counts 1. One already counting, with a 2 second delay, stamps 102 and sets the high bit. A second witness bit stamps 100. A last use of second 1000 stays, as do a person who cannot see and a non-person. The next tick leaves the count. A ten-second cooldown stays at second 1009 and counts 1 at second 1010. A never-again nibble stays. An activated stamp inside a ten-second window refreshes to 109. At the window, with a 2 second delay, it re-arms at 112. A future stamp stays and reports updated. Held-off leaves an activated word. A never-again window at the last second refreshes to that second. The grip pass, the arm-mass scale, and steering along an accepted path are not in.
Done (change `hold-a-refreshed-weapon`): at 8001 ms a ghoul holding fists is held again. Its balance is 1, its arm stiffness is 1, its hands weigh twice, and its reach is 50 cm. A Fael whose unarmed reach is 150 cm weighs four times, reaches 150 cm, and sets arm stiffness to one half. Steering along an accepted path is not in.
Done (change `step-a-carrot`): a ground point 50 cm behind a 200 cm segment keeps fraction -0.25 and aims 50 cm along. A point on a corner keeps the first segment, and the goal is 20 cm up and 100 cm along the next. A vertical segment is skipped and the goal is the last point. Exactly 100 cm of segment left has no goal, and the cursor stays at the start. The out-of-scene slide is not in.
Done (change `slide-out-of-scene`): with the low scene bits clear, the start of a 100 cm segment becomes fraction 0.02, the ground moves 2 cm, and a pelvis base of 90 raises only the standing height. Fraction 0.75 on an 8 cm segment with 8 cm after it becomes -0.28125, and the ground is at 5.75 cm. A vertical segment counts as length 0.1 and crosses behind the next start. One centimetre from the end of a last segment stops at the end. A set low bit does not slide. The run bit and the soft stop are not in.
Done (change `clear-a-run`): urgency 0 sets the run bit off the last segment, and on it while the carrot fraction, read as a double, is not past 0.99. The f32 value 0.99 is past that double. Ten centimetres from the carrot stops, sets the stopping bit, and keeps the carrot. Exactly 30 cm clears the run bit and does not stop. Forty centimetres up does not stop. A standing point 100 cm above the carrot pulls the goal 20 cm along the facing. Urgency 1 stops at 99 cm from the last point and does not at exactly 100 cm. Urgency -1 sets the run bit. The line walk is not in.
Done (change `walk-a-line`): a line across an open floor from cell 1 to cell 6 keeps its end. A floor of 40 at cell 4 blocks before that cell and keeps the asked height. Starting on the cell before that floor keeps the start. In the scene that blocked line clears the run bit, asks for a re-plan, and leaves the carrot. A clear line keeps the run bit. Out of the scene, and a higher scene bit alone, the line is not walked. The straight walk and the nearest walkable point are not in.
Done (change `query-the-grid`): a floor five above the current height is open to a straight walk and blocked to the line walk. Six above blocks the straight walk. A drop of four makes a later rise of six block. A high floor one cell ahead is not an obstacle, and two cells ahead is. A penalty of 61 on an early cell is an obstacle. A penalty of 60 is not, nor is 61 on the last cell. A point off the centre of a cheap cell keeps its place and takes the floor height. A penalty of 80 steps to the neighbour toward the origin. Footprints are not in.
Done (change `stamp-a-footprint`): a footprint at height 1 adds 63 to the centre, 32 to a neighbour, and 16 to a cell beyond. The obstacle test then fires on a line through the centre, and removing the footprint clears both the penalties and the test. A centre in the outer five cells is not stamped. At height 5 the upper floor takes the 63 and the lower stays 0. The mover refresh is not in.
Done (change `start-a-long-path`): an empty path and a single point are not followed, even when forced. Two points exactly 150 cm apart are not followed. Two points 151 cm apart are followed. Two points on top of each other are followed when forced. Steering along the points is not in.
Done (change `refresh-a-mover`): a plan through a footprint costs less than the same search with the footprint still down, and the centre is 63 again, with 32 on a neighbour. A zero-extent box covers its cell and a neighbour, inside the margin of three. A rectangle on cell 12 joins the one on cell 10, and one on cell 17 stays apart. Seventeen separated rectangles keep the newest sixteen. A sample at voxel 20 raises a lower floor of 10 to 20 and the cost pass runs. Voxel 30 raises the upper floor to 30 and leaves the lower floor at 10. A sample of voxel 12 on a floor of 20 whose underside is 15 raises the upper floor to 12. Voxel 255 and a sample outside the grown rectangle do not. The scene-wide countdown is not in.
Still open: follow, sleep, and where the wander point is picked.

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

Done (change `run-an-arena-career`): a new company starts with 343 copper
(2 gold, 4 silver, 7 copper). It hires seven combatants and one of each
specialist. A standard match takes its fee; a win pays the prize;
experience is the template amount times the damage share, plus half with
a trainer.
Done (change `advance-the-story`): a story loads `exanima01` and advances
through `exanima08`. A topic can be chosen once. The journal opens on
"New Entry" and stores the written text.
Done (change `cast-the-four-domains`): long door places the body, shock
takes yellow health, blast knocks 60 along the aim, and calm lowers
concerns. Each spends its focus and does nothing without it.
Done (change `price-the-power-table`): the table holds the 33 recorded
rows. Barrier at 0.07 for a 500 ms tick costs 0.035, and 0.0175 with the
sustain technique. Repeating blast with the repeat technique costs 0.05.
Blast spends 1.2 times its cost, barrier twice, calm 0.3 times, the long
door once, and vanish 80 times.
Done (change `read-a-power-tree`): a tree whose first word is wrong is refused. The node count is the word at byte 52. A node at 10, 20 stays there. A node at 15, 25 becomes 5, 5. One 16-byte link is kept. A link count below 1 stores none. A file that ends after the magic is an error. Drawing the tree and the text library are not in.
Done (change `read-the-tree-text`): a text chunk stores Blast and an empty string. A tree with no chunk has no strings. A chunk that ends after its first word is an error. The second node is still at 5, 5. Drawing the tree is not in.
Done (change `draw-a-tree-line`): a node at 2, −1 with zoom 1 and no pan snaps to 20, −10. A node at 0.25 snaps to 2, 0 and stays at 2.5 while dragged. A factor of one half pulls 20, −10 to 10, −5. A link with both marks at zero draws nothing. A large node at the origin and a small node 400 to the right, both turns zero and zoom 1, starts at the large reach above the origin and ends at the small reach above 400, in 19 points, and is dim when the source mark is zero. Icons, frames, glows and the thick-line quads are not in.
Done (change `draw-a-node-picture`): an icon with no columns draws nothing. Power 6 on a sheet of 4 by 2, size 10 at 100, 50, sits at 95, 45 in cell 0.25, 0. A second byte of 1 uses bias 1. A view of 10.5 is 10 for a wide node and 5.25 for a narrow one. Frame slot 5 at size 10 is an 8-unit square at −4, −4 in cell 0, 0. A glow of width 10 and outer 4 has inner 8.4, 33 spokes, and the first sits straight up. Learning a node and the thick-line quads are not in.
Done (change `learn-and-thicken`): a lone node is marked 0x10000. A known id with bits 20 to 23 set still matches, and the later pair wins. A link to a closed node stays closed. A link to an opened node gains 0x10000. A top nibble of 7 does not. A mark of 0x5000 becomes 0x15000. A width of 2 through 0, 10 and 20 sits at 10, −2 and 10, 2. A zero-length chord stays on the point. The other effect meshes and charge-up are not in.
Done (change `charge-a-blast`): a 12 ms tick rates the charge at 0.01, and 1000 ms at five sixths. An amount of 0.5 plus 0.5 stays at 1 and keeps charging. An amount of 0.6 plus 0.5 clears to 0, sets the state to 1 and releases. Fizzling 0.5 by 0.1 leaves 0.3. Fizzling 0.3 by 0.2 becomes 0 with state −2. After release, 0.2 rises by 0.3. A still chunk grows by 0.0018 in one tick. Chunks at 0 and 1 with radius 1 move to −0.5 and 1.5. A pair 2 apart stays, and a pair on the same point stays. The other effect meshes are not in.
Done (change `fade-a-barrier-ripple`): no ripples leave the dome extent at 30, and four ripples make it 35. A ripple at shown 1 and strength 1 becomes about 0.96 and 0.95, and a hit 10 ahead eases the place to 0.5. Shown and strength of 0.06, and of 0.07, are gone. Wave, torrent and shell meshes are not in.
Done (change `step-a-wave`): a 12 ms wave rate is about 0.02, and 1000 ms is five thirds. Charging 0.99 by that 12 ms rate releases. The blast rate for the same tick leaves the amount at 1. A charging radius grows by 0.0015. A released front of speed 0 has reach 0.6, radius 0 and trail 1. Speed 10 grows the radius by 0.006 and sets the trail to 1.7. Torrent and shell meshes are not in.
Done (change `fade-a-shell-ripple`): shown 1 and strength 1 at scale 1 become 0.96. The same pair at scale 2 becomes 0.92. Shown 0.09 and strength 0.09 become 0.05 and stay. Shown 0.05 and strength 0.05 are gone. The torrent mesh and the shell mesh are not in.
Done (change `step-a-torrent`): a charge of 1 below phase 1 sets the level to 0.4, and a charge of one half sets it to 0.3. Phase 1 with channels at 0 eases them to 0.03 and 0.036 while a level of 0.4 falls to 0.39. Phase 2 multiplies channels at 1 by 0.96. A level of 0.03 falls under 0.02 and stops. The stream mesh and the shell mesh are not in.
Done (change `place-a-torrent-stream`): spans of 1, 2 and 3 count as 1, 2 and 2 streams. A stream along height 1 is placed at height -6. Two streams ask for 10 records, 130 groups, 1560 bytes and 1040 indices. A radius of 2 at scale one half and margin 1 reaches 12. Age 1 of 2 samples raises channels at 0 to 0.0025 and 0.005. A second channel of 1.8 still rises, and the next float does not. A point at the origin inside a reach of 10 moves to 0.01. A point on the reach stays, and a point past it is missed. The index winding and the shell mesh are not in.
Done (change `place-a-shell-vertex`): a position at 1, 2, 3 offset by a sum of 0, 3, 0 is raised on its height. A zero sum stays at 1, 2, 3. A hit of 0.1 stores about 3.333. A hit of 0.3 stores 6. An eighth hit is refused. The copied index list is not in.
Done (change `copy-a-shell-index`): two visible ranges and a hidden range copy to 14, 15, 16, 11, 12, 10, 17, 18, 19. A reversed triangle stays reversed. A range that ends early is empty. A short list is an error. A missing remap is an error. The skinned section filter is not in.
Done (change `keep-a-skinned-section`): a section with bit 2 and a hidden section that also has bit 2 copy to 14, 15, 16, 11, 12, 10. A section without bit 2 is absent. The same hidden section copies nothing on the object path. A reversed triangle stays reversed. A range that ends early is empty. A short list is an error. A missing remap is an error. The compacted vertex map is not in.
Done (change `compact-a-shell-vertex`): marks for a skinned range, a plain range, and a later skinned range are 0, 255, 2. Three vertices number as 0, absent, and 1. Indices 2, 0, 2 become 1, 0, 1. An absent index is an error. A short mark list is an error. The GPU buffers are not in.
Done (change `record-a-shell-slot`): three vertices store section 0 local 1, nothing, and section 2 local 2. A later link is ignored. A base of -1 subtracted from 1 stores 2. A short corner list is an error. A missing base is an error. The GPU buffers are not in.
Done (change `count-a-shell-index`): two skinned triangles, a hidden skinned triangle and a range that ends one early count as 3 triangles and 32 bytes for two kept vertices. A plain range adds nothing. A range that ends two early counts as -1 triangles and 16 bytes. The kind is 3 and the base is 0. The GPU upload is not in.
Done (change `fade-a-shell-by-its-size`): the stored size is 800. Opening for 0 stays at 0, opening for 1200 stays at 1, and opening for 1201 finishes at 1. A held open enters the hold state. A hold at 0 stamps 200, and a half rounds to a stamp of 10. A close of 400 is one half, and a close of 800 finishes at 0. A held close shows 1 with a gain of 1.5. A resting fade stays. The GPU upload is not in.
Done (change `light-a-shell`): an opening light at 0 is 0, and at one half it is a tenth of 0.75. The float just under 0.666 stays on the bend, and the next float is a twentieth. A resting light at 1 is a twentieth and keeps its gain. A closing light at one half stores a gain of 1.4. The GPU upload is not in.
Done (change `pair-a-shell-light`): a resting pair is 0.1 and 0.75. An open at 0 is 0 and 1. The float just under 0.25 is just under 0.1 and 0.75. An open at 0.25 is 0.1 and 0.75. A hold keeps its pair. A close at 0 is 0.1 and 0, and a close at 1 is 0 and 0.75. Three triangles ask for 28 header bytes and 12 payload bytes. A name of 0 stays, and a name of 1 unmaps. The play renderer does not issue the buffer calls.
Done (change `submit-a-shell-index`): two skinned spans submitted with the flag set and a name of 1 put 3 indices on the mesh, a header of 28 and a payload of 12, and unmap. The same spans with the flag clear keep a room of 32 bytes and do not unmap. The Bevy render graph does not issue the original's GL calls.
Done (change `save-the-session`): an `EXRS` file restores that session.
The original's save packages are not read.

Done (change `confirm-a-shop-trade`): the shop refuses two empty baskets, and refuses a buy the company's copper cannot cover. A larger sell is received.
Done (change `scale-match-experience`): a shared finish multiplies the template by 1 + tier/4 and splits it by contribution. Tier 0 with shares 1 and 3 credits 250 and 750; tier 4 with a trainer and equal shares adds 1500 to each. The special board lists Challenger through The Beast. How a tier scales a fee was not read.

Done (change `close-the-arena-day`): a merchant's wage is 2, a physician's 12, a trainer's the skill total, and a combatant's half of that, at most 5, plus 1. A new career is on day 0. Ending the day pays the bill. A day mod 7 of 1 gives the week to three or more of the six events with no tie. Day 7 drops an entry under health 3276. Four won weeks grant four rewards; from day 0 that lands on day 22.
Done (change `rest-overnight`): a night adds 6553 to health, or 9830 when a physician is hired, and stops at 65535. A pool of 58981 becomes 65534. A pool of 58982 becomes 65535. Health 3275 across seven nights is 49146 and stays entered. Health 1000 with a physician becomes 10830. The trainer's lessons are not in.
Done (change `teach-a-lesson`): with no trainer a technique of 7 stays at 0. A trainer who knows 7 adds 1000. A trainer who knows 3 adds nothing. A merchant and a physician stay at 0. A word of 4500 snaps to 5000. Five other skills at level 1 make the grant 166. A first skill at 25000 is left there and the next skill becomes 166. Item prices are not in.
Done (change `price-a-hire`): 11 copper hires nobody and stays at 11. 12 copper hires a combatant and leaves 0, with 12 spent. A second physician is refused and the spent total stays 24. Item prices are not in.
Done (change `price-a-shop-line`): a line of 7 buys for 7 and sells for 1. A lower mark of 2 buys for 24, a higher mark of 2 for 288, and both marks of 2 for 2880. Two lines of 3 sell for 1. A line with every bit set buys for 23591520. Buying 40 then buying 10 against a sale of 100 leaves 318 copper and 15 earned. Emblems and the reward item identities are not in.
Done (change `scale-a-match-stake`): a stake dword is copper, silver times 12, and gold times 144. Challenger asks 12 for a prize of 36. The Beast asks 24 for a prize of 432. A duel prize is 12. Tier 1 doubles both. Hiring one combatant and winning Challenger at tier 1 leaves 379 copper, with 72 earned. Emblems and the reward item identities are not in.
Done (change `grant-season-rewards`): a swept season grants four item ids. Season 0 is 1175, 1174, 1177, 1176. Season 1 is 1167, 1165, 1168, 1169. Season 2 is 1171, 1170, 1173, 1172. Season 3 is 1178, 1179, 1180, 1181. Day 134 grants season 0 again. A lost season grants none. Emblems are not in.
Done (change `keep-the-season`): a save on day 8 with the first two weeks won loads those weeks and no reward ids. Sweeping the loaded career through day 22, then saving again, restores day 22, one season, four rewards, and the ids 1175, 1174, 1177, 1176. The original's packages are not read.
Done (change `select-the-klean-emblem`): the name klean selects background 52 and design 81, with layer bytes 7, 0, 0, 6. The pictures are emblembkg052 and emblemdes081. Any other name selects nothing. A candidate's listed price was not read.
Done (change `price-a-candidate`): no skills and no gear cost 12. A skill total of 2 costs 48. Skill totals of 10 and 12 cost 432. A gear line of 7 costs 18. A two-silver line costs 31. 47 copper refuses the skill total of 2. A full roster with no copper is refused before the cap, and with 100 copper the cap refuses without spending. A second physician leaves the spent total at 60. The other emblem controls are not in.
Done (change `draw-an-unwatched-match`): teams of strength 4, then 2 and 2, then 10, drawn from rolls 1, 4 and 12, name teams 1, 2 and 3. The rolls become 0x08088406, 0x20221015 and 0x6066303d. Mode 16 for team 1 names team 2. A strength of 100 set aside still names team 1. An empty list names nobody and still steps the roll to 0x08088406. The bracket that clears losing records is not in. `XA_Arena` is 24%. The game-modes section stays 0%.
Done (change `clear-a-losing-bracket`): width 1 with force 5 leaves ids 5, 0, 3 and 0 and steps the roll to 0x08088406, and reports another group. One group from roll 1 leaves id 1. Roll 100 leaves id 9 at roll 0x235391f5. A zero id of strength 100 stays 0, id 1 is cleared, and both ids of 3 stay, at roll 0x20221015. A forced group does not step roll 4. An empty list keeps roll 7 and reports no other group. `XA_Arena` is 26%. The game-modes section stays 0%.
Done (change `draw-a-rival-field`): roll 0 draws 1, 2, 17, 6, 8, 14, 9, 4, 11, 12, 3, 13, 5, 19, 7, 15 and ends at 0x4b157bf0. Roll 1 draws 1, 17, 5, 7, 14, 8, 4, 10, 11, 2, 13, 3, 18, 6, 12, 19 and ends at 0xead32bb1. Three companies are left out. Spawning them is not in. `XA_Arena` is 28%. The game-modes section stays 0%.
Done (change `choose-a-daily-match`): rolls 0 and 1 both choose templates 0, 1, 4, 1, 2, 3 and end at 0x1210d4e5. Roll 4 chooses 0, 1, 6, 4, 4, 1 and ends at 0xb761a41e. A limit of 2 stops a third pick, a slot before the earliest is refused, and mask 3 refuses a top-three bit it shares. The clock that replaces the roll, the template copy, and the fee scale are not in. `XA_Arena` is 30%. The game-modes section stays 0%.
Done (change `scale-a-daily-match`): counter 0x04000000 replaces roll 99 and chooses 0, 1, 6, 4, 4, 1, ending at 0xb761a41e. Counter 0 chooses 0, 1, 4, 1, 2, 3 and ends at 0x1210d4e5. A high bit on the same low word does not change the row. A copied fee of 3 and prize of 12 at slot 3 with bit 2 become 12 and 48, and the tier is 3. Kind 8 shows 0 and 5. A tier of -4 becomes -1. Unit 0 at slot 4 leaves tier 4, and unit 1 leaves tier 0. Bit 4 at slot 0 from roll 0x10000000 stores extra 3 and ends at 0x50000001. The typed field walker and the unit generator are not in. `XA_Arena` is 32%. The game-modes section stays 0%.
Done (change `draw-a-unit-and-copy-fields`): seed 5489 draws 0xd091bb5c then 0x22ae9ef6. The first unit's bits are 0x3fea12376b800000. The index is 2. A mark of 5489 against stamp 0 draws 0xd091bb5c and reverses the mark. That unit from seed 1 at slot 4 sets the tier to 4. Bytes 0x11 through 0x88 with fields at 1 and 5 copy in full. A three-byte buffer and a backwards field are errors. Managed strings are not in. `XA_Arena` stays 32%. The runtime section stays 4%. The game-modes section stays 0%.
Done (change `leave-before-a-match`): a hired physician at 699 is removed and the death count is 1. A hired merchant at 700 is removed and the count stays 0. A hired trainer at 65000 stays. A hired combatant at 0 stays while the physician behind him is removed. A hired physician at 100 is removed and the merchant at 100 stays. An empty roster removes nobody. Releasing a kit and unlinking a world character are not in. `XA_Arena` is 34%. The runtime section stays 4%. The game-modes section stays 0%.
Done (change `begin-an-arena-match`): in-game command 4 in arena removes a hired physician at 699, asks for a save that does not abort on low health, marks the hub busy, clears the click hook, and sets the phase to transitioning. An empty roster still does that. Story, practice, command 1, and another page change nothing. Closing windows, writing a save package, and the transition are not in. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.
Done (change `dismiss-marked-windows`): the same command dismisses a child flagged 0xce100000, then a fully flagged child, and leaves an unmarked child, a later marked child past count 2, and a child flagged 0xce000000. The physician at 699 is still removed. Story dismisses nobody. Count 0 and count -1 dismiss nobody. The method on the child and the call after the two lists are not in. `TGUIForm` stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.
Done (change `close-a-dismissed-child`): a pinned window closes child A, skips an already closed child, then runs its own callback. The notice is raised. Argument 0 is stored. An already closed window is still dismissed, runs no callback, and leaves its child open. The physician at 699 is still removed. The capture release, the task's other children, detaching, redraw, and the call after the two lists are not in. `TGUIForm` is 1%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.
Done (change `finish-a-form-close`): a captured shown window that is the hover drops the capture, clears bit 0x20, clears the hover, closes the sibling that names it, hides, and the redraw word is 3 after four asks. An already closed captured window drops the capture and does not ask. The active form clears that pointer and does not ask. A miss leaves the bit. Cursor restore and the call after the two lists are not in. `TGUIForm` is 2%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.
Done (change `restore-a-cursor-and-follow-lists`): releasing the top capture publishes the lower code 0x15, shape 0, and the default. Code 0x8007 publishes shape 7 and does not mark the default. The only capture publishes code 0 and no holder. Mask 1 with clock 99 and mode word 6 copies the ten hand words, leaves mode word 3, and stamps 99. Mask 0 with no focus leaves the hands unset. A focus with mask 0 latches and clears the pending byte. The shape table and the virtual call are not in. `TGUIForm` is 3%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.
Done (change `apply-a-cursor-shape`): shape 7 with image 42 and a hook shows once, and the same image does not show again while the hook still runs. Shape 0 shows default image 9, marks the default, and stamps clock 12. An empty table leaves the current image. A focus with a session mark of 1 is invoked once before the ten hand words, and the mark becomes 0. Mask 1 without a focus does not invoke it. The physician at 699 is still removed. The focus method body, the rest of the session-mark path, and drawing the image are not in. `TGUIForm` is 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.
Done (change `copy-a-control-bank`): a focus whose mark is 1, on side 0 with a set flag, copies sixteen words from the first bank, clears the flag, and sets the code to 4. Side 1 with a clear flag copies the second bank and raises a notice. A flag that already matches leaves the words and still counts the install. A mark that is already 0 leaves the code. Mask 1 without a focus copies nothing. The physician at 699 is still removed. The install body, a world update of the side, and the non-zero argument path are not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.
Done (change `run-the-install-body`): a ready install of index 1 with value 42 and a hook shows once and runs the hook. The same index does not show or run it again. Index 2 with value 0 runs the hook and does not show. A block word leaves the shown value and still runs the hook. An index past the table stores the index and does not show. Index 0 while the current index is 5 shows fallback 9, arms, and stamps clock 12. An index that is already 0 leaves the armed byte clear. A ready byte of 0 leaves the index. The physician at 699 is still removed. Drawing through the platform call is not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `draw-a-platform-cursor`): a shown value of 42 is passed with owner 7 and extra 8. The indirect target is not called. An open world with byte 0 copies the first bank and installs twice. Byte 5 copies the second bank, raises the notice, clears the span, and masks the flags with 0xfffffdbd. A force leaves byte 5 and selects the first bank once. A lock leaves byte 0 and selects the second bank. Raising the mark to 1 stores the flag 1 and the code 0, installs index 0, shows fallback 9, leaves the bank, clears the hold byte, sets view bit 0x10, and clears the view code. A second raise leaves the code. The physician at 699 is still removed. Callback registration is not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `register-a-hook`): a cleared mark with hold 0 registers code 0x8001 for hook 11. Hold 1 registers nothing. World byte 5 registers the wide hook at 0x2001 and at 0x1002, then the gate hook. Raising the mark drops the plain, lab, and gate hooks and inserts the watch hook at key 4. Value 42 activates the action set once with owner 7 and extra 8. The same value does not activate it again. The Steam library is not loaded. The physician at 699 is still removed. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `run-a-hook-body`): the gate hook clears two result words and sets the flags to 1. A set mid byte counts an alternate hook. A clear mid byte leaves the flags at 1. The pick under the cursor is not in. A watch without bit 0x2000 stores a stamp of 0. A ready watch with code 1, clock 12, and bit 0x800000 stores stamp 12, clears that bit, and counts a release and a refresh. Code 2 sets the bit and counts a press. A leave with a focus and mark 2 clears the focus, sets the mark to 0, and registers code 0x8001. The hook returns 1. The focus method before the hand words is not this watch hook. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `pick-a-cursor`): a clear gate on an open world with a taken item ORs the flags with 5 and publishes owner 6. A hold byte and a busy byte do not pick. Bank side 1 with no world aim does not pick, and a world aim counts one. An actor parent equal to the world counts an open. Kind 0xa00 counts a chest. A named row counts an open. Bit 0x800 does not. A set item kind counts one kind call. Aim bits above 0x428c0000 copy the place and a miss with the hold byte clears the aimed word. A taken item sets click flags 0x11. Equal aim bits do not copy. A hover stores flags 0x15, publishes owner 3, and clears the byte and the mark. A focus begin at clock 5 counts one call before the hands. The raycast, the rest of the click path, and the focus method body are not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `walk-a-pick-row`): a record of another kind, then kind 0x10010a linked to the world, then that kind again, opens once. Kind 0x10010d linked elsewhere does not open, and the same record linked to the world opens once. A set named check opens without a matching record. The raycast and the named-check body are not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `find-a-kind-mark`): item 7 counts one kind call when its plain span holds 0x20000, and the same id without the mark counts none. Id 0 counts none. Id 0x80000005 counts when the high key 5 is marked and the plain key 5 is not. Id 0xC0000005 still searches the high table. Id 0x40000009 misses a marked plain row of the whole id and counts once the wide key 9 is marked. Key 7 is found between keys 2 and 9. A later record in the span counts. A size of 0, a short span, and a step of 0 do not. A taken item does not look up the mark. The two calls after a hit are not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `release-a-picked-kind`): cursor 8 on a view of 8 clears the hit, counts no kind call, and leaves a held count of 2. Cursor -1 does not count. A marked item with count 3 drops once, leaves the count 0, and keeps the pointer. Count 0 with byte 1 closes the pointer. Count -1 with byte 2 clears the pointer and does not close. Cursor 7 on a view of 8 shows pane 11, the first tagged 0xce1c2013, and does not build. With no pane, a marked span counts one build. The scene walk and the pane builder are not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `walk-the-scene`): a sector that shares no camera bit is skipped. A body with bit 0x4000000 is skipped, as is a body without flags 0x89. A probe of 1 with three hits is replaced by a clear probe with two hits, so the hit is body 15 and the index is 1. The pads are the bits of 5 then of 0.1. An empty scene leaves the supplied hit. A marked build with unit 2, margin 5, pad 6, span words 3 and 4, and row 9 stores rows 18, width 28, height 58, place 86, and order 0x10a. Bit 0x1000 with slot 2 and a cap of 10 stores 8 rows. Cursor 30 on a screen of 20 stores place -8. The ray distance and child layout are not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `shorten-the-ray`): the ray starts at 4000 along z. A centre at 100 with two hits is kept, the reach becomes 100, and a centre at 500 is left. A centre at 5000 misses. A pane of 28 by 58 places a centre child of 10 by 4 at 9, 27, moves an anchor child from 4 to 32, and grows a stretch child from 10 to 38. Nested children are not visited. A free camera ray is not in. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.
Done (change `open-a-draw-skin`): unit 5 stores spans 10, 5, 10 and 10, pads 8, 4, 8 and 8, and the colour 255, 250, 230, 100 three times. The mark is 1. Argument 0 counts the base fill and not the call after it. Argument 1 counts an allocation and that later call. A failed allocation returns nothing. Argument 2 counts the later call and not the allocation. The slider, the frame, and the widget dispatcher are not in. `TGUIDrawSkin` is 1%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-flat-slider`): unit 5 at 10, 20 with width 100 stores caps, a track, a pad of 10, and a thumb of 5 at value 0. Value one half stores a thumb of 40. A spare of 1 at one half stays at thumb 5, and a spare of 3 stores 7. State bit 0x10 selects ink 0 ahead of bit 0x200000, and bit 0x10000 counts a mark. A scroll mode selects ink 2 and draws no flat quads. The scroll bar, the frame, and the widget dispatcher are not in. `TGUIDrawSkin` is 3%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-scroll-bar`): unit 5 at 10, 20, size 100 by 80 and lip 4 stores a trough of 16, 26, 88, 68 and twelve quads. Value 0 stores thumbs 10 and 25. Value one half stores thumb 52. A spare of 1 at one half stays at 10, and a spare of 3 stores 12. State bit 0x200000 selects the bar ink, and bit 0x10000 counts a mark. Mode 0x20000000 draws neither. The frame and the widget dispatcher are not in. `TGUIDrawSkin` is 10%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-frame`): unit 5 at 10, 20 and size 100 by 20 stores four corners of span 15 and two edge bars, and the ink is 2. Height 80 adds side bars from 10, 85 to 15, 35 and from 110, 85 to 105, 35. State bit 0x10 selects ink 0. A wide bit with a clear mark selects ink 0. State bit 0x20 selects ink 3. The widget dispatcher is not in. `TGUIDrawSkin` is 14%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `centre-an-icon`): an icon of size 16 in a widget at 10, 20 of size 100 by 80 sits at 52, 52 with ink 2. A spare of -6 sits at 7, 16. State bit 0x10000 passes scale bits 0x3f800000. Bit 0x40 inside that mark stores height 53, and bits 0x60 together leave 52. A wide bit selects the wide draw. An own list is kept. Bit 0x10 selects ink 0. A clear high bit with bit 0x40 selects ink 1. A set high bit selects ink 4, or ink 3 when bit 0x40 is also set. The glyph call is not in. `TGUIDrawSkin` is 16%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `dispatch-a-form`): unit 5 stores spans 10, 15 and 20. Bit 0x40 counts shade 9. Bit 0x20000 with bit 4 clear draws a quad from 0, 0 to 100, 80. A spare of 30 at value 1 shifts by 30 and the origin is -20. A spare of 18 at one half shifts by 9. Bit 0x800 leaves the shift at 0. Bit 0x100 skips the shift call. Children marked 0, 1, 0, 1 visit 1 and 3. A spare of 1 at one half stays at 0, and a spare of 3 stores 2. The rest of the dispatcher is not in. `TGUIDrawSkin` is 19%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-after-the-children`): a title of width 40 in a form of width 100 sits at 30, and a tint byte of 200 stays 200. A width of 10 with a measure of 16 sits at -3. Style bits 0x10 draw four corners of span 30. Bit 2 adds a square from 100, 0 to 80, 20 unless the mark is -4, and the top corner then uses 0, 0.4, 0, 0.7. Bit 0x400000 counts a wide call and draws no corners. The later edge bars are not in. `TGUIDrawSkin` is 22%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-edge-bars`): unit 5 on a form of 100 by 80 with style 0x10 and no title draws a top bar from 33, 0 to 67, 10, side bars, and a bottom bar. A title of width 40 splits the top at gap 17 and adds caps. A style that is not 0x10, and bit 0x400000, draw no edges. The drawing after those bars is not in. `TGUIDrawSkin` is 25%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-wide-top`): a wide form whose style word has bits 0xc0 equal to 0x80 draws three top quads. At unit 5 and width 100 the caps run from 0, -5 to 10, 5 and from 90, 5 to 100, -5, and the bar runs from 10, -5 to 90, 5. Style 0x10 without those bits, and a form that is not wide, draw none. The side bars after that top are not in. `TGUIDrawSkin` is 26%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-wide-side`): the same wide form then draws a left side. At unit 5 and height 80 the bar runs from -5, 10 to 5, 70, with a cap from y 0 to 10 and a cap from y 70 to 80. Style 0x10 without those bits, and a form that is not wide, draw none. The bottom after that side is not in. `TGUIDrawSkin` is 28%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-wide-bottom`): the same wide form then draws a bottom. At unit 5, width 100 and height 80 the bar runs from 10, 75 to 90, 85, with caps from 0, 75 to 10, 85 and from 90, 75 to 100, 85. Style 0x10 without those bits, and a form that is not wide, draw none. The right side is not in. `TGUIDrawSkin` is 29%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-wide-right`): the same wide form then draws a right side. At unit 5, width 100 and height 80 the bar runs from 95, 10 to 105, 70, with a cap from y 0 to 10 and a cap from y 70 to 80. Style 0x10 without those bits, and a form that is not wide, draw none. The mark after that side is not in. `TGUIDrawSkin` is 31%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `copy-the-ink-mark`): on that same wide form, bit 0x40 of the first flag word counts one ink copy. A clear bit counts none, and so does a form that is not on that wide path. The join after that copy is not in. `TGUIDrawSkin` stays 31%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-late-close`): the join then draws one square when the close mark is -4. At unit 5 and width 100 it runs from 100, 0 to 80, 20. A mark of 0 or -3 draws none. The hold-bit edge test is not in. `TGUIDrawSkin` is 32%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `mark-the-held-edges`): with the hold bit, style bits 0xc0 equal to 0x80, and no wide bit, a y below the unit sets mask bit 1 and an x below the unit sets bit 2. A right sum past the room sets bit 4, and a bottom sum past the room sets bit 8. An equal sum sets neither. A clear hold bit, a wide bit, and style 0x10 without those bits set none. The quads after that mask are not in. `TGUIDrawSkin` stays 32%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-bar`): the same held form then draws one bar from 30, 0 to 70, 10 at unit 5 and width 100. A y below the unit with owner bit 1 and without bit 0x800000 draws none. Owner bit 0x800000 draws the bar again. A clear hold bit and a wide bit draw none. The drawing after that bar is not in. `TGUIDrawSkin` is 33%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-tab`): bit 0x200 on that held form draws one tab. A grip of 10, 20 at unit 5 sits from 16, 30 to 26, 40. Mask bits 1 and 4 together raise it to 35. A clear bit 0x200 and a clear hold bit draw none. The next anchor after that tab is not in. `TGUIDrawSkin` is 34%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-lower-tab`): the same bit then draws a lower tab from the seat. A seat of 80 at unit 5 sits from 16, 60 to 26, 70. Mask bits 4 and 8 together raise it so the top is 65. A clear bit 0x200 and a clear hold bit draw none. The anchor after that lower tab is not in. `TGUIDrawSkin` is 35%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-post-quad`): the same bit then draws one quad from the grip and a post. A grip of 10 and a post of 40 at unit 5 runs from 11, 35 to 31, 55. A clear bit 0x200 and a clear hold bit draw none. The compare after that quad is not in. `TGUIDrawSkin` stays 35%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-left`): a held form that is not wide then draws a left strip. At unit 5 and height 80 it runs from 0, 30 to 10, 50. A clear bit 0x200 still draws it. A clear hold bit and a wide bit draw none. The mask compare after that strip is not in. `TGUIDrawSkin` is 36%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-cap`): mask bits 1 and 2 together then draw a cap under that strip. At unit 5 it runs from 0, 10 to 10, 30. An edge of 5, 5 draws none. A clear hold bit draws none. The flag test after that cap is not in. `TGUIDrawSkin` stays 36%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-ledge`): mask bits 1 and 2 together then draw a ledge. At unit 5 it runs from 10, 0 to 30, 10. Bit 1 of the second flag word, or owner bit 1, draws none unless the owner also has bit 0x800000. An edge of 5, 5 and a clear hold bit draw none. The sign flip after that ledge is not in. `TGUIDrawSkin` is 37%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-open-frame`): with the band bits and no wide bit, a clear hold bit forces mask 15 and draws the wider strip. At unit 5 and height 100 the strip runs from 10, 40 to 20, 60, with a cap from 10, 20 to 20, 40 and a ledge from 20, 10 to 40, 20. A clear bit 1 counts one call, and a set bit counts none. The held mask stays 0. The helper body is not in. `TGUIDrawSkin` stays 37%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-sign-flip`): when the edge bits are not both set, a held form draws one quad. At unit 5 it runs from -10, -10 to 30, 30. An edge of 4, 4 draws none, and the cap remains. A clear hold bit draws none. The next mask test is not in. `TGUIDrawSkin` stays 37%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-end`): mask bits 1 and 4 together, with bit 0x200, draw one cap. At unit 5 and width 100 it runs from 70, 10 to 100, 30. A clear bit 0x200 draws none. An edge that misses either bit, and a clear hold bit, draw none. The quad after a clear tab bit is not in. `TGUIDrawSkin` is 38%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-plain-cap`): mask bits 1 and 4 together, with bit 0x200 clear, draw one narrower cap. At unit 5 and width 100 it runs from 90, 10 to 100, 30. A set bit 0x200 draws the wider cap instead. An edge that misses either bit, and a clear hold bit, draw none. The flag test after that cap is not in. `TGUIDrawSkin` stays 38%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-shelf`): the same clear tab bit then draws a shelf. At unit 5 and width 100 it runs from 70, 0 to 90, 10. Bit 1, or owner bit 1, draws none unless the owner also has bit 0x800000. A set bit 0x200, an edge that misses either mask bit, and a clear hold bit draw none. The quad after that shelf is not in. `TGUIDrawSkin` is 39%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-over`): when mask bits 1 and 4 are not both set, bit 0x200 draws one quad. At unit 5 and width 100 it runs from 70, -10 to 110, 30. A clear bit 0x200 draws none. An edge of 4, 4 with a right room of 100 draws the wider cap instead. A clear hold bit draws none. The quad after a clear tab bit on that path is not in. `TGUIDrawSkin` stays 39%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-bare`): when mask bits 1 and 4 are not both set, a clear bit 0x200 draws one quad. At unit 5 and width 100 it runs from 70, -10 to 110, 30. A set bit 0x200 draws the other sample instead. An edge of 4, 4 with a right room of 100 draws none of either. A clear hold bit draws none. The drawing after that join is not in. `TGUIDrawSkin` is 40%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-right`): bit 0x200 then draws one strip. At unit 5, width 100 and height 80 it runs from 80, 30 to 90, 50. A clear bit 0x200 draws none. A clear hold bit with that bit set draws the wider strip, from 70, 40 to 80, 60 at height 100. The drawing after that strip is not in. `TGUIDrawSkin` stays 40%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-beam`): with the tab bit set and held-edge mask bit 4 clear, a clear bit 0x200 of the plain word draws one bar. At unit 5, width 100 and height 80 it runs from 96, 20 to 106, 60. That plain bit draws none, and the right strip remains. Mask bit 4 draws none. A clear tab bit and a clear hold bit draw none. The drawing after that bar is not in. `TGUIDrawSkin` is 41%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-caps`): the same clear plain bit then draws two caps. At unit 5, width 100 and height 80 the upper cap runs from 96, 0 to 106, 20 and the lower cap runs from 96, 60 to 106, 80. That plain bit, mask bit 4, a clear tab bit, and a clear hold bit draw none. The drawing on the clear tab path is not in. `TGUIDrawSkin` is 42%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-trim`): a clear tab bit then draws one strip. At unit 5, width 100 and height 80 it runs from 90, 30 to 100, 50. A set tab bit draws none. A clear hold bit with the tab bit clear draws the wider strip, from 80, 40 to 90, 60 at height 100. The next mask test is not in. `TGUIDrawSkin` stays 42%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-low`): mask bits 2 and 8 together then draw one strip. At unit 5 and height 80 it runs from 0, 50 to 10, 70. A missing bit draws none. A set tab bit still draws it. A clear hold bit draws the wider strip, from 10, 60 to 20, 80 at height 100. The next quad is not in. `TGUIDrawSkin` stays 42%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-sole`): the same bits then draw one ledge. At unit 5 and height 80 it runs from 10, 70 to 30, 80. A missing bit draws none. A set tab bit still draws it. A clear hold bit draws the wider ledge, from 20, 80 to 40, 90 at height 100. The other arm is not in. `TGUIDrawSkin` is 43%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-skirt`): when mask bits 2 and 8 are not both set, a held form draws one quad. At unit 5 and height 80 it runs from -10, 50 to 30, 90. Both bits set draw the ledge instead. A clear hold bit draws none. The next quad is not in. `TGUIDrawSkin` stays 43%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-floor`): both arms then draw one bar. At unit 5, width 100 and height 80 it runs from 30, 70 to 70, 80. A set tab bit still draws it. A clear hold bit draws the wider bar, from 40, 80 to 60, 90 at height 100. A style that is not that band draws none. The next mask test is not in. `TGUIDrawSkin` is 44%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-rear`): mask bits 4 and 8 together, with the tab bit, draw one strip. At unit 5, width 100 and height 80 it runs from 80, 50 to 100, 70. A missing bit draws none. A clear tab bit draws none. A clear hold bit with the tab bit draws the wider strip, from 70, 60 to 90, 80 at height 100. The clear-tab arm is not in. `TGUIDrawSkin` stays 44%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-lip`): the same bits with a clear tab bit draw one narrower strip. At unit 5, width 100 and height 80 it runs from 90, 50 to 100, 70. A set tab bit draws the wider strip instead. A missing bit draws none. A clear hold bit with a clear tab bit draws the wider clear strip, from 80, 60 to 90, 80 at height 100. The next quad is not in. `TGUIDrawSkin` is 45%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-sill`): both tab arms then draw one quad. At unit 5, width 100 and height 80 it runs from 70, 70 to 90, 80. A set tab bit still draws it, and the wider strip remains. A missing bit draws none. A clear hold bit draws the wider quad, from 60, 80 to 80, 90 at height 100. The other arm is not in. `TGUIDrawSkin` is 46%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-drape`): when mask bits 4 and 8 are not both set, the tab bit draws one quad. At unit 5, width 100 and height 80 it runs from 70, 50 to 110, 90. Both bits set draw the bottom quad instead. A clear tab bit draws none. A clear hold bit draws none. The clear-tab arm is not in. `TGUIDrawSkin` stays 46%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-held-fold`): the same bits with a clear tab bit draw one quad. At unit 5, width 100 and height 80 it runs from 70, 50 to 110, 90. A set tab bit draws the other sample instead. Both bits set draw none. A clear hold bit draws none. The join after that quad is not in. `TGUIDrawSkin` stays 46%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-band-close`): on that band, a close mark of -4 draws one square. At unit 5 and width 100 it runs from 80, 0 to 100, 20. The earlier square remains. A mark of 0 or -3 draws none of this square. A style that is not that band draws none of it. A wide bit draws none of it. The call after that square is not in. `TGUIDrawSkin` is 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `count-the-band-call`): on that band the dispatcher counts one call with argument 0. A close mark of -4 still counts one, and the square remains. A style that is not that band counts none. A wide bit counts none. The bytes the jump skips are not in. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `restore-the-frame`): every dispatch passes the frame to one call, restores the saved register, then sets the stack to the frame and pops the frame. A held band, a wide form, and a style that is not that band each do so. The helper body is not in. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `release-the-frame-text`): the frame helper releases one string slot. That slot starts empty, and an empty slot is not freed. A held band, a wide form, and a style that is not that band each release it once. The bytes the jump skips do not run. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-span-bar`): unit 5 from 10, 20 with span 100 and stored word 4 draws a middle from 15, 20 to 105, 28, a left cap from 10, 20 to 15, 28, and a right cap from 105, 20 to 110, 28. One preparation is counted. The middle sample binds once and the cap sample twice. Stored word -3 uses thickness -6. The view scale and the colour word are not in. `TGSLines` is 1%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `place-a-span-bar`): shifts 2 and 100 with scales one half and 2 place the first corner at 8.5, 160 and store colour 17 twelve times. The raw corner stays 15, 20. An unset view places that corner at positive zero and negative zero and stores colour 0. The mark written when bit 4 is set is not in. `TGSLines` stays 1%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `mark-a-span-bar`): cursor 3 and mark 9 with flag 4 store indexes 3 through 14 and finish at 15. Flag 5 and a flag with the high bit and bit 4 set still store them. Flag 0 and flag 1 store none and still finish 12 ahead. Cursor 4294967294 stores 4294967294, 4294967295 and 0, then finishes at 10. An unset flag stores none. The other line-class methods are not in. `TGSLines` stays 1%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-span-square`): stored word 4 at 10, 20 draws (3, 29), (3, 13), (19, 13), (19, 29). Stored word 6 draws (-1, 33), (-1, 9), (23, 9), (23, 33). Stored word -1 places the first corner at bits 0xdf000000, 0xdf000000. The cursor starts at 0. Flag 4 and mark 9 store indexes 0 through 3 and finish at 4. Flag 0 stores none and still finishes at 4. Colour 17 is stored four times. The scale rebuild is not in. `TGSLines` is 2%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-line-chip`): at unit 5 and place 10, 20 the chip draws (10, 40), (10, 20), (30, 20), (30, 40). The first sample is 0.75, 0.8, 0.75, 1, then 0.85, 1, 0.85, 0.8. Flag 0x40 binds the second sample, 0.85, 0.8, 0.85, 1, then 0.95, 1, 0.95, 0.8, and draws the same corners again, so four colour writes become eight. Flag 0x10 selects the first ink, mode 0x10000 the second with alpha 255, and any other word the third. The text bind is not in. `TGSLines` is 4%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-bevel-box`): place 10, 20 of size 100 by 80 fills 12, 22 to 108, 98 as two quads with a hard seam at 60, borders 11.5, 21.5 to 108.5, 98.5 as six lines with the left seam at 60.5 and the right at 60, and outlines 10.5, 20.5 to 109.5, 99.5 as four lines. Fill stops are 0.6 and 0.4, border stops 0.65 and 0.35. Bit 0x40 selects the raised arm, else bit 0x10 the pressed arm, else idle. Three begins, twelve binds and three ends are counted. The insets of one and two pixels are literal. The shipped inks make raised and idle transparent and pressed black at alpha 0xE0. The colour ramps are not in. `TGSLines` stays 4%. `TGUIDrawSkin` is 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-capped-span`): unit 5 with left 10, right 110 and centre 28 draws a left cap from 0, 18 to 30, 38, a right cap from 120, 18 to 90, 38, and a middle from 30, 28 to 90, 38. The cap sample binds twice and the middle sample once. Twelve colour writes are counted. The retail method's second positional float is dead. The corners follow the view. The caller is not in. `TGSLines` is 6%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `place-a-line-label`): a label of width 40, line height 24 and base 18 at 10, 20 of size 100 by 80 at unit 5 puts x at -35 for style 0x1000, 39 for 0x2000, 65 for 0x3000, 115 for 0x4000, and 13 for any other word. y is 18 for 0x100, 73 for 0x200, 117 for 0x300, 100 for 0x400, and 23 otherwise. State bit 0x10 selects the first ink, mode 0x10000 the coloured renderer with a one-pixel nudge on bit 0x40, bits 0x60 the third ink with a nudge, and any other word the second ink. The glyph renderer and the string storage are not in. `TGSLines` is 7%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-line-rail`): unit 5 at place 10, 20 of size 100 by 80 draws a left cap from 0, 85 to 30, 105, a right cap from 100, 85 to 90, 105, a middle from 30, 95 to 90, 105, and a thumb from 105, 30 to 95, 40. The cap sample binds twice, the middle once and the thumb once. Bit 0x10 selects the first ink, otherwise the second. Mode 0x10000 rebinds the thumb in the third ink with alpha 255. The thumb value is not in. The frame's 1,511 bytes are also in this class, so `TGSLines` is 17%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `scale-the-line-heights`): a vertical scale of 1 stores 80, 104 and 128; one half stores 40, 52 and 64; 2 stores 160, 208 and 0 because the low byte wraps. One parent scale call is counted. The parent body is not in. `TGSLines` is 18%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `hit-the-line`): the span is four units, or six when wide. Flags 0x30 equal 0x10 and a y at or above the span run the trim test: right minus the span at or below x returns -4 when control bit 2 is set, else -3 on bit 1. Otherwise control bit 0x200 and x past the left limit return -8 above the bottom limit, -10 inside the bottom band, and -9 near the top limit. Any other point is 0. The meaning of the codes is not established. `TGSLines` stays 18%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-a-line-shade`): unit 5 at place 10, 20 of size 100 by 80 with side inset 15 draws six quads: cap, bar and cap from y 13 to 23 around edges 25 and 95, then the same three from y 97 to 107. The binds are cap, bar, cap, cap, bar, cap. An inner rectangle runs 15, 25 to 105, 95. The half unit is an integer shift. State band 0x10000 selects the special ink with alpha 255, and the inner ink forces alpha 255. Three modes, two begins, two ends, 28 vertices and 28 colour writes are counted. The owner's inset is not in. `TGSLines` is 24%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `store-the-line-extent`): opening a skin at unit 5 also stores a fifth extent of 20, four times the unit. The readers are not in. `TGSLines` stays 24%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Correction (2026-10-19): the slider (3,339 B), icon (756 B), held-edge mask (352 B), opening (58 B) and dispatcher (11,016 B) methods are line-class overrides, not parent methods. `TGSLines` is 87% and `TGUIDrawSkin` is 32%. The frame (1,511 B) was already moved in an earlier correction. The UI section stays 0% and the headline stays 14.4% and 5.7%.
Done (change `highlight-the-close-edges`): with the still bit set, close mark -9 draws the tab tint from x0 plus one to three units between y0 and y0 plus two units; -10 draws it below y0; -8 draws a four-unit square at x0, y0. x0 is the close x plus the rounded 0.15 of the unit, and y0 is the close y plus three units when mask bits 1 and 4 are set, else two. The tint is 255, 204, 153, 255 with the tab sample. The click path is not in. `TGSLines` stays 87%. `TGUIDrawSkin` stays 32%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `open-the-line`): opening the line calls the parent's open once and the texture load once. `TGSLines` stays 87%. `TGUIDrawSkin` stays 32%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `prepare-the-skin-renderer`): with the guard clear, the two setup calls run and the method returns 1; with the guard set, no calls run and it still returns 1. `TGUIDrawSkin` stays 32%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `scale-the-skin-alpha`): the first stored alpha, scaled by the horizontal scale, fills 14 ramp entries; the second, scaled by the vertical scale, fills 48. Both round to nearest even and keep the low byte. `TGUIDrawSkin` is 33%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `place-a-skin-label`): the parent label aligns like the line label. Bit 0x10 draws once in the first ink; mode 0x10000 uses the coloured renderer with a nudge on bit 0x40; bit 0x40 draws a shadow one pixel left in the second ink then one pixel down in the third; any other word draws once in the fourth. `TGUIDrawSkin` is 34%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-shade`): the parent shade prepares, sets mode 1, draws one rectangle inset by the stored inset and sets mode 0x10. At place 10, 20 of size 100 by 80 and inset 5 it runs from 15, 25 to 105, 95 with the place colour and alpha 255. Four vertices and four colour writes are counted. `TGUIDrawSkin` stays 34%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-cross`): the parent cross draws four line segments around a point. The half extent is the unit plus one, halved, plus one. At unit 5 and point 10, 20 the corners are (8, 15), (13, 15), (8, 25), (13, 25), (6, 18), (6, 23), (16, 18), (16, 23). One begin, eight vertices and one line end are counted, with no colour bind. `TGUIDrawSkin` is 35%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `place-a-control-rect`): the control-code switch returns the close band for codes -4, -6 and -5, shifted by the tab metric selected by flag bits 1 and 3, and the rail squares for -9, -10, -13 and -14. Any other code writes nothing. At unit 5 and trim 2, code -4 at x 100 returns 74, 4, 95, 26, and code -13 returns 20, 11, 39, 30. `TGUIDrawSkin` is 37%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `place-the-skin-frames`): style bits 0x30 select one textured frame and 0xc0 independently select a second, inset by one on every side. Each group swaps the 0x9c/0xb8 pair for 0xb4/0x9c; any other value in a group selects nothing. At place 10, 20 of size 100 by 80 the outer frame runs to 110, 100 and the inner one to 109, 99. `TGUIDrawSkin` is 38%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-image`): the control image centres its square in the place by half the spare width and height, with the marked band one lower. Mode 0x10000 draws with scale 1 and no ink; otherwise flag bit 0x10 binds slot 0xd8, bit 0x40 binds 0xf4 and any other word 0xe4, and style bit 0x1000000 adds the raised or plain colour to the last two. Style bits 0x70000000 select the frame index. An image of size 16 at 10, 20 of size 100 by 80 sits at 52, 52, or 52, 53 when marked. `TGUIDrawSkin` is 40%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `credit-the-parent-bevel`): the 775-byte bevel at 0x1002198c0 is `TGUIDrawSkin`'s slot 27, inherited by `TGSLines`, so it counts under the parent. `draw_skin_bevel` takes the method's two booleans, with the first winning, else the second, else idle, and shares the fills, six border lines and four outline lines already tested. `TGUIDrawSkin` is 42%. `TGSLines` stays 87% of its own overrides. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `hit-the-skin`): the parent hit test returns 0x80000011/0x80000012 and 0x80000014/0x80000018 for the edge flags before probing. Flag bit 0 probes -4, -5 and -10 above its threshold and falls back to -3; flag 0x200 probes -9 and -10 from x 0x44 and falls back to -8; flag 0x100 probes -14 and -13 from y 0x3c and falls back to -12. A probe hits only when the supplied rectangle contains the point inclusively. `TGUIDrawSkin` is 44%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-detail`): the detail dispatch prepares and draws one inset rectangle. Style 0x20000000 with flag 0x40 clear adds the horizontal helper at the original place with offset -unit/2 and mode 0x50; style 0x40000000 with flag 0x40 set, or flag 0x80, adds the vertical helper at the inset rectangle. At place 10, 20 of size 100 by 80 and inset 5 the rectangle runs from 15, 25 to 105, 95. `TGUIDrawSkin` stays 44%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-rect`): the rectangle dispatch draws one inset rectangle and passes flag bits 0x10 and 0x40 as two booleans; bit 0x40 adds one cornered overlay at the same rectangle. At place 10, 20 of size 100 by 80 and inset 5 it runs from 15, 25 to 105, 95. `TGUIDrawSkin` stays 44%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-strip`): the textured strip draws four two-vertex bands between the first and third floats, with the middle y half way from the fourth float to the second. Slots 0x12c, 0x134, 0x138 and 0x130 bind in that order; one begin, four binds and one end are counted. With 10, 20, 30 and 40 the eight vertices run from (30, 20) to (30, 40) around a middle y of 30. `TGUIDrawSkin` is 45%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-slider`): style 0 draws the track inset by two, translates by the rounded thumb offset, draws the thumb and translates back; style 0x10000000 fills an inset rectangle and draws a unit-wide bar at the same offset. At place 10, 20 of size 100 by 80 with inset 5, thumb 10 and fraction one half the track is 12, 22 of size 96 by 76 and the thumb 55, 25 of size 10 by 70; the filled arm runs from 16, 26 and its bar from 57, 20. Any other style draws nothing. `TGUIDrawSkin` is 47%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `set-the-skin-measures`): the 48-byte setter copies the three measured words to fields 8, 0xc and 0x10 and writes four units plus four to field 0x14. With 2, 2 and 24 at unit 4 the fields are 2, 2, 24, 20. `TGUIDrawSkin` stays 47%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `prepare-the-skin-programs`): the 333-byte setup builds a solid shader of kind 5 with `MPrj` set to 2, a textured shader of kind 0x15 with `MPrj` and a `Scale` lookup, and a rounded shader of kind 5 with `MPrj`, `Tex` set to 0 and a `TexCol` lookup. Six uniform operations are counted and the guard makes a second call do nothing. The three GLSL sources are kept with the programs. `TGUIDrawSkin` is 48%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-inset-bevel`): with flag 0x40 the inset bevel draws an inner fill at inset plus two, a border at inset plus one, the 0x6c bind and outline at the inset, then a corner detail at the border rectangle with mode 0xa0 and the unit. Without the flag one virtual inset rectangle draws with the flag's two booleans. At place 10, 20 of size 100 by 80 and inset 5 the inner is 17, 27 of size 86 by 66 and the outline 15, 25 of size 90 by 70. `TGUIDrawSkin` is 50%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-scrollbar`): the second slider draws its plain track at x + 1, y + 2 with width w - 2 and height h - 4 and its thumb at y + 1 + the inset, translating by the rounded offset; the filled arm fills the inset rectangle and draws a unit-wide bar. At place 10, 20 of size 100 by 80 with inset 5, thumb 10 and fraction one half the track is 11, 22 of size 98 by 76 and the thumb 55, 26 of size 10 by 68. `TGUIDrawSkin` is 52%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skin-region`): the 965-byte scroll region stops after one begin on mode bit 0 or a rejected clip. Otherwise it clips at the origin plus the inset, scrolls by the rounded totals less that span times the fractions (or zero on flag 0x800), replays every buffered quad, visits every visible child and releases the scissor. Flag 0x80 adds a 0x30 detail at the scroll offsets with one unit; flag 0x100000 adds a 0x50 detail with four. At origin 10, 20, inset 5, 4, extent 100, 80, totals 150, 120 and fractions one half and one quarter the clip is 15, 24 of size 95 by 76 and the scroll is 28, 11. `TGUIDrawSkin` is 55%, closing every sized own method; the remaining two-byte slot is an identity stub. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `scale-the-menu-clock`): the main menu converts a clock word by multiplying by 1/135, rounding to nearest even, then multiplying by 1/8. Clock 135 gives 0.125 and 270 gives 0.25; 67 gives 0 and 68 gives 0.125. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `close-the-menu-on-the-key`): the journal and character-loot key override closes with argument 0 and returns 1 when the mask carries both 0x2000 and 0x2, without running the base handler; any other mask returns the base result. Both 89-byte methods share the body. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `select-the-menu-class`): the class-select screen stores its index at field 0x6c and returns 3; it plays one selection sound only when the index and the sound flag are both above zero. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skill-tip`): the tooltip draws its icon at two units across and five units down and its label at the tooltip's x less four units, its y, six units wide and two units high. At tooltip 100, 50 and unit 5 the icon is at 10, 25 and the label at 80, 50 of size 30 by 10; one begin, one bind, two setup calls and two draws are counted. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `fade-the-transition-screen`): on page 2 the transition screen sets blend mode 0x203, takes the skin colour and its own alpha, and draws "TO BE CONTINUED" centred from (x - 15 glyph widths) / 2 and (y - glyph height) / 2. At 200, 50 with a glyph of 8 by 16 it draws at 40, 17. Any other page draws nothing but still calls the base draw. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `settle-the-skill-selection`): the skill selector takes the low 16 bits of its raw word, clamps above eight to zero, and on page 1 sets or clears bits 0x110 on the object. A positive index sets them, a zero index clears them, and any other page leaves them. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `close-the-inventory-on-the-key`): the inventory closes on the same 0x2002 mask and returns 1, using its stored callback with argument 0 when one exists and the form close otherwise. Any other mask returns the base result. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `key-the-skill-selector`): the selector routes its key to the base handler when a positive selection is open on a page above one, or when the word is nine with a clear mode byte; otherwise it closes with 0x40. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `dispatch-the-end-session`): pages 2, 4 and 8 resolve their first two actions into the resume call, the special page-4 call on form page 4, the page-8 trio plus form 0x100124800 on form page 2, or form 0x100123d30 with the last-round flag. Any other page calls its registered handler when present. Every path clears the action and field 0x204 and returns 0. `SG_GameMain` is 7%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `hover-the-menu`): the main menu takes the hover when it equals field 0x204 and zero otherwise; an unchanged selection returns 0, a change returns 3, and the sound plays only for a non-zero selection when the last stamp plus 200 is below the clock, which also updates the stamp. `SG_GameMain` is 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `key-the-main-menu`): mask 0x100 confirms a positive selection with the sound, 0x200 and 0x400 call forward and back, direction 0x8000 moves to max(selection + 1, 1) below the limit, and direction 0x4000 moves to selection - 1 above 1. The helper's result returns, or 0 when nothing runs. `SG_GameMain` is 9%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `detach-the-inventory-pane`): an inactive pane returns true without work; otherwise the detach marks the owner, places a visible child, decrements a stack above one or frees a stack of one, copies the loot pane's origin and runs the base method. The page override stores -3 when its argument is -1. `SG_GameGUI` is 5%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `type-the-menu-name`): a key without bit 0x2 goes to the base handler; otherwise the ten-byte name buffer shifts left, A-Z are lowered, and the buffer "barechoons" runs the submit routine. `SG_GameMain` is 10%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `bound-the-splash-screen`): the splash bounds write the screen and two -1 values and return -1. The destroy makes one release call on a set free flag, clears the splash global, frees each stored pointer, makes one list call for a positive count, always runs the base destructor and makes a final call only when freeing. `SG_GameMain` stays 10%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `draw-the-skill-selector`): page 2 draws a title at 12 units across, the place width less two units and 20 units high; any other page with a selection in 0 through 7 draws the measured icon centred at 16 units down and a label at 18 units across, the place width less two units and the height less 20 units. At place 10, 180 with a right edge of 110 and an icon of 20 the icon is at 40, 80 and the label at 90, 90 wide by 80 high. `SG_GameGUI` is 6%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `show-the-skill-tip`): a found row stores the object word less one as the index and the row word less one as the threshold, adds 0x10 to the flags, places at the anchor x and nine units plus its y, uses 24 units of height when global 0x2ad3c0 is 0 and 18 otherwise, sets visible, makes three calls and stamps three times the base plus the 0x120 offset. A missing row shows nothing. `SG_GameGUI` is 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `show-the-loot-and-journal`): the loot screen marks its owner, places a visible child and runs the base when it becomes active, returning true immediately when already inactive. The journal copies its origin, assigns its page string from the table and runs the base. `SG_GameGUI` stays 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `clear-the-character-view`): the view destructor releases on a set free flag and always runs the base, with a final call only when freeing. The clear releases the view when present, clears the owner's 0x130 word and the pointer, and always returns the base result. `SG_GameGUI` stays 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `show-the-skill-selector`): the show clears bits 0x60 on its object, applies a positive selection, or applies -1 when the word is nine or ten; any other word or a missing object applies nothing, and the base always runs. `SG_GameGUI` stays 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `confirm-the-class`): a positive class selection stores itself at 0x70 and one less in the global, returns 7, plays a sound when the sound global is positive, copies the page global and shows form 0x10012a870 with flag 0. A zero or negative selection returns 0x40 and does nothing else. `SG_GameMain` is 11%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `navigate-the-class`): mask 0x100 confirms a positive selection with result 7 and form 0x10012a870; 0x200 and 0x400 call the forward helper; direction 0x8000 moves to the next available class at or after the selection up to class 3, and 0x4000 to the previous from selection - 2 or to 1. The selection virtual runs only when the target differs. `SG_GameMain` is 13%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `hit-the-class-cards`): the class-card hit test derives its band from the clock scale as half of the clock less 1024 steps plus 140 steps, with a height of 512 steps, and lays up to four cards 288 steps apart at 256 steps wide, centred on the screen. A point must lie strictly inside the band and a card; it then names that card's ordinal plus one. At screen 1000, clock 2048 and scale 1 the cards occupy 212-468 and 500-756 and the band 652-1164. `SG_GameMain` is 15%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `confirm-the-main-menu`): a zero selection copies field 0x70 to 0x204 and returns 0x40; a positive selection on direction 0x1000 stores itself, copies it, plays the sound and returns 7, while another direction returns 0x40 without copying. The select stores its index, sounds it at most once per 200 units and returns 3. `SG_GameMain` is 16%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `start-the-cloth`): the cloth start adds 0x10000 to its flags, copies the seed, installs a 4x4 identity, sets its step to the square root of the count times 1e-6 and links a body carrying 0x10000000. The frame swap copies the pending buffer twice and clears it at one or less, or rotates the three buffers and stores the frame index above one. `TVCCloth` is 71%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `activate-the-cloth`): activation adds 0x20000000 to 0x300, 0x10000 to 8, 0x204030 to 0xc and 0x6000 to 0x188, sets phase 2 and scale 0.4, writes the handle, copies the root origin and adds 0x8000 when slow. The transform makes three calls per particle, the object origin on flag 0x20 and its extent always. `TVCCloth` is 72%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Done (change `rebuild-the-cloth`): a non-root cloth does nothing; a root copies its origin, stops on a mismatched override and otherwise transforms every particle point into a fresh buffer and submits it. `TVCCloth` is 73%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
Still open:

- The other emblem controls. A candidate's listed price is the rank square plus four fifths of the gear. A swept season grants its four item ids, and day 134 repeats the first row. A shop line prices at 1, 12, 144 or 1440 copper, and a sale is a quarter of the basket. A tier multiplies a match stake by one more than the tier. A recruitment costs 12 copper. Overnight rest and a trainer's lesson are in.
- Story side levels, scripted encounters, the role interpreter, map discovery.
- The other powers and their meshes. A blast charge, a barrier ripple, a wave front, a shell ripple, a shell vertex, a shell index list, a skinned section filter, a compacted vertex map, a section slot, an index room, a fade by the stored size of 800, a light that follows that fade, a second light pair, an index upload, a play draw that submits that upload, a torrent mix and a torrent stream are in. The Bevy render graph does not issue the original's GL calls.
- Character creation (race, body, face, hair, colours, class, skills).
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

1. Gait walking, clip blending, and parts' spheres fitted to their
   meshes (Phase 3). The standing leg bend, recovery steps, the
   support pins, the run overlay on the walk, the sprint overlay
   weight, the sprint clock, the sprint row's thrust, gait plants,
   the backpedal (Walk played backwards), knockback steps, and the
   tighter run cap (1 − 0.4·encumbrance while the sprint latch is kept,
   1 − 0.3·encumbrance on the walk keys) are in.
2. Inventory, equipment and containers, then blocking and parrying
   (Phase 4). Equipping by occupancy, container transfers, a parry that
   stops a blow and a shield's yellow leak are in. An inventory pane places
   icons, checks bulk and size, and hits the top-most one. A marked flag, a
   missing required mark, and an unlisted id are refused. A limit of 0
   does not refuse. An item card chooses a weapon, apparel, shield, or plain
   page and quarters its coin word. A weapon row is shown only above 5,
   at that byte times 0.05. Apparel coverage is always that byte times
   0.05, and its other rows appear only above 0.25. A plain card places
   its icon by dividing the span minus the icon size by 2. A stats card
   places it from the unit, and a picture wider than eleven units shifts
   it. A shield's weight is that byte times 0.01 and its impact is that
   byte times 0.05. Encumbrance uses that scale unless the byte is 0.
   Flags sharing a bit with 0x06000000 store weight as 1 and encumbrance
   as 8. Arena turns rank and points on. Story and a fresh practice
   turn them off. A shown rank reads the top three skill bits, and points
   are the low five bits times one half. A description phrase is chosen
   from the rows whose range holds the value, and a repeated long word
   adds nothing. A weapon sentence wraps that phrase with the name,
   "It is", ", but" or " and", and "A " or "An ". When the id word is
   -1, a 0x10 byte and the byte after it are deleted. An apparel sentence
   drops four letters from a bracket, says "They are " or "It is", joins
   the same side of 50 with ", but", strips a leading "A ", and keeps a
   non-final dot as a tail. Quality reads 14 made rows. A material from 1
   through 8 uppercases a side prefix, and a 0x10 there is replaced by
   the material name and a space. Injuries beyond damage are not.
3. The original's NPC brain: perception, targets and the combat state that
   drives the same inputs as the player (Phase 6). A path on the grid, the
   look-ahead, and an order that writes the player's heading and velocity
   are in. A contact's closeness, hostility and the notice, watch, engage
   or flee choice are in. Once the fight has started, the blow is an
   overhead, a thrust or a side, and energy under eight tenths of the
   maximum refuses it. A disturbance above 0.3 is walked to, and an
   alert of 0.2 looks around or faces a target more than 45° off. The
   roam, patrol and sentry cycles are in. Follow, sleep, and the
   wander-point picker are not.
4. Portal clipping and doors' open state, and level transitions
   (Phase 2) are in. The lid in the lights' cube faces is not.
5. `add-static-mesh-decoding` is still open, waiting on a visual mirroring
   check against the running game.
