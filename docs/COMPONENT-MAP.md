# Component map: the original against Exarust

Every function of the retail `Exanima.exe` (Exanima 0.9.5, Free Pascal 3.2.2, Win64), placed in a component and a section, and each component rated for how much of its behaviour Exarust reproduces. The interactive treemaps are published at <https://claude.ai/artifact/GybAiwT9utFDbJTSaPLUkC>. This page is the summary that lives in the tree.

## Headline

- **8,783 functions**, 2,616 KB of code, in **445 components** across 16 sections.
- Weighted by code size, with code the retail game never runs left out (1,343 functions, 163 KB): **67.2% reimplemented** by Exarust's own code and **4.9% provided** by Bevy, wgpu, rapier or the Rust standard library.
- Weighted by function count (zero-byte imported names left out): 60.7% and 11.8%. The runtime's many tiny RTL routines make this figure lean towards the engine, which is why code size is the headline.
- The 24 combat-rule functions (52 KB of damage, armour and weapon-stat rules inside body, part, AI and item owners) are **45.4%** reproduced.

## By section

| section | functions | KB | components | reimplemented | by engine | largest missing |
|---|---:|---:|---:|---:|---:|---|
| Runtime & platform | 2,407 | 293 | 26 | 13% | 72% | Win32 clipboard image/icon glue |
| Core, math & threading | 282 | 37 | 11 | 80% | 1% |  |
| Data, assets & I/O | 440 | 91 | 26 | 71% | 0% | TStringLibrary, RF_DataManager, TRFCExporter |
| Rendering & GPU | 823 | 334 | 63 | 75% | 1% | FLX_IconRender, Screen-quad GLSL snippets, TTexLoaderThread |
| World, levels & terrain | 454 | 174 | 31 | 67% | 0% | Scene node despawn and viewport step (TSceneTaskInt by address) |
| Physics, bodies & cloth | 363 | 242 | 13 | 65% | 0% |  |
| Characters, motion & animation | 506 | 198 | 23 | 70% | 0% |  |
| Combat, weapons & damage | 14 | 3 | 3 | 31% | 0% | RFE_Effects_Blood, TBloodSprays |
| Items, materials & crafting | 341 | 140 | 40 | 67% | 0% | TCharLoot, TPopUpDesc, TPopUpContainer |
| AI & behaviour | 437 | 206 | 17 | 71% | 0% | FG_AI_Behaviours, FLX_Entities, TEntity |
| Thaumaturgy & effects | 697 | 296 | 83 | 69% | 0% |  |
| Audio | 59 | 18 | 4 | 84% | 0% |  |
| Script, dialogue & story | 184 | 46 | 11 | 76% | 0% | TJournal, TScriptActor |
| Game modes, arena & session | 646 | 193 | 23 | 71% | 0% | Save-package section delete (SG_EmblemGen by address), TSaveGameManager |
| UI | 959 | 312 | 63 | 76% | 0% | TReadKeyScreen, THairEditForm, TCharEditForm |
| Other, tools & editor | 171 | 33 | 8 | 55% | 0% |  |

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
| TCharCreation | Game modes, arena & session | 12 | 3 | 14% |
| FLX_IconRender | Rendering & GPU | 13 | 3 | 0% |
| Shading-mask GLSL snippets | Rendering & GPU | 6 | 2 | 10% |
| Scene content helpers (unit before RF_Dynamics) | World, levels & terrain | 11 | 2 | 10% |
| Win32 clipboard image/icon glue | Runtime & platform | 8 | 2 | 0% |
| Screen-quad GLSL snippets | Rendering & GPU | 7 | 1 | 0% |
| Steam API glue | Runtime & platform | 16 | 1 | 7% |
| TCharLoot | Items, materials & crafting | 7 | 1 | 0% |
| TReadKeyScreen | UI | 10 | 1 | 0% |
| TPopUpDesc | Items, materials & crafting | 9 | 1 | 0% |
| THairEditForm | UI | 4 | 1 | 0% |
| RFE_Effects_Blood | Combat, weapons & damage | 7 | 1 | 0% |
| TCharEditForm | UI | 3 | 1 | 0% |
| TPickerForm | UI | 4 | 1 | 0% |
| TPhysiquePicker | UI | 6 | 1 | 0% |
| TCacheShaderScreen | UI | 6 | 1 | 0% |
| TPopupParchment | UI | 6 | 1 | 0% |
| RF_GUI_Taskbar | UI | 6 | 1 | 0% |
| TStringLibrary | Data, assets & I/O | 7 | 1 | 0% |
| TCharBaseForm | UI | 3 | 1 | 0% |
| TCopperSatin | Items, materials & crafting | 1 | 1 | 5% |
| TGemstone | Items, materials & crafting | 1 | 1 | 5% |
| TResin | Items, materials & crafting | 1 | 1 | 5% |
| TCopperAlloy | Items, materials & crafting | 1 | 1 | 5% |
| TEndSessionMenu | Game modes, arena & session | 2 | 1 | 5% |

## Best covered components

| component | section | KB | coverage | Exarust |
|---|---|---:|---:|---|
| TMotileDynamics | Physics, bodies & cloth | 121 | 53% | exa_physics/src/body_feet.rs; exa_physics/src/body_gait.rs |
| XA_Arena | Game modes, arena & session | 71 | 81% | exa_game/src/arena_outfit.rs; exa_game/src/arena_outfit_data.rs |
| TMotileAI | AI & behaviour | 60 | 74% | exa_ai/src/chase.rs; exa_ai/src/route.rs |
| SG_CharacterGen | Characters, motion & animation | 43 | 75% | exa_character/src/layers.rs; exa_character/src/look.rs |
| TGUIDrawSkin | UI | 35 | 90% | exa_ui/src/skin.rs; exa_ui/src/gui_draw.rs |
| TMotilePart | Characters, motion & animation | 53 | 54% | exa_physics/src/part.rs; exa_physics/src/grip.rs |
| TPathFinder | AI & behaviour | 32 | 86% | exa_world/src/nav/path.rs; exa_world/src/nav/mover.rs |
| TThaumTree | Thaumaturgy & effects | 38 | 66% | exa_game/src/thaum/tree.rs; exa_game/src/power_screen.rs |
| TVCCloth | Physics, bodies & cloth | 30 | 84% | exa_physics/src/cloth.rs; exa_world/src/cloth.rs |
| SG_PlayerControls | AI & behaviour | 47 | 53% | exa_game/src/player_controls.rs; exa_game/src/idle_orbit.rs |
| TVXGIManager | Rendering & GPU | 29 | 79% | exa_render/src/gi.rs; exa_render/src/gi.wesl |
| FLX_ItemFactory | Items, materials & crafting | 24 | 89% | exa_formats/src/factory.rs; exa_formats/src/weapon_stats.rs |
| TGSLines | UI | 24 | 85% | exa_ui/src/skin.rs; exa_ui/src/line_skin.rs |
| TGUIItemContainer | Items, materials & crafting | 26 | 80% | exa_game/src/item_container.rs; exa_items/src/card.rs |
| TMesh | Rendering & GPU | 25 | 82% | exa_formats/src/rfc.rs; exa_assets/src/model.rs |
| RF_Skinning | Characters, motion & animation | 25 | 82% | exa_assets/src/skin.rs; exa_assets/src/model.rs |
| TSimSector | Physics, bodies & cloth | 28 | 70% | exa_physics/src/sector.rs; exa_physics/src/body_pairs.rs |
| RF_SceneNodes | World, levels & terrain | 24 | 80% | exa_world/src/scene_nodes.rs; exa_world/src/light_flood.rs |
| RFE_MeshTools | Rendering & GPU | 19 | 95% | exa_assets/src/mesh_weld.rs; exa_assets/src/mesh_edit.rs |
| SG_ObjectDB2 | Data, assets & I/O | 19 | 90% | exa_formats/src/objdb.rs; exa_formats/src/objdb/store.rs |

## Combat rules

| rule | inside | bytes | coverage |
|---|---|---:|---:|
| NPC combat state | TMotileAI | 18,453 | 5% |
| damage routine: hit point, speed/impact, zone factors, armour roll | TMotileDynamics | 13,157 | 71% |
| per-part weapon stat bytes of factory weapons | FLX_ItemFactory | 3,692 | 90% |
| armour layers from equipped apparel | SG_CharacterGen | 3,680 | 60% |
| weapon mesh assembly (feeds striking-part extent) | FLX_ItemFactory | 3,282 | 40% |
| combat positioning score (spacing/reach) | TCombatant | 3,152 | 85% |
| factory weapon spawn, striking-part extent | FLX_ItemFactory | 1,349 | 30% |
| builds the weapon stat record from the item weapon section | TMotilePart | 1,284 | 55% |
| grip/owner pass over the weapon stat record | TMotilePart | 1,244 | 80% |
| re-runs the grip pass for each held weapon | TMotilePart | 798 | 80% |
| apparel instance quality/condition | Item description phrases | 780 | 40% |
| factory item condition | Item description phrases | 532 | 60% |
| applies damage to a body | TMotileDynamics | 481 | 50% |
| attacker recoil after a hit | TMotileDynamics | 266 | 0% |
| factory item stats | FLX_ItemFactory | 247 | 55% |
| shield stat block | TMotilePart | 187 | 90% |
| weapon mass distribution | TMotilePart | 178 | 100% |
| weapon mass distribution | TMotilePart | 146 | 100% |
| sweet spot | TMotilePart | 141 | 0% |
| daze | TMotilePart | 138 | 80% |
| enter combat state | TMotileAI | 116 | 15% |
| blade length | TMotilePart | 79 | 60% |
| weapon mass distribution | TMotilePart | 66 | 100% |
| fetches the weapon stat record up the body chain | TMotilePart | 38 | 50% |

## How it was made

- **Inventory.** Ghidra (through REA) lists every function. Virtual methods are attributed to classes through the Free Pascal VMTs and RTTI, helpers through the call graph, libraries by imports and strings, and the rest by unit and neighbours. Each function is counted once.
- **Placement.** Classes follow `research/engine-classes.md`, unless an override gives a reason. Unit routines follow their unit; anonymous clusters follow their strings and callers. Where the research notes show that some of an owner's functions serve another subsystem, those functions are split out by address.
- **Rating.** One agent per section described each component from the decompiled code. A second rated Exarust's code against it, and a third tried to refute every rating. A completeness critic then checked the whole map; its 39 findings were fixed and verified the same way, and a final critic re-checked the result.
- **Rules.** Coverage is the share of a component's code, by bytes, whose behaviour Exarust reproduces. Missing is below 5%, partial below 80%, reimplemented from 80%. "Provided by the engine" counts only behaviour that Bevy, wgpu, rapier or std supplies and Exarust actually uses. Each Exarust behaviour is credited to one component only. Editor-only tools, dormant handlers and unused RTL units are not applicable and are left out of the percentages.
- **The README image.** `docs/img/coverage-map-{dark,light}.png` are this map's coverage view, rendered by `context/tools/coverage_image.py` whenever the ratings change.
- **Where the data lives.** The inventory, the notes and the build scripts are research material and stay in the git-ignored `context/` (`context/re/component-map/`, `context/tools/sections/`, `context/tools/build_map_data.py`, this page from `context/tools/component_map_doc.py`).
