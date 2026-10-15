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
  index, `.rfi` decoding of every texture format (BC1/3/4/5, BGR8, BGRA8,
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
| Materials (`BA01`, `.rml` libraries): ✅ texture layers and slots; parameter block (colours, shader/plug-in IDs) still open | `exa_formats::material` | map the 184-byte parameter block to plug-in classes seen in the binary (`TMaterialPlugIn`, `TRTMatPlugIn`, `TProcMaterial`) |
| Skinned meshes, weights, morph targets (`*base.rfc`, `*morph.rfc`) | `exa_formats::rfc` | |
| Skeleton and motions (`.rsq`) | `exa_formats::rsq` | 18 bones, 48 clips, keyframe controllers (linear, Catmull-Rom) |
| Level grid (`3DED`), level sections (`3DE10000`, `DBC0`, …), object placement | `exa_formats::level` | wall/floor sets `gews_*` / `gefs_*` |
| Terrain (`.rft`) and terrain tables (`.ftb`) | `exa_formats::terrain` | heightfield, layers, grass |
| ~~Packed `.rfi` storage~~ | `exa_formats::rfi` | done in Phase 0 (RLE) |
| Databases (`races`, `objdb`, `charroles`, `locales`, `objstrings`) | `exa_formats::rdb` | record schemas |
| Factories, actors, powers, dialogue (`.rcd`) | `exa_formats::{factory,actor,power,dialogue}` | |
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
Remaining: ceiling caps of terrain cells, per-room mesh merging, rooms and
portals, terrain gloss textures, fine displacement, the sky, time of day
and weather of the outdoor arenas,
sectors/portals (the room flood fill is understood),
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
up ([formats/RSQ.md](formats/RSQ.md)). Still to do: link PD muscles,
recovery steps, foot planting, collisions between bodies.

This phase carries the most risk and defines how the game feels.

- `exa_anim`: skeleton, clip sampling and blending, procedural layers
  (aim, look-at, stance, idle noise) that produce **target poses**.
- `exa_physics`: fixed-step simulation at the original's 83 Hz. Rigid bodies and
  collision for props and level geometry. **Articulated character bodies**
  driven by joint motors toward the target pose (an "active ragdoll"), with
  balance (`Rebalance`, falls, get-ups), momentum and joint limits.
  - Decision: prototype collision and rigid bodies on **avian3d** (ECS-native,
    deterministic mode). Keep the character solver in-house behind
    `exa_physics` so it can later be tuned or replaced to match the original.
- `exa_character`: race body plus morphs, skinning, hair, apparel meshes fitted
  to the body (the original uses skin pivots and edge splitting, see
  `TSkinBuilder` and `TEdgSplt`).
- Player control matching the original: movement relative to the camera, the
  body facing the mouse, combat mode toggle, attacks steered by mouse
  movement, run/walk, crouch, interact. Camera: elevated third-person follow
  with zoom.

**M3 "standing man":** a fully dressed character walks around a level under
player control, stumbles when pushed, falls and gets up.

### Phase 4 — Combat and items

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

### Phase 5 — Visual and audio fidelity

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
  music.

**M5 "looks and sounds like Exanima":** reference shots taken from matching
camera positions in the original and in Exarust are hard to tell apart.

### Phase 6 — AI

- Perception (sight, hearing), navigation on tile grids and terrain
  (generated navmesh plus grid A*), activities and idles.
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

## 7. Immediate next steps (Phase 1, in order)

1. ✅ OpenSpec change `add-static-mesh-decoding`: static meshes, material
   texture layers, `exa-tool export-gltf`, `exa-tool corpus meshes`. Waiting
   on a visual mirroring check against the running game.
2. ✅ Level props and grid header in Bevy (`exarust level`).
3. ✅ Tile-grid builder: walls, floors and ceilings from the sets.
4. ✅ Skeleton and motions (`.rsq`).
5. Seam stitching between body parts, apparel on bodies, then the player
   character in a level with physics (avian3d) and controls.
4. `exa-tool corpus` coverage report.
5. `exa_assets` asset source and loaders, then a Bevy level viewer (M2).
