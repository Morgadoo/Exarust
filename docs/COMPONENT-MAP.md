# Component map: the original against Exarust

Every function of the retail `Exanima.exe` (Exanima 0.9.5, Free Pascal 3.2.2, Win64), placed in a component and a section, and each component rated for how much of its behaviour Exarust reproduces. The interactive treemaps are published at <https://claude.ai/artifact/GybAiwT9utFDbJTSaPLUkC>. This page is the summary that lives in the tree.

## Headline

- **8,783 functions**, 2,616 KB of code, in **445 components** across 16 sections.
- Weighted by code size, with code the retail game never runs left out (1,343 functions, 163 KB): **14.4% reimplemented** by Exarust's own code and **5.7% provided** by Bevy, wgpu, rapier or the Rust standard library.
- Weighted by function count (zero-byte imported names left out): 13.3% and 12.5%. The runtime's many tiny RTL routines make this figure lean towards the engine, which is why code size is the headline.
- The 24 combat-rule functions (52 KB of damage, armour and weapon-stat rules inside body, part, AI and item owners) are **27.3%** reproduced.

## By section

| section | functions | KB | components | reimplemented | by engine | largest missing |
|---|---:|---:|---:|---:|---:|---|
| Runtime & platform | 2,407 | 293 | 26 | 4% | 72% | Steam Input action bindings, Win32 clipboard image/icon glue, Steam API glue |
| Core, math & threading | 282 | 37 | 11 | 41% | 1% | RFT_BoundSphere, Bounding-sphere vector helpers (FLX_Entities by address) |
| Data, assets & I/O | 440 | 91 | 26 | 21% | 0% | RFE_Export_RFC2, Graphics & key settings (ini), RF_StoreSettings |
| Rendering & GPU | 823 | 334 | 63 | 35% | 3% | RFE_MeshSubdivide, RFP_Mipmaps_Kaiser, RF_Polyhedra |
| World, levels & terrain | 454 | 174 | 31 | 29% | 7% | TGridEnvManager, TRBDDoor, RF_Dynamics_Game |
| Physics, bodies & cloth | 363 | 242 | 13 | 20% | 0% | TVCCloth, RF_Dynamics_Volumes, TVoxelMap |
| Characters, motion & animation | 506 | 198 | 23 | 27% | 0% | RF_Morphing, Equipment effects and focus-item slot (RFE_Effects_Electricity by address), Golem guardians and alarm zones (RFE_Effects_Electricity by address) |
| Combat, weapons & damage | 14 | 3 | 3 | 0% | 0% | TWeaponBlood, RFE_Effects_Blood, TBloodSprays |
| Items, materials & crafting | 341 | 140 | 40 | 17% | 0% | TGUIItemContainer, Item description phrases, SG_ItemGUI |
| AI & behaviour | 437 | 206 | 17 | 1% | 0% | TMotileAI, SG_PlayerControls, TPathFinder |
| Thaumaturgy & effects | 697 | 296 | 83 | 3% | 0% | TThaumTree, SG_Thaum_Mind, TForceBlastEffect |
| Audio | 59 | 18 | 4 | 28% | 0% |  |
| Script, dialogue & story | 184 | 46 | 11 | 0% | 0% | RFG_Dialogue, FG_AIFunctions, RFG_Scripts |
| Game modes, arena & session | 646 | 193 | 23 | 0% | 0% | XA_Arena, XA_SaveGames, SG_GameMain |
| UI | 959 | 312 | 63 | 0% | 0% | TGUIDrawSkin, TGSLines, SG_GameGUI |
| Other, tools & editor | 171 | 33 | 8 | 0% | 0% | Debug command registry (unit before RF_Dynamics) |

- **Runtime & platform.** The Free Pascal runtime, the Windows imports and the engine's thin platform layer: program entry, window and OpenGL context, the frame loop with its pacing, and Steam with Steam Input gamepads. Much of the linked runtime never runs. Rust std, winit, Bevy and wgpu cover most of what does run; Steam, gamepads, the custom cursor, fullscreen display modes and the error message box are missing.
- **Core, math & threading.** The engine's foundation: vector and matrix maths with fast trig, colour conversions, 16-byte asset names, a seeded random generator, callback lists, text formatting, a smallest-enclosing-sphere solver for the sun shadow, and the engine's own threads and parallel-for pool. Exarust covers most of the maths, random generator and colour helpers; the sphere solver and worker threads are missing.
- **Data, assets & I/O.** The I/O stack: file and pack reads, the project bootstrap, the .rfc level reader and writer, object, character and locale databases with their string table, cached prop instancing, background streaming and settings files. Exarust reads packs, the project and much of the .rfc content; its databases are read-only and partial, and saving, settings, locales, object strings and streaming are missing.
- **Rendering & GPU.** The OpenGL renderer: textures and the .rfi codec, meshes, template-built shaders, the fixed pass order, 2x supersampling, bloom, cube and sun shadows, per-frame voxel GI and terrain compositing, plus camera control and effect geometry. Exarust rebuilds the core path on Bevy with its own shaders; mesh subdivision, icon rendering, image resampling, decals, refraction and effect draws are missing.
- **World, levels & terrain.** Holds a loaded level: scene graph and per-frame tick, the tile-grid dungeon builder, rooms and portals, streamed terrain with rule-blended materials and plants, kd-trees, sun, fog and weather, and doors, levers, pressure plates and teleporters with their signals. Exarust builds grid levels and terrain at load, with rapier for the kd-trees; rooms, mechanisms, day/night and weather are missing.
- **Physics, bodies & cloth.** The game's own 83 Hz Verlet physics: tetrahedral rigid bodies with spheres voxelised from their meshes, linked and collided with level meshes, plus wind-blown cloth and the character body that plays clips as target poses, balances, swings and takes hits. Exarust steps the limbs with pose drive, balance and falls on rapier colliders; cloth, body-to-body contact, props and voxelising are missing.
- **Characters, motion & animation.** Builds characters from a record and race: model, physique morphs, limb bodies and muscles, apparel merged into one auto-skinned mesh, painted skin and apparel, the motion library, item effects and golems. Exarust spawns placed characters with auto-skin weights, GPU skinning, painted apparel and clip-driven limbs; morphs, skin textures, hair, full records, item effects and golems are missing.
- **Combat, weapons & damage.** Holds only the blood visuals of combat: a spray jet at each bleeding wound and a stain that creeps down the weapon with the red damage it deals. Exarust has none of it. The combat maths lives in the body, limb, AI and item owners and is rated as 24 combat rules: 27% reproduced, mainly damage, health, armour layers, daze and weapon stats; recoil, grips, shields and positioning are missing.
- **Items, materials & crafting.** Builds weapons from .fty recipes (seeded part, deform and material rolls, stats), rolls item quality, condition and dyes, paints item and apparel textures from 25 materials, and runs equipping and the inventory, loot and container windows with icons and item cards. Exarust builds factory weapons with stats and paints cloth, leather and steel; other materials, equipping and item UI are missing.
- **AI & behaviour.** Decides what characters do: player input and steering, the NPC brain (personality, idle activities from roles, hostility and morale, fight or flee, attack choice), roles and relations, companion scavenging, a grid A* pathfinder and the AI manager. Exarust has part of the player's steering and footwork and a placeholder duellist with invented rules; NPC behaviour and pathfinding are missing.
- **Thaumaturgy & effects.** Runs thaumaturgy: per-body powers and focus, the power tree and quick bar, AI casting, and the Mind, Force, Energy and Displacement powers, each a render node, simulation, GI light and AI-visible entity. It also holds shared effects: particle fire, torch and burning-item flames, electric arcs, teleport visuals, portals, rain. Exarust has only the small fire's particles and light, and no powers.
- **Audio.** An OpenAL Soft front end: it binds OpenAL and EFX at run time, opens the device lazily (silent until then), loads sounds from Sound.rpk, ranks 512 voices onto 64 sources each frame, and adds reverb, area ambience and crossfading music. Exarust plays hit and footstep sounds and a looping level track via Bevy's rodio backend; voice ranking, variant sets, reverb, ambience and crossfades are missing.
- **Script, dialogue & story.** The story layer: a small interpreter for role and dialogue scripts, and the conversation engine that finds a dialogue two characters can hold through their roles and mood, keeps per-relation topic memory and drives the narrator, reply choices, journal, loading tips and scripted world objects. Exarust has none of it; its script crate is an empty plugin.
- **Game modes, arena & session.** The game's flow: main, pause and end-of-fight menus, switching between practice, story and arena, saving to packages with checkpoints and permadeath, class select, character creation, discovery XP and the whole arena career (company, hirelings, loadouts, matches, shop). Exarust has only debug stand-ins: start spots, a one-on-one arena opponent, a character preview and emblem texture lookup.
- **UI.** The original's own GUI toolkit (windows, controls, skins, bitmap fonts, icon atlas, tooltips) and every screen built on it: splash, menus, settings with key bindings and GPU benchmark, HUD, character view, skill trees, manual, portal screen, creation pickers and arena dialogs. Exarust has no GUI layer, only a bitmap-font FPS readout, the in-game world cursor, the HUD's health and focus bars with the low-health heartbeat border, launch flags, mouse input and a window-sized view.
- **Other, tools & editor.** Developer tools that never run in retail (166 of 171 functions): the dormant object appearance editor and weather panel, editor taskbar menus, the file dialog, tree view, popup menu and render-options panel. The only live part is a small debug command registry run from key bindings, which Exarust does not have.

## Largest missing components

Ordered by code size; these are where most of the original still has no counterpart.

| component | section | functions | KB | coverage |
|---|---|---:|---:|---:|
| XA_Arena | Game modes, arena & session | 219 | 71 | 2% |
| TMotileAI | AI & behaviour | 85 | 60 | 2% |
| SG_PlayerControls | AI & behaviour | 122 | 47 | 3% |
| TThaumTree | Thaumaturgy & effects | 76 | 38 | 0% |
| TGUIDrawSkin | UI | 73 | 35 | 0% |
| TPathFinder | AI & behaviour | 24 | 32 | 0% |
| TVCCloth | Physics, bodies & cloth | 50 | 30 | 0% |
| XA_SaveGames | Game modes, arena & session | 96 | 27 | 3% |
| TGUIItemContainer | Items, materials & crafting | 44 | 26 | 0% |
| TGSLines | UI | 23 | 24 | 0% |
| SG_GameGUI | UI | 62 | 22 | 1% |
| SG_Thaum_Mind | Thaumaturgy & effects | 45 | 21 | 0% |
| TGUIForm | UI | 77 | 20 | 0% |
| SG_GameMain | Game modes, arena & session | 59 | 20 | 3% |
| SG_Character_Data | Characters, motion & animation | 76 | 20 | 9% |
| SG_ObjectDB2 | Data, assets & I/O | 116 | 19 | 11% |
| RFG_Dialogue | Script, dialogue & story | 62 | 18 | 0% |
| TSkillUI | UI | 29 | 18 | 0% |
| RFE_MeshSubdivide | Rendering & GPU | 23 | 18 | 0% |
| TPlayerBehaviour | AI & behaviour | 31 | 17 | 16% |
| TMatchBoard | Game modes, arena & session | 30 | 17 | 0% |
| SG_Character_Create | Game modes, arena & session | 90 | 17 | 1% |
| TCombatant | AI & behaviour | 30 | 17 | 0% |
| TForceBlastEffect | Thaumaturgy & effects | 22 | 17 | 0% |
| RF_Morphing | Characters, motion & animation | 25 | 16 | 0% |

## Best covered components

| component | section | KB | coverage | Exarust |
|---|---|---:|---:|---|
| TVXGIManager | Rendering & GPU | 29 | 55% | exa_render/src/gi.rs; exa_render/src/gi.wesl |
| TMesh | Rendering & GPU | 25 | 60% | exa_formats/src/rfc.rs; exa_assets/src/model.rs |
| RF_Skinning | Characters, motion & animation | 25 | 55% | exa_assets/src/skin.rs; exa_assets/src/model.rs |
| FLX_ItemFactory | Items, materials & crafting | 24 | 50% | exa_formats::factory; exa_formats::weapon_stats |
| RF_Import_RFC2 | Data, assets & I/O | 17 | 51% | exa_formats/src/rfc.rs; exa_formats/src/chunk.rs |
| Texture compression formats (DXT/LATC) | Rendering & GPU | 11 | 75% | exa_formats/src/rfi.rs; exa_assets/src/texture.rs |
| TTerrainGrass | World, levels & terrain | 12 | 60% | exa_world/src/terrain/vegetation.rs; exa_world/src/terrain/mod.rs |
| RF_3DView | Rendering & GPU | 11 | 60% | exa_render/src/supersample.rs; exa_render/src/resolve.wesl |
| FLX_System | Core, math & threading | 12 | 51% | glam via bevy::math; exa_game/src/player.rs |
| Engine math (vectors, matrices, fast trig) | Core, math & threading | 8 | 73% | exa_game/src/player.rs; exa_game/src/torch.rs |
| TSkinController2 | Characters, motion & animation | 8 | 65% | exa_anim/src/lib.rs; Bevy SkinnedMesh / SkinnedMeshInverseBindposes GPU skinning |
| TNavCameraControl | Rendering & GPU | 8 | 55% | exa_game/src/player.rs |
| TShadowCubeMap | Rendering & GPU | 5 | 83% | exa_render/src/cube_shadows.rs; exa_render/src/cube_shadow.wesl |
| RF_Terrain_Materials | World, levels & terrain | 6 | 65% | exa_world/src/terrain/blend.rs; exa_world/src/terrain/mod.rs |
| .rfi image codec | Rendering & GPU | 6 | 56% | exa_formats/src/rfi.rs; exa_assets/src/model.rs |
| TTerrainManager | World, levels & terrain | 5 | 61% | exa_formats/src/terrain.rs; exa_world/src/terrain/mod.rs |
| TGridEnvBuilder | World, levels & terrain | 4 | 65% | exa_world/src/grid.rs |
| OpenAL loader | Audio | 4 | 58% | rodio 0.22 / cpal 0.17; bevy_audio 0.20 lib.rs |
| FLX_PostProcess | Rendering & GPU | 2 | 85% | exa_render/src/post.rs; exa_render/src/post.wesl |
| TScene | World, levels & terrain | 3 | 59% | exa_world/src/lib.rs; exa_world/src/shadow_links.rs |

## Combat rules

| rule | inside | bytes | coverage |
|---|---|---:|---:|
| NPC combat state | TMotileAI | 18,453 | 5% |
| damage routine: hit point, speed/impact, zone factors, armour roll | TMotileDynamics | 13,157 | 40% |
| per-part weapon stat bytes of factory weapons | FLX_ItemFactory | 3,692 | 90% |
| armour layers from equipped apparel | SG_CharacterGen | 3,680 | 40% |
| weapon mesh assembly (feeds striking-part extent) | FLX_ItemFactory | 3,282 | 40% |
| combat positioning score (spacing/reach) | TCombatant | 3,152 | 0% |
| factory weapon spawn, striking-part extent | FLX_ItemFactory | 1,349 | 30% |
| builds the weapon stat record from the item weapon section | TMotilePart | 1,284 | 55% |
| grip/owner pass over the weapon stat record | TMotilePart | 1,244 | 0% |
| re-runs the grip pass for each held weapon | TMotilePart | 798 | 0% |
| apparel instance quality/condition | Item description phrases | 780 | 40% |
| factory item condition | Item description phrases | 532 | 60% |
| applies damage to a body | TMotileDynamics | 481 | 50% |
| attacker recoil after a hit | TMotileDynamics | 266 | 0% |
| factory item stats | FLX_ItemFactory | 247 | 55% |
| shield stat block | TMotilePart | 187 | 0% |
| weapon mass distribution | TMotilePart | 178 | 0% |
| weapon mass distribution | TMotilePart | 146 | 0% |
| sweet spot | TMotilePart | 141 | 0% |
| daze | TMotilePart | 138 | 75% |
| enter combat state | TMotileAI | 116 | 15% |
| blade length | TMotilePart | 79 | 60% |
| weapon mass distribution | TMotilePart | 66 | 0% |
| fetches the weapon stat record up the body chain | TMotilePart | 38 | 50% |

## How it was made

- **Inventory.** Ghidra (through REA) lists every function. Virtual methods are attributed to classes through the Free Pascal VMTs and RTTI, helpers through the call graph, libraries by imports and strings, and the rest by unit and neighbours. Each function is counted once.
- **Placement.** Classes follow `research/engine-classes.md`, unless an override gives a reason. Unit routines follow their unit; anonymous clusters follow their strings and callers. Where the research notes show that some of an owner's functions serve another subsystem, those functions are split out by address.
- **Rating.** One agent per section described each component from the decompiled code. A second rated Exarust's code against it, and a third tried to refute every rating. A completeness critic then checked the whole map; its 39 findings were fixed and verified the same way, and a final critic re-checked the result.
- **Rules.** Coverage is the share of a component's code, by bytes, whose behaviour Exarust reproduces. Missing is below 5%, partial below 80%, reimplemented from 80%. "Provided by the engine" counts only behaviour that Bevy, wgpu, rapier or std supplies and Exarust actually uses. Each Exarust behaviour is credited to one component only. Editor-only tools, dormant handlers and unused RTL units are not applicable and are left out of the percentages.
- **The README image.** `docs/img/coverage-map-{dark,light}.png` are this map's coverage view, rendered by `context/tools/coverage_image.py` whenever the ratings change.
- **Where the data lives.** The inventory, the notes and the build scripts are research material and stay in the git-ignored `context/` (`context/re/component-map/`, `context/tools/sections/`, `context/tools/build_map_data.py`, this page from `context/tools/component_map_doc.py`).
