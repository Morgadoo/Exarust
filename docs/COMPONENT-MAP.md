# Component map: the original against Exarust

Every function of the retail `Exanima.exe` (Exanima 0.9.5, Free Pascal 3.2.2, Win64), placed in a component and a section, and each component rated for how much of its behaviour Exarust reproduces. The interactive treemaps are published at <https://claude.ai/artifact/GybAiwT9utFDbJTSaPLUkC>. This page is the summary that lives in the tree.

## Headline

- **8,783 functions**, 2,616 KB of code, in **445 components** across 16 sections.
- Weighted by code size, with code the retail game never runs left out (1,343 functions, 163 KB): **40.4% reimplemented** by Exarust's own code and **5.7% provided** by Bevy, wgpu, rapier or the Rust standard library.
- Weighted by function count (zero-byte imported names left out): 35.9% and 12.5%. The runtime's many tiny RTL routines make this figure lean towards the engine, which is why code size is the headline.
- The 24 combat-rule functions (52 KB of damage, armour and weapon-stat rules inside body, part, AI and item owners) are **32.3%** reproduced.

## By section

| section | functions | KB | components | reimplemented | by engine | largest missing |
|---|---:|---:|---:|---:|---:|---|
| Runtime & platform | 2,407 | 293 | 26 | 4% | 72% | Steam Input action bindings, Win32 clipboard image/icon glue, Steam API glue |
| Core, math & threading | 282 | 37 | 11 | 59% | 1% |  |
| Data, assets & I/O | 440 | 91 | 26 | 56% | 0% | TStringLibrary, RF_DataManager, RFG_Locales routines (RFE_Export_RFC2 by address) |
| Rendering & GPU | 823 | 334 | 63 | 51% | 3% | FLX_IconRender, Shading-mask GLSL snippets, Screen-quad GLSL snippets |
| World, levels & terrain | 454 | 174 | 31 | 52% | 7% | TSector, Scene node despawn and viewport step (TSceneTaskInt by address), TTerrainPlugin |
| Physics, bodies & cloth | 363 | 242 | 13 | 39% | 0% | RFE_Dynamics_Cloth, TSimObject |
| Characters, motion & animation | 506 | 198 | 23 | 41% | 0% | TCharDecomp2, TKFCCatRom3f, SG_CharacterMorph |
| Combat, weapons & damage | 14 | 3 | 3 | 31% | 0% | RFE_Effects_Blood, TBloodSprays |
| Items, materials & crafting | 341 | 140 | 40 | 51% | 0% | TCharLoot, TItemRenderer, TPopUpDesc |
| AI & behaviour | 437 | 206 | 17 | 40% | 0% | TCharViewBehaviour, TObjectEntity, FLXG_Pathfinding |
| Thaumaturgy & effects | 697 | 296 | 83 | 35% | 0% | RF_Env_Effects, TForceBubbleEffect, SG_Thaumaturgy |
| Audio | 59 | 18 | 4 | 84% | 0% |  |
| Script, dialogue & story | 184 | 46 | 11 | 61% | 0% | TDialogueChoices, TScriptRole, TGameTips |
| Game modes, arena & session | 646 | 193 | 23 | 34% | 0% | TLoadoutForm, TDivCardForm, TCharCreation |
| UI | 959 | 312 | 63 | 34% | 0% | TCharacterView, TCompactSkillUI, RF_GUI_Text_Edit |
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
- **UI.** The original's own GUI toolkit (windows, controls, skins, bitmap fonts, icon atlas, tooltips) and every screen built on it: splash, menus, settings with key bindings and GPU benchmark, HUD, character view, skill trees, manual, portal screen, creation pickers and arena dialogs. Exarust has no GUI layer, only a bitmap-font FPS readout, launch flags, mouse input and a window-sized view.
- **Other, tools & editor.** Developer tools that never run in retail (166 of 171 functions): the dormant object appearance editor and weather panel, editor taskbar menus, the file dialog, tree view, popup menu and render-options panel. The only live part is a small debug command registry run from key bindings, which Exarust does not have.

## Largest missing components

Ordered by code size; these are where most of the original still has no counterpart.

| component | section | functions | KB | coverage |
|---|---|---:|---:|---:|
| SG_PlayerControls | AI & behaviour | 122 | 47 | 10% |
| SG_GameGUI | UI | 62 | 22 | 8% |
| SG_Thaum_Mind | Thaumaturgy & effects | 45 | 21 | 13% |
| TGUIForm | UI | 77 | 20 | 16% |
| SG_GameMain | Game modes, arena & session | 59 | 20 | 5% |
| RF_GUI_Controls_Ex | UI | 54 | 16 | 13% |
| RF_Navigation | Rendering & GPU | 37 | 16 | 6% |
| RF_Dynamics_Volumes | Physics, bodies & cloth | 7 | 14 | 5% |
| Steam Input action bindings | Runtime & platform | 29 | 11 | 0% |
| TCharacterView | UI | 21 | 8 | 1% |
| Rigid-body shape builders (RF_GUI_Controls_Ex by address) | Physics, bodies & cloth | 9 | 8 | 15% |
| TLoadoutForm | Game modes, arena & session | 13 | 7 | 0% |
| GUI support routines (unit before RF_GUI_Main) | UI | 21 | 7 | 6% |
| T3DViewport | UI | 25 | 6 | 10% |
| TCompactSkillUI | UI | 11 | 6 | 0% |
| TDivCardForm | Game modes, arena & session | 9 | 5 | 0% |
| RF_GUI_Text_Edit | UI | 17 | 5 | 1% |
| TForceBarrierEffect | Thaumaturgy & effects | 7 | 5 | 9% |
| TSelectionUI | UI | 18 | 5 | 0% |
| TEmblemForm | UI | 18 | 4 | 0% |
| TPwrForceKinesis_S | Thaumaturgy & effects | 9 | 4 | 10% |
| TSkillSelector | UI | 8 | 3 | 0% |
| TCharCreation | Game modes, arena & session | 12 | 3 | 3% |
| XA_Arena save and rank helpers (TMatchBoard by address) | Game modes, arena & session | 9 | 3 | 0% |
| RF_Env_Effects | Thaumaturgy & effects | 6 | 3 | 0% |

## Best covered components

| component | section | KB | coverage | Exarust |
|---|---|---:|---:|---|
| TVCCloth | Physics, bodies & cloth | 30 | 81% | exa_physics/src/cloth.rs; exa_world/src/cloth_draw.rs |
| TPathFinder | AI & behaviour | 32 | 75% | exa_world/src/nav/walkable.rs; exa_world/src/nav/query.rs |
| FLX_ItemFactory | Items, materials & crafting | 24 | 87% | exa_formats/src/factory.rs; exa_formats/src/weapon_stats.rs |
| RF_SceneNodes | World, levels & terrain | 24 | 75% | exa_world/src/scene_nodes.rs; exa_world/src/light_flood.rs |
| TVXGIManager | Rendering & GPU | 29 | 60% | exa_render/src/gi.rs; exa_render/src/gi.wesl |
| SG_Character_Data | Characters, motion & animation | 20 | 83% | exa_formats/src/characters.rs; exa_character/src/record.rs |
| TParticleSystem | Thaumaturgy & effects | 19 | 82% | exa_world/src/particles.rs; exa_world/src/flame.rs |
| RFE_MeshSubdivide | Rendering & GPU | 18 | 88% | exa_assets/src/mesh_smooth.rs; exa_assets/src/model.rs |
| TMesh | Rendering & GPU | 25 | 60% | exa_formats/src/rfc.rs; exa_assets/src/model.rs |
| TGSLines | UI | 24 | 60% | exa_ui/src/skin.rs |
| SG_ObjectDB2 | Data, assets & I/O | 19 | 74% | exa_formats/src/objdb.rs; exa_formats/src/objdb/store.rs |
| TCombatant | AI & behaviour | 17 | 85% | exa_ai/src/combatant.rs; exa_ai/src/place.rs |
| TMatchBoard | Game modes, arena & session | 17 | 82% | exa_ui/src/match_board.rs; exa_game/src/arena_board.rs |
| RF_Skinning | Characters, motion & animation | 25 | 56% | exa_assets/src/skin.rs; exa_assets/src/model.rs |
| RFE_MeshTools | Rendering & GPU | 19 | 70% | exa_assets/src/mesh_edit.rs; exa_assets/src/vertex_cache.rs |
| RF_Morphing | Characters, motion & animation | 16 | 76% | exa_formats/src/morph.rs; exa_assets/src/morph_controller.rs |
| TAudioEnvironment | Audio | 13 | 93% | exa_audio/src/ambience.rs; exa_audio/src/bank.rs |
| RFG_Dialogue | Script, dialogue & story | 18 | 65% | exa_script/src/dialogue.rs; exa_script/src/bubbles.rs |
| RF_Intersection | Physics, bodies & cloth | 14 | 85% | exa_physics/src/intersect.rs; exa_render/src/cube_shadows.rs |
| RF_HW_Config | UI | 15 | 78% | exa_formats/src/hw_config.rs; exa_ui/src/settings.rs |

## Combat rules

| rule | inside | bytes | coverage |
|---|---|---:|---:|
| NPC combat state | TMotileAI | 18,453 | 5% |
| damage routine: hit point, speed/impact, zone factors, armour roll | TMotileDynamics | 13,157 | 40% |
| per-part weapon stat bytes of factory weapons | FLX_ItemFactory | 3,692 | 90% |
| armour layers from equipped apparel | SG_CharacterGen | 3,680 | 40% |
| weapon mesh assembly (feeds striking-part extent) | FLX_ItemFactory | 3,282 | 40% |
| combat positioning score (spacing/reach) | TCombatant | 3,152 | 85% |
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
