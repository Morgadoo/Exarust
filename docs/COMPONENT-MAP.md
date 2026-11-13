# Component map: the original against Exarust

Every function of the retail `Exanima.exe` (Exanima 0.9.5, Free Pascal 3.2.2, Win64), placed in a component and a section, and each component rated for how much of its behaviour Exarust reproduces. The interactive treemaps are published at <https://claude.ai/artifact/GybAiwT9utFDbJTSaPLUkC>. This page is the summary that lives in the tree.

## Headline

Two numbers are published while the function ledger is being completed (see `docs/roadmap/README.md`, "Two numbers during the transition"): the **estimate**, reviewers' ratings per component, and the **ledger**, the share of the counted bytes whose every function is in a final state.

- **8,783 functions**, 2,616 KB of code, in **445 components** across 16 sections.
- **Estimate 82.3% + 5.1% engine; ledger 48.3% closed.**
- Weighted by code size, with code the retail game never runs left out (1,343 functions, 163 KB): **82.3% reimplemented** by Exarust's own code and **5.1% provided** by Bevy, wgpu, rapier or the Rust standard library. This is the estimate.
- The ledger, function by function over 2,453 KB of counted code (not-applicable rows left out): **48.3% closed**, of which 37.8% live (reimplemented), 8.8% engine and 1.7% closed without code (unreachable or deviation). Still open: 39.6% partial and 12.1% uncalled, missing or unaudited.
- Weighted by function count (zero-byte imported names left out): 73.8% and 12.1%. The runtime's many tiny RTL routines make this figure lean towards the engine, which is why code size is the headline.
- The 24 combat-rule functions (52 KB of damage, armour and weapon-stat rules inside body, part, AI and item owners) are **90.6%** reproduced.

## By section

The first two percentages are the estimate; the next six come from the ledger (closed = live + engine + no code; open = uncalled, missing or unaudited).

| section | functions | KB | components | reimplemented | by engine | ledger closed | live | engine | no code | partial | open | largest missing |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| Runtime & platform | 2,407 | 293 | 26 | 14% | 75% | 85% | 2% | 82% | 1% | 10% | 5% |  |
| Core, math & threading | 282 | 37 | 11 | 86% | 1% | 52% | 34% | 16% | 2% | 36% | 12% |  |
| Data, assets & I/O | 440 | 91 | 26 | 84% | 0% | 51% | 43% | 6% | 2% | 34% | 15% |  |
| Rendering & GPU | 823 | 334 | 63 | 83% | 1% | 49% | 38% | 10% | 2% | 37% | 14% |  |
| World, levels & terrain | 454 | 174 | 31 | 82% | 0% | 51% | 44% | 4% | 3% | 37% | 12% |  |
| Physics, bodies & cloth | 363 | 242 | 13 | 94% | 0% | 24% | 23% | 1% | 0% | 64% | 12% |  |
| Characters, motion & animation | 506 | 198 | 23 | 85% | 0% | 43% | 38% | 4% | 0% | 45% | 13% |  |
| Combat, weapons & damage | 14 | 3 | 3 | 83% | 0% | 32% | 28% | 3% | 0% | 68% | 0% |  |
| Items, materials & crafting | 341 | 140 | 40 | 83% | 0% | 39% | 36% | 2% | 1% | 51% | 10% | TArsenalContFrame |
| AI & behaviour | 437 | 206 | 17 | 96% | 0% | 43% | 40% | 1% | 2% | 52% | 5% |  |
| Thaumaturgy & effects | 697 | 296 | 83 | 87% | 0% | 57% | 55% | 1% | 0% | 37% | 5% |  |
| Audio | 59 | 18 | 4 | 90% | 0% | 82% | 60% | 22% | 0% | 16% | 1% |  |
| Script, dialogue & story | 184 | 46 | 11 | 87% | 0% | 84% | 82% | 2% | 0% | 12% | 4% |  |
| Game modes, arena & session | 646 | 193 | 23 | 91% | 0% | 46% | 44% | 2% | 0% | 43% | 11% |  |
| UI | 959 | 312 | 63 | 86% | 0% | 42% | 34% | 1% | 7% | 30% | 28% |  |
| Other, tools & editor | 171 | 33 | 8 | 55% | 0% | 100% | 82% | 18% | 0% | 0% | 0% |  |

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
- **Thaumaturgy & effects.** Runs thaumaturgy: per-body powers and focus, the power tree and quick bar, AI casting, and the Mind, Force, Energy and Displacement powers, each a render node, simulation, GI light and AI-visible entity. It also holds shared effects: particle fire, torch and burning-item flames, electric arcs, teleport visuals, portals, rain. Exarust has the casts' gates and prices, the active power classes and the force and…
- **Audio.** An OpenAL Soft front end: it binds OpenAL and EFX at run time, opens the device lazily (silent until then), loads sounds from Sound.rpk, ranks 512 voices onto 64 sources each frame, and adds reverb, area ambience and crossfading music. Exarust plays hit and footstep sounds and a looping level track via Bevy's rodio backend; voice ranking, variant sets, reverb, ambience and crossfades are missing.
- **Script, dialogue & story.** The story layer: a small interpreter for role and dialogue scripts, and the conversation engine that finds a dialogue two characters can hold through their roles and mood, keeps per-relation topic memory and drives the narrator, reply choices, journal, loading tips and scripted world objects. Exarust has none of it; its script crate is an empty plugin.
- **Game modes, arena & session.** The game's flow: main, pause and end-of-fight menus, switching between practice, story and arena, saving to packages with checkpoints and permadeath, class select, character creation, discovery XP and the whole arena career (company, hirelings, loadouts, matches, shop). Exarust has only debug stand-ins: start spots, a one-on-one arena opponent, a character preview and emblem texture lookup.
- **UI.** The original's own GUI toolkit (windows, controls, skins, bitmap fonts, icon atlas, tooltips) and every screen built on it: splash, menus, settings with key bindings and GPU benchmark, HUD, character view, skill trees, manual, portal screen, creation pickers and arena dialogs. Exarust has no GUI layer, only a bitmap-font FPS readout, launch flags, mouse input and a window-sized view.
- **Other, tools & editor.** Developer tools that never run in retail (166 of 171 functions): the dormant object appearance editor and weather panel, editor taskbar menus, the file dialog, tree view, popup menu and render-options panel. The only live part is a small debug command registry run from key bindings, which Exarust does not have.

## Largest missing components

Ordered by code size; these are where most of the original still has no counterpart.

| component | section | functions | KB | coverage |
|---|---|---:|---:|---:|
| Steam API glue | Runtime & platform | 16 | 1 | 7% |
| TArsenalContFrame | Items, materials & crafting | 1 | 0 | 0% |
| Win32 imports misc | Runtime & platform | 25 | 0 | 14% |

## Best covered components

| component | section | KB | coverage | Exarust |
|---|---|---:|---:|---|
| TMotileDynamics | Physics, bodies & cloth | 121 | 97% | exa_physics/src/body_feet.rs; exa_physics/src/body_gait.rs |
| XA_Arena | Game modes, arena & session | 71 | 96% | exa_game/src/arena_outfit.rs; exa_game/src/arena_outfit_data.rs |
| TMotileAI | AI & behaviour | 60 | 99% | exa_ai/src/chase.rs; exa_ai/src/route.rs |
| SG_PlayerControls | AI & behaviour | 47 | 99% | exa_game/src/player_controls.rs; exa_game/src/idle_orbit.rs |
| TMotilePart | Characters, motion & animation | 53 | 86% | exa_physics/src/part.rs; exa_physics/src/grip.rs |
| SG_CharacterGen | Characters, motion & animation | 43 | 85% | exa_character/src/layers.rs; exa_character/src/look.rs |
| TThaumTree | Thaumaturgy & effects | 38 | 91% | exa_game/src/thaum/tree.rs; exa_game/src/power_screen.rs |
| TGUIDrawSkin | UI | 35 | 93% | exa_ui/src/skin.rs; exa_ui/src/gui_draw.rs |
| TPathFinder | AI & behaviour | 32 | 91% | exa_world/src/nav/path.rs; exa_world/src/nav/mover.rs |
| TVCCloth | Physics, bodies & cloth | 30 | 96% | exa_physics/src/cloth.rs; exa_world/src/cloth.rs |
| TVXGIManager | Rendering & GPU | 29 | 97% | exa_render/src/gi.rs; exa_render/src/gi.wesl |
| TSimSector | Physics, bodies & cloth | 28 | 94% | exa_physics/src/sector.rs; exa_physics/src/body_pairs.rs |
| XA_SaveGames | Game modes, arena & session | 27 | 96% | exa_formats/src/save_package.rs; exa_formats/src/arena_save.rs |
| TGUIItemContainer | Items, materials & crafting | 26 | 90% | exa_game/src/item_container.rs; exa_items/src/card.rs |
| FLX_ItemFactory | Items, materials & crafting | 24 | 96% | exa_formats/src/factory.rs; exa_formats/src/weapon_stats.rs |
| TMesh | Rendering & GPU | 25 | 91% | exa_formats/src/rfc.rs; exa_assets/src/model.rs |
| RF_Skinning | Characters, motion & animation | 25 | 91% | exa_assets/src/skin.rs; exa_assets/src/model.rs |
| RF_SceneNodes | World, levels & terrain | 24 | 92% | exa_world/src/scene_nodes.rs; exa_world/src/light_flood.rs |
| TGSLines | UI | 24 | 89% | exa_ui/src/skin.rs; exa_ui/src/line_skin.rs |
| SG_Thaum_Mind | Thaumaturgy & effects | 21 | 96% | exa_game/src/mind/casts.rs; exa_game/src/mind/sight.rs |

## Combat rules

| rule | inside | bytes | coverage |
|---|---|---:|---:|
| NPC combat state | TMotileAI | 18,453 | 88% |
| damage routine: hit point, speed/impact, zone factors, armour roll | TMotileDynamics | 13,157 | 93% |
| per-part weapon stat bytes of factory weapons | FLX_ItemFactory | 3,692 | 90% |
| armour layers from equipped apparel | SG_CharacterGen | 3,680 | 88% |
| weapon mesh assembly (feeds striking-part extent) | FLX_ItemFactory | 3,282 | 90% |
| combat positioning score (spacing/reach) | TCombatant | 3,152 | 95% |
| factory weapon spawn, striking-part extent | FLX_ItemFactory | 1,349 | 87% |
| builds the weapon stat record from the item weapon section | TMotilePart | 1,284 | 92% |
| grip/owner pass over the weapon stat record | TMotilePart | 1,244 | 95% |
| re-runs the grip pass for each held weapon | TMotilePart | 798 | 93% |
| apparel instance quality/condition | Item description phrases | 780 | 92% |
| factory item condition | Item description phrases | 532 | 95% |
| applies damage to a body | TMotileDynamics | 481 | 95% |
| attacker recoil after a hit | TMotileDynamics | 266 | 95% |
| factory item stats | FLX_ItemFactory | 247 | 95% |
| shield stat block | TMotilePart | 187 | 90% |
| weapon mass distribution | TMotilePart | 178 | 100% |
| weapon mass distribution | TMotilePart | 146 | 100% |
| sweet spot | TMotilePart | 141 | 95% |
| daze | TMotilePart | 138 | 100% |
| enter combat state | TMotileAI | 116 | 95% |
| blade length | TMotilePart | 79 | 95% |
| weapon mass distribution | TMotilePart | 66 | 100% |
| fetches the weapon stat record up the body chain | TMotilePart | 38 | 90% |

## Ledger by component

Every counted component, by section and then by size. "Estimate" is the component's rating; the other columns are the ledger's shares of its counted bytes. Not-applicable components have no counted bytes and are left out.

| component | section | KB | estimate | ledger closed | live | engine | no code | partial | open |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| FPC System RTL | Runtime & platform | 69 | 96% engine | 99% | 0% | 99% | 0% | 1% | 0% |
| FPC SysUtils | Runtime & platform | 42 | 95% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| RF_OS_Functions | Runtime & platform | 12 | 68% | 32% | 11% | 19% | 2% | 54% | 14% |
| Steam Input action bindings | Runtime & platform | 11 | 75% | 4% | 4% | 0% | 0% | 39% | 57% |
| RF_Main | Runtime & platform | 8 | 80% | 41% | 21% | 20% | 0% | 54% | 5% |
| FPC RTL: lnfodwrf (DWARF line info) | Runtime & platform | 7 | 95% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| OpenGL loader & context setup | Runtime & platform | 6 | 75% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| FPC RTL: exeinfo (module info for backtraces) | Runtime & platform | 2 | 95% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| Win32 clipboard image/icon glue | Runtime & platform | 2 | 45% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| Steam API glue | Runtime & platform | 1 | 7% | 99% | 0% | 31% | 68% | 0% | 1% |
| Program main (Exanima.lpr) | Runtime & platform | 1 | 37% | 35% | 0% | 35% | 0% | 57% | 8% |
| FPC Math helpers (FPC SysUtils by address) | Runtime & platform | 0 | 95% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| Windows-unit global memory helpers (FPC RTL: exeinfo (module info for backtraces) by address) | Runtime & platform | 0 | 95% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| FLX_System | Core, math & threading | 12 | 79% | 43% | 14% | 29% | 0% | 22% | 35% |
| RFT_BoundSphere | Core, math & threading | 8 | 95% | 54% | 54% | 0% | 0% | 46% | 0% |
| Engine math (vectors, matrices, fast trig) | Core, math & threading | 8 | 97% | 26% | 20% | 6% | 0% | 71% | 3% |
| Core string/number formatting utilities | Core, math & threading | 4 | 94% | 86% | 76% | 4% | 6% | 12% | 2% |
| Colour helpers and parallel-for dispatch (RF_kdTrees by address) | Core, math & threading | 2 | 75% | 77% | 61% | 2% | 14% | 23% | 0% |
| RF_WorkerThreads | Core, math & threading | 1 | 46% | 100% | 11% | 89% | 0% | 0% | 0% |
| RF_MultiLoop | Core, math & threading | 0 | 70% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TCritSection | Core, math & threading | 0 | 95% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| Bounding-sphere vector helpers (FLX_Entities by address) | Core, math & threading | 0 | 90% | 100% | 100% | 0% | 0% | 0% | 0% |
| TThread@FLX_System | Core, math & threading | 0 | 50% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TThread@RF_OS_Functions | Core, math & threading | 0 | 50% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| SG_ObjectDB2 | Data, assets & I/O | 19 | 96% | 59% | 53% | 2% | 4% | 25% | 16% |
| RF_Import_RFC2 | Data, assets & I/O | 17 | 83% | 88% | 84% | 1% | 2% | 9% | 4% |
| FLX_Content_Stream | Data, assets & I/O | 12 | 84% | 26% | 22% | 4% | 0% | 62% | 11% |
| RFE_Export_RFC2 | Data, assets & I/O | 11 | 90% | 23% | 18% | 0% | 5% | 75% | 2% |
| RF_DataIO | Data, assets & I/O | 7 | 93% | 29% | 9% | 20% | 0% | 12% | 58% |
| Graphics & key settings (ini) | Data, assets & I/O | 4 | 88% | 92% | 84% | 8% | 0% | 3% | 5% |
| RF_DBObjects | Data, assets & I/O | 4 | 52% | 8% | 5% | 3% | 0% | 77% | 15% |
| TProjectManager | Data, assets & I/O | 3 | 66% | 26% | 0% | 26% | 0% | 74% | 0% |
| SG_Character_DBs | Data, assets & I/O | 3 | 85% | 90% | 89% | 1% | 0% | 1% | 9% |
| RF_ObjectLibs | Data, assets & I/O | 2 | 70% | 33% | 30% | 3% | 0% | 55% | 13% |
| RF_StoreSettings | Data, assets & I/O | 2 | 97% | 95% | 93% | 1% | 0% | 5% | 0% |
| RF_UIDManager | Data, assets & I/O | 2 | 50% | 21% | 17% | 3% | 0% | 14% | 65% |
| TMemBlockLibrary | Data, assets & I/O | 2 | 60% | 14% | 4% | 10% | 0% | 17% | 69% |
| TStringLibrary | Data, assets & I/O | 1 | 65% | 31% | 17% | 14% | 0% | 0% | 69% |
| TPackageLibrary | Data, assets & I/O | 1 | 95% | 29% | 21% | 7% | 0% | 71% | 0% |
| TDataLibrary | Data, assets & I/O | 1 | 75% | 45% | 33% | 12% | 0% | 0% | 55% |
| TObjLibManager | Data, assets & I/O | 0 | 45% | 28% | 0% | 28% | 0% | 72% | 0% |
| RF_DataManager | Data, assets & I/O | 0 | 75% | 32% | 26% | 6% | 0% | 68% | 0% |
| RFG_Locales routines (RFE_Export_RFC2 by address) | Data, assets & I/O | 0 | 75% | 100% | 100% | 0% | 0% | 0% | 0% |
| TDBObjManager | Data, assets & I/O | 0 | 30% | 25% | 0% | 25% | 0% | 75% | 0% |
| TRFCExporter | Data, assets & I/O | 0 | 38% | 28% | 0% | 28% | 0% | 72% | 0% |
| TDataStream | Data, assets & I/O | 0 | 60% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TStreamThread | Data, assets & I/O | 0 | 55% | 0% | 0% | 0% | 0% | 0% | 100% |
| TObjectDB | Data, assets & I/O | 0 | 72% | 100% | 0% | 100% | 0% | 0% | 0% |
| TUIDManager | Data, assets & I/O | 0 | 50% | 100% | 0% | 100% | 0% | 0% | 0% |
| TVXGIManager | Rendering & GPU | 29 | 97% | 42% | 32% | 10% | 0% | 58% | 0% |
| TMesh | Rendering & GPU | 25 | 91% | 41% | 34% | 7% | 0% | 52% | 7% |
| RFE_MeshTools | Rendering & GPU | 19 | 95% | 88% | 86% | 2% | 0% | 0% | 12% |
| RFE_MeshSubdivide | Rendering & GPU | 18 | 89% | 93% | 91% | 1% | 0% | 7% | 1% |
| FLX_GPUPrograms | Rendering & GPU | 16 | 84% | 7% | 2% | 5% | 0% | 53% | 40% |
| RF_Navigation | Rendering & GPU | 16 | 87% | 56% | 46% | 3% | 7% | 31% | 13% |
| RFP_Mipmaps_Kaiser | Rendering & GPU | 15 | 86% | 99% | 88% | 2% | 10% | 0% | 1% |
| FLX_Terrain_Texrender2 | Rendering & GPU | 14 | 81% | 16% | 13% | 3% | 0% | 84% | 0% |
| RF_Mesh | Rendering & GPU | 13 | 88% | 60% | 57% | 3% | 0% | 23% | 17% |
| TShaderBuilder | Rendering & GPU | 13 | 94% | 0% | 0% | 0% | 0% | 0% | 100% |
| RF_Textures | Rendering & GPU | 11 | 82% | 41% | 30% | 11% | 0% | 49% | 9% |
| RF_3DView | Rendering & GPU | 11 | 77% | 23% | 3% | 20% | 0% | 67% | 10% |
| Texture compression formats (DXT/LATC) | Rendering & GPU | 11 | 88% | 63% | 63% | 1% | 0% | 32% | 5% |
| TRenderer | Rendering & GPU | 10 | 75% | 33% | 10% | 20% | 3% | 62% | 5% |
| RF_MeshVoxels | Rendering & GPU | 10 | 93% | 81% | 80% | 0% | 0% | 15% | 5% |
| TNavCameraControl | Rendering & GPU | 8 | 73% | 30% | 29% | 1% | 0% | 61% | 9% |
| RFE_Image_Noise | Rendering & GPU | 7 | 90% | 100% | 100% | 0% | 0% | 0% | 0% |
| .rfi image codec | Rendering & GPU | 6 | 78% | 55% | 51% | 5% | 0% | 0% | 45% |
| Pixel-format converters (8-bit and float) | Rendering & GPU | 6 | 95% | 56% | 46% | 10% | 0% | 0% | 44% |
| RF_Polyhedra | Rendering & GPU | 5 | 95% | 50% | 46% | 4% | 0% | 50% | 0% |
| TShadowMapDirect | Rendering & GPU | 5 | 85% | 42% | 25% | 14% | 3% | 58% | 0% |
| TShadowCubeMap | Rendering & GPU | 5 | 85% | 36% | 15% | 16% | 5% | 64% | 0% |
| RF_Materials | Rendering & GPU | 5 | 66% | 21% | 19% | 2% | 0% | 79% | 0% |
| TTexture | Rendering & GPU | 5 | 75% | 72% | 31% | 40% | 0% | 19% | 10% |
| TLightSource | Rendering & GPU | 4 | 76% | 11% | 7% | 4% | 0% | 78% | 11% |
| GL immediate-draw helpers & built-in shaders | Rendering & GPU | 4 | 63% | 100% | 0% | 100% | 0% | 0% | 0% |
| TGeometry | Rendering & GPU | 4 | 55% | 29% | 0% | 29% | 0% | 52% | 19% |
| RF_GLGeometry | Rendering & GPU | 4 | 83% | 93% | 10% | 27% | 56% | 0% | 7% |
| Default material shader factory | Rendering & GPU | 4 | 62% | 8% | 6% | 2% | 0% | 89% | 4% |
| TTerrainMesh | Rendering & GPU | 3 | 70% | 51% | 24% | 18% | 10% | 49% | 0% |
| TGLGeometry | Rendering & GPU | 3 | 85% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TLightAlias | Rendering & GPU | 3 | 74% | 33% | 28% | 5% | 0% | 48% | 19% |
| RF_Material_Manager | Rendering & GPU | 3 | 62% | 50% | 8% | 41% | 0% | 44% | 7% |
| FLX_IconRender | Rendering & GPU | 3 | 50% | 4% | 0% | 4% | 0% | 0% | 96% |
| FLX_PostProcess | Rendering & GPU | 2 | 86% | 33% | 33% | 0% | 0% | 60% | 7% |
| Shading-mask GLSL snippets | Rendering & GPU | 2 | 75% | 16% | 12% | 5% | 0% | 82% | 2% |
| TPostProcessor | Rendering & GPU | 2 | 87% | 22% | 0% | 22% | 0% | 78% | 0% |
| RF_Renderer | Rendering & GPU | 1 | 54% | 13% | 0% | 13% | 0% | 0% | 87% |
| Screen-quad GLSL snippets | Rendering & GPU | 1 | 20% | 3% | 0% | 3% | 0% | 97% | 0% |
| RF_Splines | Rendering & GPU | 1 | 86% | 58% | 53% | 6% | 0% | 42% | 0% |
| RF_ShadowMap | Rendering & GPU | 1 | 62% | 19% | 0% | 19% | 0% | 70% | 11% |
| Texture pixel-format names | Rendering & GPU | 1 | 85% | 100% | 88% | 12% | 0% | 0% | 0% |
| T3DView | Rendering & GPU | 1 | 80% | 0% | 0% | 0% | 0% | 93% | 7% |
| TRendererShaderFactory | Rendering & GPU | 1 | 62% | 0% | 0% | 0% | 0% | 89% | 11% |
| TVoxelLight | Rendering & GPU | 1 | 71% | 61% | 61% | 0% | 0% | 39% | 0% |
| TMaterialManager3 | Rendering & GPU | 1 | 70% | 0% | 0% | 0% | 0% | 0% | 100% |
| TMaterial | Rendering & GPU | 1 | 70% | 0% | 0% | 0% | 0% | 55% | 45% |
| TShadowCubeTex | Rendering & GPU | 0 | 70% | 0% | 0% | 0% | 0% | 53% | 47% |
| TTextureManager | Rendering & GPU | 0 | 80% | 0% | 0% | 0% | 0% | 100% | 0% |
| TSplineRender | Rendering & GPU | 0 | 85% | 100% | 100% | 0% | 0% | 0% | 0% |
| TSubObject | Rendering & GPU | 0 | 80% | 0% | 0% | 0% | 0% | 100% | 0% |
| TConvexVol | Rendering & GPU | 0 | 55% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TQuadFBO | Rendering & GPU | 0 | 70% | 0% | 0% | 0% | 0% | 100% | 0% |
| TCamera | Rendering & GPU | 0 | 80% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TTexLoaderThread | Rendering & GPU | 0 | 20% | 0% | 0% | 0% | 0% | 100% | 0% |
| TVoxeliser | Rendering & GPU | 0 | 85% | 0% | 0% | 0% | 0% | 0% | 100% |
| TShaderLibrary | Rendering & GPU | 0 | 80% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TAbstractRenderer | Rendering & GPU | 0 | 70% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TAnimNoise | Rendering & GPU | 0 | 80% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TFrameBuffer | Rendering & GPU | 0 | 85% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TShadowMap | Rendering & GPU | 0 | 70% | 100% | 0% | 0% | 100% | 0% | 0% |
| TShaderFactory | Rendering & GPU | 0 | 90% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TGIEmissive | Rendering & GPU | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| RF_SceneNodes | World, levels & terrain | 24 | 92% | 25% | 22% | 3% | 0% | 65% | 10% |
| TStaticTerrain | World, levels & terrain | 20 | 82% | 45% | 36% | 5% | 5% | 52% | 3% |
| RFE_GridEnvs | World, levels & terrain | 15 | 81% | 56% | 50% | 0% | 5% | 44% | 0% |
| TGridEnvManager | World, levels & terrain | 15 | 83% | 86% | 76% | 3% | 7% | 14% | 0% |
| TkdTree | World, levels & terrain | 14 | 85% | 86% | 75% | 1% | 10% | 12% | 2% |
| TTerrainGrass | World, levels & terrain | 12 | 83% | 66% | 60% | 4% | 2% | 34% | 0% |
| TSceneEnvironment | World, levels & terrain | 10 | 94% | 81% | 81% | 0% | 0% | 19% | 0% |
| TRBDDoor | World, levels & terrain | 9 | 91% | 13% | 12% | 0% | 0% | 65% | 23% |
| RF_Terrain | World, levels & terrain | 7 | 71% | 52% | 50% | 1% | 0% | 9% | 40% |
| RF_Terrain_Materials | World, levels & terrain | 6 | 90% | 30% | 30% | 0% | 0% | 52% | 18% |
| RF_Dynamics_Game | World, levels & terrain | 5 | 93% | 57% | 56% | 1% | 0% | 35% | 8% |
| TTerrainManager | World, levels & terrain | 5 | 71% | 21% | 14% | 5% | 2% | 61% | 19% |
| T3DNode | World, levels & terrain | 5 | 48% | 43% | 24% | 19% | 0% | 25% | 32% |
| TRBRotator | World, levels & terrain | 4 | 88% | 86% | 81% | 5% | 0% | 14% | 0% |
| TGridEnvBuilder | World, levels & terrain | 4 | 69% | 62% | 41% | 21% | 0% | 11% | 26% |
| TScene | World, levels & terrain | 3 | 72% | 38% | 10% | 28% | 0% | 22% | 40% |
| RF_kdTrees | World, levels & terrain | 3 | 50% | 23% | 11% | 11% | 0% | 77% | 0% |
| SG_Teleporters | World, levels & terrain | 2 | 80% | 45% | 42% | 3% | 0% | 55% | 0% |
| Scene content helpers (unit before RF_Dynamics) | World, levels & terrain | 2 | 34% | 15% | 8% | 7% | 0% | 41% | 44% |
| RF_SceneEditor | World, levels & terrain | 2 | 48% | 24% | 16% | 9% | 0% | 0% | 76% |
| TRBPushSurface | World, levels & terrain | 2 | 98% | 97% | 95% | 2% | 0% | 3% | 0% |
| TTErrainkdTree | World, levels & terrain | 1 | 80% | 0% | 0% | 0% | 0% | 0% | 100% |
| TSceneTaskInt | World, levels & terrain | 1 | 78% | 0% | 0% | 0% | 0% | 0% | 100% |
| TSector | World, levels & terrain | 1 | 45% | 0% | 0% | 0% | 0% | 30% | 70% |
| RF_Environment | World, levels & terrain | 1 | 90% | 25% | 25% | 0% | 0% | 0% | 75% |
| Scene node despawn and viewport step (TSceneTaskInt by address) | World, levels & terrain | 1 | 30% | 0% | 0% | 0% | 0% | 0% | 100% |
| RF_Terrain_Static | World, levels & terrain | 0 | 50% | 8% | 0% | 8% | 0% | 0% | 92% |
| TTerrainBrushMgr | World, levels & terrain | 0 | 90% | 100% | 54% | 46% | 0% | 0% | 0% |
| RF_Terrain_Vegetation2 | World, levels & terrain | 0 | 90% | 100% | 100% | 0% | 0% | 0% | 0% |
| TTerrainPlugin | World, levels & terrain | 0 | 85% | 0% | 0% | 0% | 0% | 0% | 100% |
| TScenePlugin | World, levels & terrain | 0 | 90% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TMotileDynamics | Physics, bodies & cloth | 121 | 97% | 15% | 15% | 1% | 0% | 80% | 4% |
| TVCCloth | Physics, bodies & cloth | 30 | 96% | 23% | 20% | 2% | 1% | 63% | 14% |
| TSimSector | Physics, bodies & cloth | 28 | 94% | 18% | 17% | 1% | 0% | 82% | 1% |
| TTetraRBD | Physics, bodies & cloth | 15 | 80% | 41% | 40% | 1% | 0% | 52% | 7% |
| RF_Dynamics_Volumes | Physics, bodies & cloth | 14 | 90% | 37% | 37% | 0% | 0% | 45% | 18% |
| RF_Intersection | Physics, bodies & cloth | 14 | 93% | 64% | 61% | 3% | 0% | 2% | 34% |
| RF_Dynamics | Physics, bodies & cloth | 9 | 76% | 11% | 10% | 1% | 0% | 23% | 66% |
| Rigid-body shape builders (RF_GUI_Controls_Ex by address) | Physics, bodies & cloth | 8 | 85% | 39% | 37% | 2% | 0% | 0% | 61% |
| TSimulation | Physics, bodies & cloth | 2 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TVoxelMap | Physics, bodies & cloth | 0 | 95% | 100% | 51% | 49% | 0% | 0% | 0% |
| RFE_Dynamics_Cloth | Physics, bodies & cloth | 0 | 80% | 0% | 0% | 0% | 0% | 100% | 0% |
| TPhysicsController | Physics, bodies & cloth | 0 | 80% | 45% | 0% | 45% | 0% | 42% | 12% |
| TSimObject | Physics, bodies & cloth | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TMotilePart | Characters, motion & animation | 53 | 86% | 62% | 62% | 1% | 0% | 32% | 6% |
| SG_CharacterGen | Characters, motion & animation | 43 | 85% | 20% | 18% | 2% | 0% | 68% | 12% |
| RF_Skinning | Characters, motion & animation | 25 | 91% | 49% | 44% | 5% | 0% | 49% | 2% |
| SG_Character_Data | Characters, motion & animation | 20 | 97% | 61% | 60% | 1% | 0% | 11% | 28% |
| RF_Morphing | Characters, motion & animation | 16 | 80% | 14% | 14% | 0% | 0% | 47% | 39% |
| FLX_Character_Textures | Characters, motion & animation | 8 | 83% | 17% | 14% | 3% | 0% | 81% | 2% |
| TSkinController2 | Characters, motion & animation | 8 | 81% | 39% | 6% | 34% | 0% | 47% | 14% |
| FLX_Character_Motions | Characters, motion & animation | 6 | 80% | 41% | 40% | 1% | 0% | 26% | 33% |
| Equipment effects and focus-item slot (RFE_Effects_Electricity by address) | Characters, motion & animation | 3 | 93% | 34% | 34% | 0% | 0% | 48% | 18% |
| FLXG_Character_Races | Characters, motion & animation | 3 | 68% | 85% | 82% | 3% | 0% | 15% | 0% |
| Golem guardians and alarm zones (RFE_Effects_Electricity by address) | Characters, motion & animation | 3 | 80% | 61% | 61% | 0% | 0% | 39% | 0% |
| RF_Dynamics_Animation | Characters, motion & animation | 3 | 60% | 18% | 12% | 7% | 0% | 76% | 6% |
| TCharDecomp2 | Characters, motion & animation | 2 | 70% | 0% | 0% | 0% | 0% | 100% | 0% |
| TKFCCatRom3f | Characters, motion & animation | 1 | 85% | 0% | 0% | 0% | 0% | 100% | 0% |
| TMotionController | Characters, motion & animation | 1 | 90% | 100% | 0% | 100% | 0% | 0% | 0% |
| TSGCharacter | Characters, motion & animation | 1 | 50% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| SG_CharacterMorph | Characters, motion & animation | 1 | 60% | 100% | 71% | 12% | 18% | 0% | 0% |
| RF_KFControllers | Characters, motion & animation | 1 | 80% | 73% | 73% | 0% | 0% | 0% | 27% |
| TCharacterFactory | Characters, motion & animation | 0 | 38% | 100% | 0% | 100% | 0% | 0% | 0% |
| TKFCLinear3f | Characters, motion & animation | 0 | 80% | 88% | 0% | 0% | 88% | 0% | 12% |
| TMorphController | Characters, motion & animation | 0 | 40% | 100% | 0% | 100% | 0% | 0% | 0% |
| TMotileBehaviour | Characters, motion & animation | 0 | 40% | 15% | 0% | 15% | 0% | 11% | 74% |
| TSkinningManager2 | Characters, motion & animation | 0 | 90% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| TWeaponBlood | Combat, weapons & damage | 2 | 85% | 20% | 20% | 0% | 0% | 80% | 0% |
| RFE_Effects_Blood | Combat, weapons & damage | 1 | 80% | 41% | 32% | 9% | 0% | 59% | 0% |
| TBloodSprays | Combat, weapons & damage | 0 | 85% | 100% | 100% | 0% | 0% | 0% | 0% |
| TGUIItemContainer | Items, materials & crafting | 26 | 90% | 34% | 33% | 0% | 0% | 53% | 13% |
| FLX_ItemFactory | Items, materials & crafting | 24 | 96% | 81% | 79% | 3% | 0% | 19% | 0% |
| Item description phrases | Items, materials & crafting | 16 | 86% | 69% | 64% | 2% | 3% | 29% | 2% |
| SG_ItemGUI | Items, materials & crafting | 15 | 84% | 32% | 26% | 2% | 4% | 50% | 18% |
| Character equipment & inventory routines (TCharacterView by address) | Items, materials & crafting | 11 | 79% | 3% | 3% | 0% | 0% | 83% | 14% |
| FLX_Materials_Procedural | Items, materials & crafting | 10 | 82% | 19% | 17% | 2% | 0% | 53% | 28% |
| TProcMaterial | Items, materials & crafting | 6 | 79% | 0% | 0% | 0% | 0% | 91% | 9% |
| Item variant & condition randomisers (FLX_Character_Textures by address) | Items, materials & crafting | 5 | 84% | 55% | 54% | 1% | 0% | 45% | 0% |
| TItemGUIManager | Items, materials & crafting | 4 | 62% | 25% | 14% | 7% | 4% | 48% | 27% |
| TArsenalView | Items, materials & crafting | 3 | 80% | 32% | 32% | 0% | 0% | 32% | 36% |
| TInventory | Items, materials & crafting | 3 | 80% | 18% | 18% | 0% | 0% | 79% | 4% |
| TCharLoot | Items, materials & crafting | 1 | 65% | 78% | 78% | 0% | 0% | 22% | 0% |
| TItemRenderer | Items, materials & crafting | 1 | 50% | 100% | 0% | 100% | 0% | 0% | 0% |
| TPopUpDesc | Items, materials & crafting | 1 | 85% | 68% | 68% | 0% | 0% | 32% | 0% |
| TCopperSatin | Items, materials & crafting | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TGemstone | Items, materials & crafting | 1 | 55% | 0% | 0% | 0% | 0% | 100% | 0% |
| TResin | Items, materials & crafting | 1 | 55% | 0% | 0% | 0% | 0% | 100% | 0% |
| TCopperAlloy | Items, materials & crafting | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TSteelPainted | Items, materials & crafting | 1 | 65% | 0% | 0% | 0% | 0% | 100% | 0% |
| TSteelChain | Items, materials & crafting | 1 | 55% | 0% | 0% | 0% | 0% | 100% | 0% |
| TIronGen | Items, materials & crafting | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TSteelGen | Items, materials & crafting | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| Palette colour helpers (unit FLX_VoxelGI by address) | Items, materials & crafting | 1 | 72% | 100% | 86% | 14% | 0% | 0% | 0% |
| TSteelBlade | Items, materials & crafting | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TSteelPlate2 | Items, materials & crafting | 1 | 65% | 0% | 0% | 0% | 0% | 100% | 0% |
| TPlanksPainted | Items, materials & crafting | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TClothPadded1 | Items, materials & crafting | 1 | 62% | 0% | 0% | 0% | 0% | 100% | 0% |
| TClothPadded2 | Items, materials & crafting | 1 | 62% | 0% | 0% | 0% | 0% | 100% | 0% |
| TClothSilk | Items, materials & crafting | 1 | 50% | 0% | 0% | 0% | 0% | 100% | 0% |
| TClothVelvet | Items, materials & crafting | 1 | 52% | 0% | 0% | 0% | 0% | 100% | 0% |
| TClothBasic | Items, materials & crafting | 1 | 52% | 0% | 0% | 0% | 0% | 100% | 0% |
| TClothFine | Items, materials & crafting | 0 | 62% | 0% | 0% | 0% | 0% | 100% | 0% |
| TWoodFineA | Items, materials & crafting | 0 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TWoodRoughA | Items, materials & crafting | 0 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TLeatherA01 | Items, materials & crafting | 0 | 58% | 0% | 0% | 0% | 0% | 100% | 0% |
| TLeatherA02 | Items, materials & crafting | 0 | 58% | 0% | 0% | 0% | 0% | 100% | 0% |
| TSuedeA01 | Items, materials & crafting | 0 | 58% | 0% | 0% | 0% | 0% | 100% | 0% |
| TWiredGripA01 | Items, materials & crafting | 0 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TPopUpContainer | Items, materials & crafting | 0 | 85% | 46% | 46% | 0% | 0% | 54% | 0% |
| TArsenalContFrame | Items, materials & crafting | 0 | 0% | 0% | 0% | 0% | 0% | 0% | 100% |
| TMotileAI | AI & behaviour | 60 | 99% | 35% | 31% | 0% | 3% | 58% | 7% |
| SG_PlayerControls | AI & behaviour | 47 | 99% | 48% | 46% | 1% | 0% | 50% | 2% |
| TPathFinder | AI & behaviour | 32 | 91% | 41% | 41% | 0% | 0% | 52% | 7% |
| TPlayerBehaviour | AI & behaviour | 17 | 99% | 55% | 52% | 3% | 0% | 45% | 0% |
| TCombatant | AI & behaviour | 17 | 97% | 66% | 58% | 0% | 8% | 28% | 6% |
| TAISearchActivity | AI & behaviour | 16 | 90% | 42% | 38% | 4% | 0% | 57% | 1% |
| Character roles & relations (script functions) | AI & behaviour | 11 | 93% | 39% | 36% | 3% | 0% | 55% | 5% |
| SG_CharacterAI | AI & behaviour | 5 | 88% | 11% | 9% | 3% | 0% | 89% | 0% |
| TCharViewBehaviour | AI & behaviour | 0 | 70% | 0% | 0% | 0% | 0% | 100% | 0% |
| TObjectEntity | AI & behaviour | 0 | 70% | 0% | 0% | 0% | 0% | 0% | 100% |
| FLXG_Pathfinding | AI & behaviour | 0 | 70% | 8% | 0% | 8% | 0% | 92% | 0% |
| TAbstractPathFinder | AI & behaviour | 0 | 85% | 12% | 1% | 3% | 8% | 0% | 88% |
| TPlayerControl | AI & behaviour | 0 | 50% | 100% | 100% | 0% | 0% | 0% | 0% |
| FG_AI_Behaviours | AI & behaviour | 0 | 43% | 100% | 39% | 61% | 0% | 0% | 0% |
| FLX_Entities | AI & behaviour | 0 | 90% | 0% | 0% | 0% | 0% | 100% | 0% |
| TEntity | AI & behaviour | 0 | 40% | 0% | 0% | 0% | 0% | 33% | 67% |
| TAIActivity | AI & behaviour | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TThaumTree | Thaumaturgy & effects | 38 | 91% | 94% | 91% | 0% | 3% | 5% | 1% |
| SG_Thaum_Mind | Thaumaturgy & effects | 21 | 96% | 49% | 49% | 0% | 0% | 10% | 41% |
| TParticleSystem | Thaumaturgy & effects | 19 | 89% | 66% | 64% | 2% | 0% | 25% | 9% |
| TForceBlastEffect | Thaumaturgy & effects | 17 | 92% | 65% | 63% | 2% | 0% | 35% | 0% |
| TForceTorrentEffect | Thaumaturgy & effects | 15 | 89% | 24% | 22% | 1% | 0% | 72% | 5% |
| TTendrilEmitter | Thaumaturgy & effects | 15 | 90% | 37% | 36% | 2% | 0% | 62% | 0% |
| SG_Thaum_Force | Thaumaturgy & effects | 14 | 90% | 69% | 60% | 9% | 0% | 28% | 3% |
| SG_Thaum_DIsp | Thaumaturgy & effects | 12 | 86% | 62% | 60% | 2% | 0% | 32% | 5% |
| TForceBarricadeEffect | Thaumaturgy & effects | 12 | 86% | 13% | 13% | 0% | 0% | 87% | 0% |
| TTendrilEffect | Thaumaturgy & effects | 10 | 86% | 24% | 24% | 0% | 0% | 76% | 0% |
| TFireEffectRow | Thaumaturgy & effects | 8 | 82% | 48% | 47% | 1% | 0% | 52% | 0% |
| TForceShellEffect | Thaumaturgy & effects | 7 | 84% | 33% | 33% | 0% | 0% | 67% | 0% |
| Fire particle presets | Thaumaturgy & effects | 7 | 94% | 95% | 85% | 8% | 2% | 5% | 0% |
| TDomainWheel | Thaumaturgy & effects | 6 | 88% | 100% | 100% | 0% | 0% | 0% | 0% |
| RFE_Effects_Electricity | Thaumaturgy & effects | 6 | 82% | 83% | 81% | 2% | 0% | 17% | 0% |
| TShellEffect | Thaumaturgy & effects | 5 | 86% | 28% | 28% | 0% | 0% | 52% | 20% |
| TScanEffect | Thaumaturgy & effects | 5 | 80% | 45% | 45% | 0% | 0% | 34% | 21% |
| TForceBarrierEffect | Thaumaturgy & effects | 5 | 88% | 59% | 59% | 0% | 0% | 41% | 0% |
| TForceDomeEffect | Thaumaturgy & effects | 4 | 85% | 73% | 73% | 0% | 0% | 27% | 0% |
| TBubbleEffect | Thaumaturgy & effects | 4 | 82% | 58% | 54% | 4% | 0% | 42% | 0% |
| TForceWaveEffect | Thaumaturgy & effects | 4 | 83% | 0% | 0% | 0% | 0% | 100% | 0% |
| TPwrForceKinesis_S | Thaumaturgy & effects | 4 | 78% | 16% | 16% | 0% | 0% | 84% | 0% |
| SG_Dynamics_Effects | Thaumaturgy & effects | 3 | 86% | 45% | 40% | 5% | 0% | 55% | 0% |
| SG_Thaum_Energy | Thaumaturgy & effects | 3 | 82% | 94% | 94% | 0% | 0% | 6% | 0% |
| RF_Env_Effects | Thaumaturgy & effects | 3 | 78% | 40% | 38% | 2% | 0% | 60% | 0% |
| TForceBubbleEffect | Thaumaturgy & effects | 3 | 90% | 100% | 100% | 0% | 0% | 0% | 0% |
| SG_Thaumaturgy | Thaumaturgy & effects | 3 | 67% | 39% | 32% | 7% | 0% | 51% | 11% |
| TPwrDispSummonDemon_I | Thaumaturgy & effects | 2 | 86% | 69% | 69% | 0% | 0% | 31% | 0% |
| TPwrForceStrikes_C | Thaumaturgy & effects | 2 | 62% | 9% | 9% | 0% | 0% | 89% | 2% |
| TPwrDispLongDoor_I | Thaumaturgy & effects | 2 | 78% | 0% | 0% | 0% | 0% | 100% | 0% |
| TPwrForceThrow_I | Thaumaturgy & effects | 2 | 67% | 17% | 17% | 0% | 0% | 73% | 9% |
| TBarricadeEntity | Thaumaturgy & effects | 2 | 96% | 12% | 12% | 0% | 0% | 88% | 0% |
| TBubbleEmissive@SG_Teleporters | Thaumaturgy & effects | 2 | 70% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrControlMind_C | Thaumaturgy & effects | 1 | 87% | 56% | 56% | 0% | 0% | 44% | 0% |
| TPwrSpiritForm | Thaumaturgy & effects | 1 | 89% | 44% | 44% | 0% | 0% | 56% | 0% |
| TBubbleEmissive@SG_Thaum_Force | Thaumaturgy & effects | 1 | 65% | 100% | 100% | 0% | 0% | 0% | 0% |
| TBarricadeEmissive | Thaumaturgy & effects | 1 | 88% | 100% | 100% | 0% | 0% | 0% | 0% |
| TBarrierEmissive | Thaumaturgy & effects | 1 | 78% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrDispVanish_I | Thaumaturgy & effects | 1 | 82% | 14% | 14% | 0% | 0% | 86% | 0% |
| TBlastEntity | Thaumaturgy & effects | 1 | 88% | 13% | 13% | 0% | 0% | 87% | 0% |
| FLX_EffectRender | Thaumaturgy & effects | 1 | 40% | 30% | 22% | 9% | 0% | 70% | 0% |
| TCuboidEmissive | Thaumaturgy & effects | 1 | 85% | 100% | 100% | 0% | 0% | 0% | 0% |
| TTendrilEmissive | Thaumaturgy & effects | 1 | 78% | 100% | 100% | 0% | 0% | 0% | 0% |
| TVanishEffect | Thaumaturgy & effects | 1 | 91% | 80% | 80% | 0% | 0% | 20% | 0% |
| TTendrilGeometry | Thaumaturgy & effects | 1 | 55% | 26% | 26% | 0% | 0% | 74% | 0% |
| TStrikesGeometry | Thaumaturgy & effects | 1 | 70% | 42% | 42% | 0% | 0% | 58% | 0% |
| TStrikesEmissive | Thaumaturgy & effects | 1 | 82% | 100% | 100% | 0% | 0% | 0% | 0% |
| TForceBlastEmissive | Thaumaturgy & effects | 1 | 96% | 100% | 100% | 0% | 0% | 0% | 0% |
| TForceBlastGeometry | Thaumaturgy & effects | 1 | 87% | 0% | 0% | 0% | 0% | 100% | 0% |
| TForceTorrentGeometry | Thaumaturgy & effects | 1 | 85% | 27% | 27% | 0% | 0% | 73% | 0% |
| TForceTorrentEmissive | Thaumaturgy & effects | 1 | 96% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrEnergyShock_I | Thaumaturgy & effects | 1 | 86% | 0% | 0% | 0% | 0% | 100% | 0% |
| TForceWaveEntity | Thaumaturgy & effects | 1 | 84% | 21% | 21% | 0% | 0% | 79% | 0% |
| FLX_FX_Fire | Thaumaturgy & effects | 1 | 55% | 60% | 53% | 7% | 0% | 40% | 0% |
| TPortalEffect | Thaumaturgy & effects | 1 | 95% | 100% | 100% | 0% | 0% | 0% | 0% |
| TBarricadeGeometry | Thaumaturgy & effects | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TPwrLoyalty_E | Thaumaturgy & effects | 1 | 90% | 18% | 18% | 0% | 0% | 61% | 21% |
| TPwrForceShell_C | Thaumaturgy & effects | 1 | 64% | 93% | 93% | 0% | 0% | 0% | 7% |
| TPwrCommandDead_C | Thaumaturgy & effects | 1 | 92% | 81% | 81% | 0% | 0% | 0% | 19% |
| TPwrLoyalty_C | Thaumaturgy & effects | 1 | 90% | 77% | 77% | 0% | 0% | 7% | 16% |
| TBubbleGeometry | Thaumaturgy & effects | 1 | 60% | 100% | 100% | 0% | 0% | 0% | 0% |
| TForceBarrierGeometry | Thaumaturgy & effects | 0 | 60% | 100% | 100% | 0% | 0% | 0% | 0% |
| TRainEffect | Thaumaturgy & effects | 0 | 95% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrConstantDisp_C | Thaumaturgy & effects | 0 | 95% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrMindMask_I | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TForceGeometry | Thaumaturgy & effects | 0 | 80% | 100% | 97% | 2% | 1% | 0% | 0% |
| TPwrShockWeapons_C | Thaumaturgy & effects | 0 | 95% | 100% | 100% | 0% | 0% | 0% | 0% |
| TActivePower | Thaumaturgy & effects | 0 | 95% | 51% | 50% | 1% | 0% | 47% | 2% |
| TPwrMindSense_C | Thaumaturgy & effects | 0 | 90% | 0% | 0% | 0% | 0% | 76% | 24% |
| TPwrCommandDead_E | Thaumaturgy & effects | 0 | 87% | 49% | 49% | 0% | 0% | 51% | 0% |
| TPwrPosesses_E | Thaumaturgy & effects | 0 | 95% | 100% | 100% | 0% | 0% | 0% | 0% |
| RF_ParticleSystems2 | Thaumaturgy & effects | 0 | 90% | 100% | 92% | 8% | 0% | 0% | 0% |
| TPwrForceBubble_C | Thaumaturgy & effects | 0 | 95% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrMindMask_C | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrForceBarricade_C | Thaumaturgy & effects | 0 | 95% | 0% | 0% | 0% | 0% | 100% | 0% |
| TPwrMindSense_I | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrConfusion | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrEnrage | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrSuspect_E | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TPwrTrust_E | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TAbstractForceBarrier | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TEffectGeometry@FLX_EffectRender | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TEffectGeometry@SG_Thaum_DIsp | Thaumaturgy & effects | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| TAudioEnvironment | Audio | 13 | 98% | 82% | 81% | 1% | 0% | 16% | 2% |
| OpenAL loader | Audio | 4 | 72% | 79% | 0% | 79% | 0% | 21% | 0% |
| RFA_Main | Audio | 1 | 72% | 90% | 29% | 61% | 0% | 10% | 0% |
| TDummyAudioEnv | Audio | 0 | 70% engine | 100% | 0% | 100% | 0% | 0% | 0% |
| RFG_Dialogue | Script, dialogue & story | 18 | 91% | 94% | 92% | 2% | 0% | 0% | 6% |
| FG_AIFunctions | Script, dialogue & story | 11 | 86% | 88% | 86% | 2% | 0% | 12% | 0% |
| RFG_Scripts | Script, dialogue & story | 6 | 86% | 100% | 99% | 1% | 0% | 0% | 0% |
| SG_GameFunctions | Script, dialogue & story | 3 | 82% | 41% | 38% | 3% | 0% | 40% | 19% |
| TRTDialogue | Script, dialogue & story | 3 | 85% | 48% | 43% | 5% | 0% | 52% | 0% |
| TDialogueChoices | Script, dialogue & story | 2 | 80% | 100% | 98% | 2% | 0% | 0% | 0% |
| TScriptRole | Script, dialogue & story | 1 | 82% | 52% | 52% | 0% | 0% | 48% | 0% |
| TGameTips | Script, dialogue & story | 0 | 85% | 48% | 48% | 0% | 0% | 52% | 0% |
| TJournal | Script, dialogue & story | 0 | 78% | 38% | 38% | 0% | 0% | 62% | 0% |
| TScriptActor | Script, dialogue & story | 0 | 80% | 0% | 0% | 0% | 0% | 100% | 0% |
| TScriptClass | Script, dialogue & story | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| XA_Arena | Game modes, arena & session | 71 | 96% | 61% | 58% | 2% | 0% | 36% | 4% |
| XA_SaveGames | Game modes, arena & session | 27 | 96% | 38% | 35% | 3% | 0% | 56% | 6% |
| SG_GameMain | Game modes, arena & session | 20 | 87% | 57% | 56% | 2% | 0% | 36% | 7% |
| TMatchBoard | Game modes, arena & session | 17 | 90% | 8% | 7% | 1% | 0% | 57% | 34% |
| SG_Character_Create | Game modes, arena & session | 17 | 86% | 34% | 31% | 3% | 0% | 40% | 26% |
| TLoadoutForm | Game modes, arena & session | 7 | 85% | 12% | 12% | 0% | 0% | 87% | 1% |
| XA_ExperienceSys | Game modes, arena & session | 6 | 85% | 56% | 53% | 3% | 0% | 28% | 16% |
| TDivCardForm | Game modes, arena & session | 5 | 91% | 25% | 25% | 0% | 0% | 74% | 2% |
| SG_TradeSystem | Game modes, arena & session | 4 | 80% | 75% | 72% | 3% | 0% | 24% | 2% |
| TCharCreation | Game modes, arena & session | 3 | 62% | 1% | 0% | 1% | 0% | 71% | 28% |
| XA_Arena save and rank helpers (TMatchBoard by address) | Game modes, arena & session | 3 | 86% | 43% | 43% | 0% | 0% | 0% | 57% |
| TClassSelect | Game modes, arena & session | 3 | 80% | 28% | 28% | 0% | 0% | 72% | 0% |
| TShopWindow | Game modes, arena & session | 2 | 95% | 96% | 96% | 0% | 0% | 4% | 0% |
| SG_SaveWorld | Game modes, arena & session | 2 | 92% | 100% | 95% | 5% | 0% | 0% | 0% |
| SG_Prelude_Main | Game modes, arena & session | 2 | 80% | 72% | 68% | 4% | 0% | 28% | 0% |
| SG_EmblemGen | Game modes, arena & session | 2 | 55% | 19% | 10% | 9% | 0% | 10% | 71% |
| TEndSessionMenu | Game modes, arena & session | 1 | 80% | 0% | 0% | 0% | 0% | 100% | 0% |
| TCompanyCreate | Game modes, arena & session | 1 | 55% | 0% | 0% | 0% | 0% | 0% | 100% |
| Save-package section delete (SG_EmblemGen by address) | Game modes, arena & session | 0 | 85% | 100% | 100% | 0% | 0% | 0% | 0% |
| TSaveGameManager | Game modes, arena & session | 0 | 70% | 100% | 90% | 10% | 0% | 0% | 0% |
| TManageForm | Game modes, arena & session | 0 | 60% | 100% | 100% | 0% | 0% | 0% | 0% |
| TGUIDrawSkin | UI | 35 | 93% | 42% | 42% | 0% | 0% | 11% | 47% |
| TGSLines | UI | 24 | 89% | 13% | 13% | 0% | 0% | 60% | 26% |
| SG_GameGUI | UI | 22 | 90% | 44% | 26% | 1% | 16% | 46% | 10% |
| TGUIForm | UI | 20 | 96% | 13% | 13% | 1% | 0% | 46% | 41% |
| TSkillUI | UI | 18 | 88% | 63% | 63% | 0% | 0% | 20% | 17% |
| RF_GUI_Controls_Ex | UI | 16 | 87% | 75% | 12% | 1% | 62% | 25% | 0% |
| RF_HW_Config | UI | 15 | 93% | 67% | 59% | 5% | 2% | 32% | 1% |
| RF_GUI_Main | UI | 14 | 86% | 39% | 37% | 2% | 0% | 35% | 26% |
| TGUIItemView | UI | 13 | 80% | 0% | 0% | 0% | 0% | 0% | 100% |
| TTextBox | UI | 12 | 88% | 24% | 23% | 1% | 0% | 2% | 74% |
| TIconManager | UI | 10 | 85% | 44% | 42% | 1% | 1% | 0% | 56% |
| RF_GUI_Controls | UI | 8 | 80% | 14% | 13% | 1% | 0% | 71% | 15% |
| TCharacterView | UI | 8 | 76% | 8% | 8% | 0% | 0% | 74% | 18% |
| TGUITableView | UI | 7 | 90% | 6% | 1% | 4% | 0% | 52% | 42% |
| RF_GUI_Icons | UI | 7 | 90% | 84% | 74% | 1% | 9% | 9% | 7% |
| GUI support routines (unit before RF_GUI_Main) | UI | 7 | 87% | 84% | 83% | 1% | 0% | 0% | 16% |
| T3DViewport | UI | 6 | 66% | 6% | 0% | 4% | 2% | 77% | 17% |
| TMainMenu | UI | 6 | 80% | 68% | 68% | 0% | 0% | 27% | 5% |
| TCompactSkillUI | UI | 6 | 80% | 0% | 0% | 0% | 0% | 0% | 100% |
| RF_GUI_Text_Edit | UI | 5 | 85% | 75% | 73% | 2% | 0% | 13% | 12% |
| TSelectionUI | UI | 5 | 88% | 42% | 41% | 1% | 0% | 58% | 0% |
| TEmblemForm | UI | 4 | 90% | 100% | 99% | 1% | 0% | 0% | 0% |
| TSkillSelector | UI | 3 | 80% | 100% | 0% | 0% | 100% | 0% | 0% |
| TSplashScreen | UI | 3 | 74% | 100% | 93% | 7% | 0% | 0% | 0% |
| THairColPicker | UI | 2 | 70% | 54% | 54% | 0% | 0% | 46% | 0% |
| SG_GUI_Skills | UI | 2 | 79% | 90% | 84% | 5% | 0% | 0% | 10% |
| TGUIColSwatch | UI | 2 | 92% | 88% | 86% | 2% | 0% | 0% | 12% |
| RF_GUI_Classes | UI | 2 | 85% | 100% | 0% | 3% | 97% | 0% | 0% |
| TTonePicker | UI | 2 | 82% | 100% | 85% | 0% | 15% | 0% | 0% |
| XA_HelpSystem | UI | 2 | 82% | 66% | 60% | 5% | 0% | 6% | 29% |
| TTextRenderer | UI | 2 | 92% | 0% | 0% | 0% | 0% | 100% | 0% |
| TDropDownList | UI | 2 | 88% | 76% | 76% | 0% | 0% | 11% | 13% |
| TGUITask | UI | 2 | 85% | 0% | 0% | 0% | 0% | 74% | 26% |
| TTransitionScreen | UI | 2 | 82% | 13% | 11% | 1% | 0% | 86% | 2% |
| TGUIDrawSkinTex | UI | 1 | 60% | 0% | 0% | 0% | 0% | 100% | 0% |
| TReadKeyScreen | UI | 1 | 80% | 69% | 65% | 4% | 0% | 31% | 0% |
| THairEditForm | UI | 1 | 60% | 24% | 24% | 0% | 0% | 66% | 9% |
| TToolTip | UI | 1 | 80% | 40% | 40% | 0% | 0% | 60% | 0% |
| TCharEditForm | UI | 1 | 65% | 93% | 91% | 2% | 0% | 7% | 0% |
| TPickerForm | UI | 1 | 62% | 34% | 34% | 0% | 0% | 19% | 48% |
| TPhysiquePicker | UI | 1 | 55% | 53% | 53% | 0% | 0% | 47% | 0% |
| TCacheShaderScreen | UI | 1 | 70% | 27% | 24% | 3% | 0% | 73% | 0% |
| TPopupParchment | UI | 1 | 85% | 76% | 76% | 0% | 0% | 24% | 0% |
| RF_GUI_Taskbar | UI | 1 | 45% | 15% | 0% | 15% | 0% | 63% | 22% |
| TCharBaseForm | UI | 1 | 65% | 88% | 88% | 0% | 0% | 12% | 0% |
| SGD_GameObjEdit | UI | 1 | 35% | 32% | 0% | 32% | 0% | 68% | 0% |
| TSkillTip | UI | 1 | 35% | 100% | 0% | 0% | 100% | 0% | 0% |
| TColourPicker | UI | 1 | 60% | 100% | 0% | 0% | 100% | 0% | 0% |
| FLX_GUI_Graphics | UI | 1 | 70% | 14% | 0% | 14% | 0% | 86% | 0% |
| FLX_GUI_Skinned | UI | 0 | 55% | 5% | 0% | 5% | 0% | 95% | 0% |
| TTaskInterface | UI | 0 | 40% | 1% | 0% | 0% | 1% | 0% | 99% |
| RF_GUI_3DViewport | UI | 0 | 30% | 24% | 0% | 18% | 6% | 3% | 73% |
| THelpScreen | UI | 0 | 45% | 0% | 0% | 0% | 0% | 39% | 61% |
| TConfirmDismissDlg | UI | 0 | 80% | 81% | 81% | 0% | 0% | 0% | 19% |
| TConfirmDlg | UI | 0 | 80% | 81% | 81% | 0% | 0% | 0% | 19% |
| TResetToPortalDlg | UI | 0 | 85% | 81% | 81% | 0% | 0% | 0% | 19% |
| TConfirmDialogue | UI | 0 | 40% | 79% | 79% | 0% | 0% | 0% | 21% |
| THelpScreen2 | UI | 0 | 45% | 0% | 0% | 0% | 0% | 0% | 100% |
| TAbstractToolTip | UI | 0 | 45% | 0% | 0% | 0% | 0% | 0% | 100% |
| TMessageBox | UI | 0 | 75% | 50% | 50% | 0% | 0% | 0% | 50% |
| TCursor | UI | 0 | 90% | 100% | 100% | 0% | 0% | 0% | 0% |
| TStatusForm | UI | 0 | 30% | 0% | 0% | 0% | 0% | 100% | 0% |
| TBaseGUISkin | UI | 0 | 100% | 100% | 100% | 0% | 0% | 0% | 0% |
| Debug command registry (unit before RF_Dynamics) | Other, tools & editor | 0 | 55% | 100% | 82% | 18% | 0% | 0% | 0% |

## How it was made

- **Inventory.** Ghidra (through REA) lists every function. Virtual methods are attributed to classes through the Free Pascal VMTs and RTTI, helpers through the call graph, libraries by imports and strings, and the rest by unit and neighbours. Each function is counted once.
- **Placement.** Classes follow `research/engine-classes.md`, unless an override gives a reason. Unit routines follow their unit; anonymous clusters follow their strings and callers. Where the research notes show that some of an owner's functions serve another subsystem, those functions are split out by address.
- **Rating.** One agent per section described each component from the decompiled code. A second rated Exarust's code against it, and a third tried to refute every rating. A completeness critic then checked the whole map; its 39 findings were fixed and verified the same way, and a final critic re-checked the result.
- **Rules.** Coverage is the share of a component's code, by bytes, whose behaviour Exarust reproduces. Missing is below 5%, partial below 80%, reimplemented from 80%. "Provided by the engine" counts only behaviour that Bevy, wgpu, rapier or std supplies and Exarust actually uses. Each Exarust behaviour is credited to one component only. Editor-only tools, dormant handlers and unused RTL units are not applicable and are left out of the percentages.
- **The README image.** `docs/img/coverage-map-{dark,light}.png` are this map's coverage view, rendered by `context/tools/coverage_image.py` whenever the ratings change.
- **Where the data lives.** The inventory, the notes and the build scripts are research material and stay in the git-ignored `context/` (`context/re/component-map/`, `context/tools/sections/`, `context/tools/build_map_data.py`, this page from `context/tools/component_map_doc.py`).
- **The ledger.** One row per original function with a state, built by `context/tools/ledger/build_ledger.py` into the git-ignored `context/re/ledger/`. Final states: live, engine, unreachable, deviation (and not-applicable, left out of the count). Open states: partial, uncalled, missing, unaudited. Only byte sums reach this page.
