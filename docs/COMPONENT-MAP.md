# Component map: the original against Exarust

Every function of the retail `Exanima.exe` (Exanima 0.9.5, Free Pascal 3.2.2, Win64), placed in a component and a section, and each component rated for how much of its behaviour Exarust reproduces. The interactive treemaps are published at <https://claude.ai/artifact/GybAiwT9utFDbJTSaPLUkC>. This page is the summary that lives in the tree.

## Headline

- **8,783 functions**, 2,616 KB of code, in **445 components** across 16 sections.
- Weighted by code size, with code the retail game never runs left out (1,343 functions, 163 KB): **14.4% reimplemented** by Exarust's own code and **5.7% provided** by Bevy, wgpu, rapier or the Rust standard library. Summing the stored per-component percents again on that same live-code denominator gives 14.7% and 5.8%. Cloth (`TVCCloth`, 30 KB) is rated **70%** here: the step, sleep, rustle, pushes, sweeps, mesh edges, the wind tile, the body reaction, geodesic pin distances, the two-pass cull and the materials that submit those culls are in; spawning a mesh for every cloth marker is not. That 70% adds about 0.8 points, so the fresh sum is about **15.5%**. `XA_Arena` at 22%, `XA_SaveGames` at 4%, and `TThaumTree` at 10% do not move that fresh sum. It is not 100%. The published 14.4% is the last full rating.
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
- **World, levels & terrain.** Holds a loaded level: scene graph and per-frame tick, the tile-grid dungeon builder, rooms and portals, streamed terrain with rule-blended materials and plants, kd-trees, sun, fog and weather, and doors, levers, pressure plates and teleporters with their signals. Exarust builds grid levels and terrain at load, with rapier for the kd-trees, and shows the grid's rooms as the original does (active, adjacent, hidden). A shut door hides the room behind it, a point is seen only through an open doorway, and a passage names the next level. A hinged door creaks from its handle speed, a pull on a shut door unlatches on the seventh held tick from a count of 20, opening shoves 3 cm, and a motor steps toward its aim (0.0003 rad, or 0.005 when fast). A sliding door creaks from its stored energy, a motorised open drops the fraction by 0.01 a tick, a close adds 0.075 and latches, and a plain open jumps to 0.6. A paired leaf is not held shut while its partner is more than 0.1 rad short of closed, and the two leaves pass through each other. An unlock waits 1000 ms, then opens both latches. Which key fits the door is missing. `TRBDDoor` (9 KB) is rated **85%**. A toggle opens a shut hinge for 100 ticks, or 60 when fast, and closes an open slider for 60 ticks. A signalled open plays the unlock sound once, and a latched or motorised door waits 120 ticks, then opens: a plain hinge is shoved 3 cm, a motorised hinge aims open for 100 ticks, and a slider jumps to 0.6 or drives for 300 ticks. A close-and-lock sets a motorised hinge to fraction 1 for 300 ticks, or 100 when fast, and latches a leaf that is already shut. A lever creaks from its arm speed, steps 0.02 rad toward the nearest detent, and signals when it arrives more than 0.1 rad from the last. `TRBRotator` (4 KB) is rated **40%**. A pressure plate rises by 0.02 while pressed and falls by 0.01 otherwise, clicks above 0.7, and releases below 0.5. `TRBPushSurface` (2 KB) is rated **40%**. A signal closes a door on 0, opens it on any other value, and toggles on −1. A fire starts at its stored size when the value is not 0. A light stores the value's low bit. A combiner sets one bit and signals when the state matches its code. A pair sends the value to both targets. Eight clean sequence steps signal the first target. A broadcast signals every tagged door linked to the relay, including the source, and every other operative that listens, except the source. `RF_Dynamics_Game` (6 KB) is rated **45%**. The lid in the lights' cube faces, day/night and weather are missing.
- **Physics, bodies & cloth.** The game's own 83 Hz Verlet physics: tetrahedral rigid bodies with spheres voxelised from their meshes, linked and collided with level meshes, plus wind-blown cloth and the character body that plays clips as target poses, balances, swings and takes hits. Exarust steps the limbs with pose drive, balance and falls on rapier colliders, and steps a cloth of Verlet particles, wind from a 16×16 tile, pins, structural and bend edges, sphere and plane pushes, swept tests, sleep, rustle, a body reaction, geodesic pin distances, a two-pass cull and one material per cull (about 70% of that component). Spawning a mesh for every cloth marker, body-to-body contact and voxelising are missing.
- **Characters, motion & animation.** Builds characters from a record and race: model, physique morphs, limb bodies and muscles, apparel merged into one auto-skinned mesh, painted skin and apparel, the motion library, item effects and golems. Exarust spawns placed characters with auto-skin weights, GPU skinning, painted apparel and clip-driven limbs; morphs, skin textures, hair, full records, item effects and golems are missing.
- **Combat, weapons & damage.** Holds only the blood visuals of combat: a spray jet at each bleeding wound and a stain that creeps down the weapon with the red damage it deals. Exarust has none of it. The combat maths lives in the body, limb, AI and item owners and is rated as 24 combat rules: 27% reproduced, mainly damage, health, armour layers, daze and weapon stats; recoil, grips, shields and positioning are missing.
- **Items, materials & crafting.** Builds weapons from .fty recipes (seeded part, deform and material rolls, stats), rolls item quality, condition and dyes, paints item and apparel textures from 25 materials, and runs equipping and the inventory, loot and container windows with icons and item cards. Exarust builds factory weapons with stats and paints cloth, leather and steel, equips by occupancy (braies return when the groin is bare), and places inventory icons in a pane that checks bulk, size and the top-most hit. Other materials, the icon picture and the description card are missing.
- **AI & behaviour.** Decides what characters do: player input and steering, the NPC brain (personality, idle activities from roles, hostility and morale, fight or flee, attack choice), roles and relations, companion scavenging, a grid A* pathfinder and the AI manager. Exarust has the player's steering and footwork, a grid search that walks around walls, the look-ahead, a duellist that closes with the same desired velocity the player uses, and the contact rating that notices, watches, engages or flees. Once a fight has started the duellist picks an overhead, a thrust or a side from reach, preference and a facing score, and it will not swing under eight tenths of its energy. A disturbance above 0.3 is walked to, and an alert of 0.2 faces a target more than 45° off or looks around after 0.7 to 3 seconds. A roam walks out and back, a patrol repeats the wander, and a sentry holds the guard pose. A path walker held by a door at most 0.7 open toggles it, and leaves a door that is already toggling. A door wait of exactly 10 seconds still holds. One millisecond later the walker seeks again, clears the avoided node, and skips that waypoint. A door that lets the walker through moves it past the door, and a late one still ends seeking. An approach within 250 cm of a span's midpoint starts that wait. The first miss sets only the high skipped bit, and a later miss also skips the cursor. A wandering look eases toward the heading, and draws an offset of the unit times 3.6 minus 1.8 once its timer is behind the clock. An aim snap copies the body's place, takes the yaw of its heading, and asks for the idle stance. A wander point is a quarter of the way from a drawn exit toward the room point, retrying once when that exit leads back. A follower stops inside (1 + the leader's facing toward them / 2) times 100 cm, follows at 400 cm (280 while already following, 1200 while busy), and replans past 100 cm. Sleep lies dormant: the dormant condition and flag, tone 0, a statue, and gait 2, and its tick changes nothing. Seeing a contact, when the flags are exactly the dormant flag, knocks that sleeper out at the current clock. After the knock-out the tone stays 0 for 1500 ms, then rises by 0.001 a tick, and the clip base climbs by 0.01 a tick after 8 s until the tone is 1. The tick that drops the dormant flag tells each person who can see, and on that tick each of those persons stores a memory word: a short count keeps counting, the hit that meets the need stamps the second plus the delay, a cooldown and a never-again nibble stay, and an activated window refreshes or re-arms. Finishing the clip base refreshes reach and thrust preference. A searched path is followed when it has two points and is forced, or when its ends are more than 150 cm apart. Finishing the clip base also holds a weapon record again: a ghoul's arm stiffness is 1 and its hands weigh twice, and a Fael's unarmed hands weigh four times with arm stiffness one half. In a scene the goal is 100 cm past the ground point's projection, measured from the unclamped foot. Out of the scene the body slides 2 cm along the path, or crosses once when the remainder is not strictly longer than that, and only the standing height gains the pelvis base. Near the end the run bit clears once the carrot fraction, read as a double, is past 0.99, and a body within 30 cm of the carrot or 100 cm of the last point soft-stops. In the scene a blocked line from the ground to the carrot clears the run bit and asks for a re-plan, and the carrot stays. `TMotileAI` is rated **28%**. `TPathFinder` is rated **25%**: the line walk, the straight walk, the obstacle test, the nearest walkable point, footprints on the penalty byte, a plan that lifts that footprint and stamps it again, and a dirty rectangle that grows by three cells and raises the floor are in. The scene-wide countdown is not. The player's click names a door, a corpse, a person or a prop, turns toward a point past 70 cm, and treats 300 cm as within reach (350 cm for a door). A drag pulls along the cursor by the nearest hit inside 1000 cm, and a box is refused above 280 cm or below −1000 cm. Pouring and opening a container are missing. `SG_PlayerControls` is rated **8%**.
- **Thaumaturgy & effects.** Runs thaumaturgy: per-body powers and focus, the power tree and quick bar, AI casting, and the Mind, Force, Energy and Displacement powers, each a render node, simulation, GI light and AI-visible entity. It also holds shared effects: particle fire, torch and burning-item flames, electric arcs, teleport visuals, portals, rain. Exarust has the small fire's particles and light, one effect for each domain (long door, shock, blast, calm), and the power table's costs with the sustained, repeat and spend adjustments. The tree's nodes are read and later nodes subtract the first node's place. Its text chunk stores each string as a length word and the bytes that follow. Drawing the tree, the other effect meshes and charge-up are missing. `TThaumTree` is rated **10%**.
- **Audio.** An OpenAL Soft front end: it binds OpenAL and EFX at run time, opens the device lazily (silent until then), loads sounds from Sound.rpk, ranks 512 voices onto 64 sources each frame, and adds reverb, area ambience and crossfading music. Exarust plays hit and footstep sounds and a looping level track via Bevy's rodio backend; voice ranking, variant sets, reverb, ambience and crossfades are missing.
- **Script, dialogue & story.** The story layer: a small interpreter for role and dialogue scripts, and the conversation engine that finds a dialogue two characters can hold through their roles and mood, keeps per-relation topic memory and drives the narrator, reply choices, journal, loading tips and scripted world objects. Exarust loads `exanima01` through `exanima08`, keeps one chosen reply, and stores a journal page. The role interpreter, narrator and scripted encounters are missing.
- **Game modes, arena & session.** The game's flow: main, pause and end-of-fight menus, switching between practice, story and arena, saving to packages with checkpoints and permadeath, class select, character creation, discovery XP and the whole arena career (company, hirelings, loadouts, matches, shop). Exarust has a company (343 copper, a capped roster, a standard match and the seven specials), experience scaled by 1 + tier/4 and split by contribution, a shop trade that pays or receives the basket difference, and an Exarust save of that session, including the day, the weeks, and the reward ids, plus the debug stand-ins (start spots, a one-on-one opponent, a character preview). The name klean selects emblem background 52 and design 81. A hire-list candidate costs the square of one more than half of six skill levels, that half stopping at 5, times 12, plus four fifths of the gear. Menus and the original's packages are missing. Wages, the day counter, a four-week season, an overnight rest of 6553, or 9830 with a physician, capped at 65535, a trainer lesson of 1000 divided by rank, a recruitment that costs 12 copper, a shop line priced at 1, 12, 144 or 1440 copper, with a sale that is a quarter of the basket, a match stake read from a coin dword and multiplied by one more than its tier, and a swept season's four item ids, wrapping after four seasons, are in. `XA_Arena` is rated **22%**.
- **UI.** The original's own GUI toolkit (windows, controls, skins, bitmap fonts, icon atlas, tooltips) and every screen built on it: splash, menus, settings with key bindings and GPU benchmark, HUD, character view, skill trees, manual, portal screen, creation pickers and arena dialogs. Exarust has no GUI layer, only a bitmap-font FPS readout, the in-game world cursor, the HUD's health and focus bars with the low-health heartbeat border, launch flags, mouse input and a window-sized view.
- **Other, tools & editor.** Developer tools that never run in retail (166 of 171 functions): the dormant object appearance editor and weather panel, editor taskbar menus, the file dialog, tree view, popup menu and render-options panel. The only live part is a small debug command registry run from key bindings, which Exarust does not have.

## Largest missing components

Ordered by code size; these are where most of the original still has no counterpart.

| component | section | functions | KB | coverage |
|---|---|---:|---:|---:|
| XA_Arena | Game modes, arena & session | 219 | 71 | 22% |
| TMotileAI | AI & behaviour | 85 | 60 | 28% |
| SG_PlayerControls | AI & behaviour | 122 | 47 | 8% |
| TThaumTree | Thaumaturgy & effects | 76 | 38 | 10% |
| TGUIDrawSkin | UI | 73 | 35 | 0% |
| TPathFinder | AI & behaviour | 24 | 32 | 25% |
| TVCCloth | Physics, bodies & cloth | 50 | 30 | 70% |
| XA_SaveGames | Game modes, arena & session | 96 | 27 | 4% |
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
| NPC combat state | TMotileAI | 18,453 | 15% |
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
