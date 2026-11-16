# Progress

The tracking file for the [roadmap](README.md). Update it at the end of
every round (see "How to update" at the bottom).

**Status: Phase A running (batches of 4 packages), with B0 in parallel.
Decisions DEC-1 to DEC-4 taken as recommended; DEC-5 and DEC-6 open.**

## Headline

| measure | start (r186, 11-10) | now (r297, 11-16) | target |
|---|---|---|---|
| Estimate: reimplemented, by bytes | 81.9% | 82.3% | replaced by the ledger |
| Estimate: engine-provided | 5.1% | 5.1% | — |
| Estimate: by function count | 73.4% | 73.8% | — |
| Combat rules | 90.6% | 90.6% | 100% |
| **Ledger: bytes in a final state** | — | **59.85%** | **100%** |
| Ledger: functions audited | — | 6,627 of 6,478 | 6,478 |
| Ledger: audit packages accepted | — | 90 of 90 | 90 |
| Live rows with scene or retail evidence | — | 316 of 2,868 (11%) | 100% |
| Retail references | — | 8 | about 50 |
| Scenario matrix rows passing | — | 0 (matrix not built) | all |
| Tests passing | 8,489 | 9,150 | all |
| Specs valid | 87 | 87 | all |

The ledger starts low because nothing has been confirmed function by
function yet; see the [README](README.md).

## Phases

| phase | state | next step |
|---|---|---|
| A — measure every function | done | — |
| B — close every open function | doing | B1 wiring and B4 ports by family; see the work list |
| C — behaviour | doing | C2 needs a scratch retail profile (owner); C4 scenes for test-only rows |
| D — release | not started | D1 infrastructure |

## Tasks

State: `todo`, `doing`, `done`, `blocked`.

### Phase A ([detail](phase-a-ledger.md))

| id | task | state | note |
|---|---|---|---|
| A1 | Ledger skeleton and packages | done | 8,783 rows, 90 packages (2026-11-12) |
| A2 | Reachability, second pass | done | walk through unit init, class tables, data blocks, callbacks and unwind handlers; dead candidates 740 → 49 functions (8 KB); the editor is wholly unreached; exceptions all explained (RTL libraries, never-constructed base classes) |
| A3 | Audit brief and three pilot packages | done | 3 pilots in 7-9 min and 136k-256k tokens each; brief and tools improved; Phase A now estimated at 25-30M tokens |
| A4 | Audit packages | done | 90 of 90 audited (5 redone after rejection, 2 settled by a second review) |
| A5 | Review of each package | done | 90 accepted |
| A6 | Coverage computed from the ledger | done | published 2026-11-13: ledger 48.3% closed (37.8% live, 8.8% engine, 1.7% no code), 39.6% partial, 12.1% open |
| A7 | Check the excluded code | done | 102 of 103 reached not-applicable rows confirmed; the RTL random group reopened as ported code; 36 game-class engine rows checked (33 engine, 1 partial, 2 unreachable) |
| A8 | Phase B work list | done | `phase-b-worklist.md`: 1,038 uncalled (291 KB), 1,548 partial (988 KB), 74 missing (16 KB) |

### Phase B ([detail](phase-b-close.md))

| id | task | state | note |
|---|---|---|---|
| B0.1 | Save slot kinds | done | inverted: a fresh saves screen continued from the checkpoint, and the portal reset deleted the checkpoint while the save stayed unlisted; fixed (round 203). Creator-side names left to the other session |
| B0.2 | Level-exit save position | done | round 204; arrival marker shown in game 2026-11-15 |
| B0.3 | Control Mind end frame writes | done | round 206; game run 2026-11-15: no undone write logged |
| B0.4 | Torrent stream in lockstep | done | the original draws from the two process-wide generators, which the step streams stand for; the sweep scene needs retuning, not the code |
| B0.5 | New game creates no save slot; arena saves dropped | done | round 205 (tests); the `--story-session` debug flag skips the real start, so a flag for a real New Game start is owed for a game check |
| B0.6 | Spirit Form adopt/home and other frame-sent writes in free mode | done | round 208; three bridge-ownership hazards left (item cursor drain race, one-step-stale copy-in values, the frame-side combat heading write) |
| B1 | Wire the uncalled | doing | rounds 209-226 (character data, dynamics, Mind powers, journal, shaders, arena, morphing, saves, skins, skill screen, item GUI, gamepad menus); next: scene light nodes |
| B2 | Reclassify the engine-replaced | doing | shader templates (213), data-stream primitives (220), hook lists (223); the deferred ring stays open (reachable wrap) |
| B3 | Close the unreachable | todo | |
| B4 | Port what is missing | doing | rounds 216, 218, 222, 225, 227; lanes running: TMotileDynamics (second pass), the route walker, XA_Arena, TVCCloth |
| B5 | Kept deviations | blocked | decision DEC-5 |
| B6 | Character creator | blocked | decision DEC-3 |

### Phase C ([detail](phase-c-fidelity.md))

| id | task | state | note |
|---|---|---|---|
| C1 | Comparison tools (seed, capture, diff, evidence) | done | round 207 tools; the ledger's evidence column: live rows by retail 11, scene 246, test 1,938, code reading 319 |
| C2 | Retail reference set | todo | 8 of about 50 |
| C3 | Scenario matrix | todo | sweep has 80 scenes |
| C4 | Unit-test-only ports | todo | first entries: ui-01 picker title band and hovered label (two_ink_band, glow_text), promoted by review from code reading |
| C5 | Playthroughs | todo | |
| C6 | Free mode clean | todo | one conflict known |
| C7 | Retail saves | todo | unblocked by B0.1 |

### Phase D ([detail](phase-d-release.md))

| id | task | state | note |
|---|---|---|---|
| D1 | Working infrastructure | doing | D1.2 done: `context/tools/gpu_grid.sh` runs one grid at a time with a free-memory check (after the GPU loss); GPU runs on hold for a hardware fault (GPU_HOLD); determinism_grid.sh now shares the GPU lock; since round 276 the build worker's RTX 4000 runs scenes, screenshots and determinism offscreen through `context/tools/worker_gpu.sh` while the local card is on hold |
| D2 | Performance | todo | |
| D3 | Platform | todo | Steam: decision DEC-2 |
| D4 | Regression suite | todo | |
| D5 | Packaging | todo | |
| D6 | Final review | todo | |

## Audit by section

Packages are listed one by one in [`ledger-summary.md`](ledger-summary.md).

| section | packages | accepted | open functions | open bytes |
|---|---|---|---|---|
| Physics, bodies & cloth | 7 | 7 | 146 | 159,976 |
| Rendering & GPU | 10 | 10 | 277 | 139,841 |
| UI | 12 | 12 | 339 | 127,230 |
| Thaumaturgy & effects | 9 | 9 | 151 | 109,619 |
| Characters, motion & animation | 8 | 8 | 222 | 101,754 |
| Game modes, arena & session | 7 | 7 | 178 | 76,192 |
| World, levels & terrain | 5 | 5 | 146 | 73,180 |
| AI & behaviour | 8 | 8 | 115 | 70,226 |
| Items, materials & crafting | 5 | 5 | 132 | 62,916 |
| Data, assets & I/O | 6 | 6 | 118 | 38,360 |
| Runtime & platform | 5 | 5 | 69 | 20,690 |
| Core, math & threading | 3 | 3 | 75 | 15,348 |
| Script, dialogue & story | 2 | 2 | 28 | 6,544 |
| Audio | 1 | 1 | 7 | 3,295 |
| Combat, weapons & damage | 1 | 1 | 5 | 2,201 |
| Other, tools & editor | 1 | 1 | 0 | 0 |
| **Total** | **90** | **90** | **2,008** | **1,007,372** |

## Open decisions

See [`DECISIONS.md`](DECISIONS.md): DEC-1 definition of 100%, DEC-2 Steam, DEC-3
character creator, DEC-4 start and budget of Phase A, DEC-5 kept deviations,
DEC-6 composite terrain, DEC-7 approximate float instructions, DEC-9 Calm's write past the contacts, DEC-10 desktop platform differences, DEC-11 the tendrils' reads past their data, DEC-12 generated textures on the CPU.

## Log

| date | round | what |
|---|---|---|
| 2026-11-16 | 297 | NPC bodies turn and look through their locomotion as the player body does (look, look smoothing and heading kept as in the original, combat look snap past 30 degrees); the search creates the perceived goal entity the brain weighs and attends (F389), and a classify bug was fixed (the attention bonus belongs to the attended contact). |
| 2026-11-16 | 296 | The frame's Update and PostUpdate are fully ordered in named phases following the original's frame order, and the schedule test now holds every schedule to zero unordered conflicting pairs (seven documented pairs with private engine systems allowed); ordering the frame fixed a GUI bug: paints that ran before the game screens' per-frame clear were wiped every frame, so the story death's tip banner now shows (ambiguity2) |
| 2026-11-16 | 295 | The function oracle covers 110 original functions bit for bit over 5000 cases each: the callers now take the original's exact normaliser and lerp, the swing time on non-finite input matches, and about fifteen small port defects it found are fixed (NaN and negative-zero handling, a denormal sine, reciprocal-first weights, a bearing in double, a wrapped tick count); the simulation changes in the last bit from the first tick; F569 opened (oracle2) |
| 2026-11-16 | 294 | The motile body and balance lane from the port queue: the golem-suit leg halving has the original's writers; a balance step's end stamps the stand-still clock; the servo and step-approach factors; the off-balance arms only from the posture block's test; the get-up turned from the body's heading; the femur posture in the engine's quaternion order (it had been reversed); held items sleep and wake with their holder; the AI's soft stop on a jolt; round 285's Height Query questions settled as the original's (bodybal) |
| 2026-11-16 | 293 | F560's cause found with a new headless trace tool (the armour pass's head height was unordered in the fixed step, now first); F566 fixed (the hand's pick names sliding leaves by their door record, so a portcullis can be pulled); F563 closed as the original's (the placed portcullises stand open in their arches); F568 opened (a locked portcullis can be dragged aside) (physfix) |
| 2026-11-16 | 292 | The first lane from the triage's port queue: the chase leave is reachable (it reads the brain's sight of its foe), a cast started while parrying ends the clinch, a released race shocked in combat resets its swing and a dying one takes no shock; six readings corrected; most of the lane's other items wait on NPC body turning and the brain's goal entity (npcbody lane) (combatai) |
| 2026-11-16 | 291 | Three findings from the scene evidence fixed: the saves' character screen drew none of its records (its paint ran before the game screens' per-frame clear; F565), the respawn's rebuilt player got a carried torch and started outside the rooms (F564), the hire page kept the roster's records and the hub's coins drew a row high (F561); F562 is the original's fixed character-view camera (uifix) |
| 2026-11-16 | 290 | Every schedule the simulation step runs is fully ordered (1149 unordered pairs touching the same data, each kept in the order it already ran, no simulation value changed) and PreUpdate too; a test on the game's own App fails on any new unordered pair, the frame's Update and PostUpdate held at an exact count (5835 and 275) for the next ordering lane (ambiguity) |
| 2026-11-16 | 289 | A function oracle runs the original's machine code in an emulator on the worker and replays the cases against the Rust ports (exa_oracle crate, no addresses): 29 of 33 functions identical over 5000 cases each; a denormal bug in the floor and ceiling helpers fixed; the DEC-7 numbers measured (Intel and AMD differ on about half of the approximate reciprocal answers); about 70 to 110 open rows are cheap to check next (oracle) |
| 2026-11-16 | 288 | The scenes owed by rounds 273 to 280 ran on the worker's GPU with new debug hooks (a play-mode fire, weapon seeds, mechanism list and pokes, a device scan); determinism holds per binary; a twelve-scene golden suite now runs after each round (determinism, screenshots against references, log checks); F560 to F566 opened (far characters changed by the fires round, the hire page layout, the golem view's head, the portcullis reading, the respawn's dark torch, the save screen's missing records, the portcullis pick) (evidence2) |
| 2026-11-16 | 287 | The terrain lane, held since round 242, is released after a free-camera check on the worker's GPU: blank blocks are left out of every draw as the original's only path (unseen by shipped cameras), and their plants leave the camera draws but stay shadow casters; two older faults fixed on the way: the interface camera had taken part in world visibility, and plants grew over blocks never built. No lane is held |
| 2026-11-16 | 286 | The body morph follows the original's order (each target added to the base in turn, the mesh's own refit, the transfer walking the mesh incidence last face first, the first root's morph set); the slider-driven re-morph path is proven dead in 0.9.5 (four rows unreachable) and F41 closes; no visible change on the worker's GPU across five physiques and a golem (morph2) |
| 2026-11-16 | 285 | The simulation's mesh queries run the original's per-triangle tests (the face record's closest point, the front push with its two floors, the swept push, the Height Query's swept nearest, the two-sided test) instead of a library projection; the overlap query now asks the two-sided test as the original does (a defect fixed); the segment slot proven unreachable; each binary repeats itself exactly on the worker's GPU, FPS holds (tmesh). Traces change from the first tick |
| 2026-11-16 | 284 | The game GUI unit's uncalled rows are closed: the inventory's page buttons swap the weapon sets, the journal's entry list drops down and picks an entry, and an open game screen keeps the idle camera from turning, each shown before and after on the worker's GPU; three readings corrected (gamegui2) |
| 2026-11-16 | 283 | The held terrain lane checked with a free camera on the open outdoor arenas: its 44 m cut-off is the original's and no shipped camera sees it, so it is releasable once blank blocks' plants leave the camera draws (terrainrel lane); F550 split: its light lines were rain whose odd vertex count broke the line pairs across the supersample copies (fixed), its seam is the voxel GI's block edge (open) (terraincheck) |
| 2026-11-16 | 282 | A test now builds the game's own App headless with the lockstep bridge and fails on any system skipped for a missing resource or message (the bug class of F547); F549 fixed (the Shock charges about 1.2 s by the step length and its burst is aimed); F551 fixed (a dead manager back in the hub ends the session through the death flow and the end menu); F552 fixed (the GI shape box arithmetic wraps) (bridgeaudit) |
| 2026-11-16 | 281 | The GI manager's shape list is cleared at the level teardown, its dirty-marking slot proven unreachable; voxel lights and shapes blend one pass at a time in the manager's order, meshes splat in centimetres, terrain columns read the tile's vertex grid and moving meshes are boxed by their node bounds; worker GPU pictures unchanged within noise, FPS holds (gimanager) |
| 2026-11-16 | 280 | The fires' particle systems are simulation objects as the original's sector objects are: each step ticks every flame in the solver's object stage (a carried flame at its part's pose) and meets the bodies with it, the doses reaching the characters' burn; the draw keeps the step's snapshot with no flicker in either simulation mode and takes the original's render fraction (one to two ticks); the beds and portal sparks meet no body (simfires). Torch and brazier scenes owed |
| 2026-11-16 | 279 | The factory paint job's shade scratch, its overlay regions (fullers, axe holes), the steel normal's mirrored-tile turn and the Gemstone alpha job with its refracting material are ported; the apparel job takes its tile offsets from the item's paint seed and its wear by the stored byte; both generators run about 30 percent faster in wall time (texgen). DEC-12 opened (the generated textures' GPU job queue and caches as engine-replaced): its twenty rows keep their state until it is taken. Picture checks owed |
| 2026-11-16 | 278 | The death CONTINUE rebuilds the player from the saved record (a bug fixed: the respawned player had woken wearing the gear the corpse also kept); the title loads the menu scene with no player under the original's fixed camera (shown on the worker's GPU against main); early level exits make the checkpoint by the original's rule; exits ask whether the level is available, save first then archive (saves4) |
| 2026-11-16 | 277 | Garments take the body's skin through the original's morph transfer; held since round 241 for a picture check, released on the build worker's GPU: on main bent bodies in gambeson and brigandine had shown a garment ring floating above the back and torn panels, coats a white tear and the gambeson skirt flaring; with this lane they follow the body, golems unchanged, FPS within noise (skinning) |
| 2026-11-16 | 276 | The build worker's RTX 4000 runs the game offscreen through a shared runner; it found that since round 253 the casters' tick never ran in the game (the vanish turn message and the sound sets' size cache were not bridged into the simulation world, so Vanish, Long Door and the Force Blast never got past the cast: F547, F548, fixed) and that screenshot runs lost one shot in nine (F546, fixed). Determinism passes on exanima01, 02, 06, arena and ten power and duel scenes; the owed scenes of rounds 252 to 272 mostly pass; F549 and F550 opened (workergpu) |
| 2026-11-16 | 275 | The golem's coreless mark is written where the original writes it (the core drop clears it, the shown take-off and a core taken off the loot equipment set it and lay an awake golem down); the armour pass reruns only on worn or out-set changes; the re-dress runs at the loadout form's close (roster and specialist pages) and after a hub trade; the equipment finalisers are ordered step lists, their change-detection equivalent shown by test (equip). Golem and loadout scenes owed |
| 2026-11-16 | 274 | Sliding doors are bodies by default as in the original: exanima01's 37 portcullises sit slid into their walls and close 170 cm along level rails (an older reading of a lift was wrong); their step runs right after the integration, lever and router orders move them, the hand drags them and the navigation refresh frees and raises their doorway cells; the hinged step takes the original's handle smoothing, energy order and unordered motor compares (doors). Traces change with portcullises; scenes owed |
| 2026-11-16 | 273 | The hub, save screen, manual, end menu and arsenal take their keys from the window procedure's list; the root's menu toggle dispatches on the session phase as the original's (in the hub only the loadout form closes, the title page stays up); the GUI unit fills the large icon atlas from the common glyph sheet, sets the drag distance from the window height, clamps the inventory and journal opens and answers the pad's focus checks (guimain). Key scenes owed |
| 2026-11-16 | 272 | The game's crystal drain drains the player's open inventory's pack panel and the record follows (a bug fixed: the open panel had saved stale charges over the drained record); the core crystal is shown over the figure, picked, dropped on for races that take it and taken off; the view image renders at the settings' supersampling (charview2). Inventory scenes owed (GPU on hold) |
| 2026-11-16 | 271 | The AI's room bookkeeping and nearby gather are the original's (per-room lists newest first, the gather over the room and each exit's room), used by the AI tick and the player's tick, whose contacts now read its nearby list; a removed searcher lets its object reservation go; the death hook's golem glow branch decides; the chase leave clears its target; the room move is per level room, not a 600 cm cell, as an old reading had it (motileai4). Determinism runs on grid levels with NPCs owed (GPU on hold) |
| 2026-11-16 | 270 | The grid sector step's first pass is live: the force blast and wave strike bodies in the next tick before the character ticks through the sector's cell walk and gates; the particles' body pass is ported exactly and hooked into the solver's object stage (still unfed: the fires step in the frame's world, simfires lane); every fighter carries a burn and its upkeep, which had never run, now runs; round 265's particle reading corrected (simprepass). Blast and wave scenes and determinism runs owed (GPU on hold) |
| 2026-11-16 | 269 | An older save's world objects run the original's upgrade chain on a live load; three readers the game had duplicated go through the database routines; a loadout cost bug fixed (a one-handed weapon costs a point less); the container instantiate is ported, so a level's chest fill makes local copies with each row's condition (datastream). Chest, loadout and search scenes owed; level-load traces change (GPU on hold) |
| 2026-11-16 | 268 | The remaining form keys come from the window procedure's key list: the main menu (its name buffer and the music code), the settings form, the power tree (its bound power keys), the skill and destination screens (any key: F76 and F122 fixed) and the hub's fields; the root task's key slot holds the KeyDef lines and the boot's Escape, runs only the best shortcut and ignores repeats (F135 fixed) (guikeys). A camera-mode ordering gap the new systems exposed was fixed on integration. Key scenes owed (GPU on hold) |
| 2026-11-16 | 267 | The player behaviour's gamepad screen cursor is the original's (eased in screen space, advanced in the post-step and drawn at the render fraction); the capture takes the pad's look buttons and makes the desk skip its mouse move; the cursor read takes the plain aim-plane point except in an Overhead Swing and for the powers; the stagger halt, the dash and Head Pain inputs (motion 45 identified), the kinesis-drag request drop, the placements' heading store and the death hook's capture release are wired (playerbeh). Pad and combat-aim scenes owed (GPU on hold) |
| 2026-11-16 | 266 | Live arcs test the level and bodies only when their record asks (only the Shock's burst), so hand, mace, shock-weapon arcs and sparks no longer stagger opponents (a bug fixed); the arc nodes are attached, entered, removed and placed under their weapon, item or struck part each frame, so weapon tendrils stay on the blade and sparks ride the struck limb; the records share the original's strided buffer; blow wisps are placed (they had sat in the fade-in) (tendrils). DEC-11 opened. Grid scenes with a picture check owed (GPU on hold) |
| 2026-11-16 | 265 | The sector step's character walk follows the original's list (the last registered first, the player last), the level and pair hits take the original's exact weights, plane, spread and push split, level friction works in centimetres with doubles, a part's slip is the original's difference routine, and the all-pairs sector meets the level first; no measurable solver cost (simsector2). The missing sim-object pre-pass is found (simprepass lane). Traces change with level contacts or two characters; determinism runs owed (GPU on hold) |
| 2026-11-16 | 264 | The base form's key path runs as the original's: the key-down's modifier word, the focused form's key slot (Tab moves the focus, other keys go to the record, the rest climb the parents to the task); the game screens, the manual and the hub's Tab use it, and the player's hotkeys hear a key only while no game screen has the focus; corrected: Shift+Tab steps forward, the activation moves the focus only for a nonzero control (guiform3). Key scenes owed (GPU on hold) |
| 2026-11-16 | 263 | The live scene's node passes weigh every bucket mesh by the original's two alias link methods, with real light values from the alias sync, real material colour bounds and the cell flood's wall test as the gate: 7 to 12 thousand weighted meshes per shipped dungeon at no measurable frame cost; readings corrected (the gate is the sector grid bit, the light option 0x40000 is the ranged flag, the sun's directional fill is reachable) and the scene list and material queue rows are engine (scenenodes2). No picture change expected; scenes and an FPS check owed (GPU on hold) |
| 2026-11-16 | 262 | The pathfinder's planner, corner line test and nearest-walkable flood match the original block by block, including its unchecked grid index (a column past the side reads the next row); the movers sample through the original's pre-scaled frame and the room offset is confirmed; meshes flagged for it (1536 shipped props, and dropped items) queue the grid's refresh when they register (pathfinder2). DEC-5 still open. Determinism runs and a nav scene owed (GPU on hold) |
| 2026-11-16 | 261 | The step radius base follows the body tick (30 moving, 65 off its feet, 60 at a new body), a stopping walker seeds its jolt term, parries aim at the blade's parry-height point, a shield parry holds by a plane link, and footsteps play on the body's two alternating voices; several old gaps found stale or vestigial (motiledyn5). The arena next-round test, flaky on the lane branch, passed eight repeats on the integration branch. Traces change from the first standing tick; determinism runs and duel scenes owed (GPU on hold) |
| 2026-11-16 | 260 | The arena's return to the hub and the dead manager's leave to the title run behind the transition screen; both round stagings write the company's health back first; later tournament rounds keep the company's living bodies instead of rebuilding them, which had reset health and could bring a dead fighter back; the wind call is the cloth wake (an older reading had the wrong slot); the arsenal's shelves load and store, the board's item slots open cards, the hub entry plays the manager's motion (arena3). Arena scenes and determinism runs owed (GPU on hold) |
| 2026-11-16 | 259 | The base form's show rebuilds its snap slots on every screen, journal, manual and settings open, the form hit walks its children as the original does, and Tab moves the hub forms' focus; the snap slots' only reader is the draw skin's page, which the line skin replaces, and the textured-skin forms hold no control records, so the draw skin's panel, box and scroll-bar slots have no textured-skin arm (guiform2). Hub and journal scenes owed (GPU on hold) |
| 2026-11-16 | 258 | The lift's placement follows the original: the item's box centre starts 600 cm back along the cursor ray and slides to its spheres' first contact within 1000 cm (the level, the other props and the loose props' meshes); each sphere starts at the camera's near plane, which an older reading had taken for a support plane; with the controls off the hand's own frame places the lift and a block holds it (playerctl2). Lift scenes owed and their traces change (GPU on hold) |
| 2026-11-16 | 257 | The duellist drops a collapsing or dead target and leaves combat instead of standing guard; apply roles reads the fierce bit and drives the party list (companions join the story experience, are written back to their records, leave on death); an arriving follower restarts its idle search; the idle driver's room-walk goal proven unreachable (motileai3). Arena and story scenes and determinism runs owed (GPU on hold) |
| 2026-11-16 | 256 | The line skin's wide frames: the combo drop-down lists and the palette popup draw the wide window frame; the colour picker's close mark lights, draws its icon and closes; the swatch backs and the tables' header line go through the skins; F24 closed (the title band is six units deep) and the span bar half of F29 (linesrest). Hub window scenes owed (GPU on hold) |
| 2026-11-16 | 255 | The swing's opening block: a counter keeps its armed hold while the target plays the countered motion, a shield without Ward lets go at 0.4 of the clip, two weapons guard the off hand; the supported walk's head tilt, the ground placement's clinch clear and the get-up tick's clears; the muscles-off sampler proven unreachable (motiledyn4). F545 opened and held for a GPU check: the ground placement's sampler. An arena staging order bug between the end screen and the flow's steps fixed on integration. Traces change in every scene with walking bodies; determinism runs owed (GPU on hold) |
| 2026-11-16 | 254 | The arena's next round, bracket round, rematch and hub Begin are staged behind the transition screen with the original's music flags; each staging pauses three frames at black, and the AI staging freeze now ends at the screen's close (arenafade). Left: the Continue's hub return and the dead manager's exit still run without the screen (arena3 lane). Scenes and determinism runs owed (GPU on hold). |
| 2026-11-16 | 253 | Vanish and the Long Door take the body out of the world and bring it back as the original does: the vanished bit, the body and its held items disabled with the first grab dropped, the come-back's 1000-tick settling; the duellist drops a vanished target, a vanished player leaves no new footprint, and the after-image is a real tendril image sized by the body. Two old ledger readings corrected (dispfx). Scenes and determinism runs owed (GPU on hold). |
| 2026-11-16 | 252 | F415 closed: the save header holds the view camera's eye in centimetres and the current zoom; journal pages keep their ids; a door's arrival lands the player as the original's save landing does; the arena match's return reads the hub from the arena slot's saved section (saves3). Left: the between-round reload, the death CONTINUE's player rebuild, the menu scene. Scenes owed (GPU on hold). |
| 2026-11-16 | 251 | F544 closed: every practice and arena staging sets the original's staging flag, under which the fighters stand in the idle stand for about 1.1 s while the transition screen fades in; Exarust holds the AI states for 92 ticks (aifreeze). Held for a picture check once the GPU is back: skinning and terrain (blank blocks not drawn past 44 m). Determinism runs owed (GPU on hold). |
| 2026-11-16 | 248–250 | Force effects: the torrent stirs cloths, the watch list makes brains forget a gone effect, the torrent and barricade sounds, the wave's alert (forcefx). The door drive after the leaf's integration, the impact class from sounded sets (tetrarbd). The game main's session steps; a bug fixed where the HUD disappeared after every door, death or knock-out (gamemain). Scenes owed (GPU on hold). |
| 2026-11-16 | 245–247 | The item GUI draws from the frame's engine generator, a cross-panel drop makes a local copy, the socket row answers its drag-over (itemgui2). The arena's won items are instances, the board's Choose button opens the event lineup, the in-game EXIT from the arena saves and leaves (arena2). The NPC search's executors, leader and comments follow the original, F378 fixed (aisearch). Stacked, the rounds needed four test setups updated for the shared generator. Scenes owed (GPU on hold). |
| 2026-11-16 | 243–244 | The weapon switch is queued and the sets swap at the clip's 60% point for every body; the two hands are grouped each tick; the posture arms compared (a shock now ends) (motiledyn3). The combat state's down exit, the follower called back to its leader, the route's mode word and leader point, the flight's soft stop, the walking head's look (motileai2). F544 opened: the original's practice and arena staging may freeze the AI states. Scenes owed (GPU on hold). |
| 2026-11-16 | 242 | The level's cloths run inside the bodies' tick at the original's points: the body takes its share of a cloth's push inside the solver, grabs are grab records, normal-mapped cloths rebuild their tangents, the cloth grid is the scene grid (clothbody); a follow-up brought the stage's CPU cost back to the base on the game's path. Scenes and an FPS check owed (GPU on hold). |
| 2026-11-16 | 241 | The inventory character view's equipment side: a bound body's trust follows drags and drops (F278 fixed), script HasItem answers (it always answered no), the full shield take, the swap gate and the page buttons, the arsenal's keys (charview; F281, F282 fixed). Grid scenes owed (GPU on hold). |
| 2026-11-16 | 239–240 | Each game mode gives the camera its own record and GameModes is the live mode word (F361, F362 fixed); window points use the original's projection (a 2x lift-warp bug under supersampling fixed); the post sharpen follows the supersample level (cammodes). The living placement of a character body replaces the rigid moves for the arena's kept bodies, the golem reset, the revive, the player's level placement and the hub entry (replace). Two lanes held before integration: clothbody (CPU cost being cut) and skinning (a large garment-weight change waiting for a picture check). Scenes owed (GPU on hold). |
| 2026-11-16 | 238 | The in-game GUI unit: the heartbeat fades out, the HUD takes the shield, the power bar and the bars' holds, the dead powers close the corpse's windows, the AI search reads the open-container registry, a bound body's inventory sets its mind window, a save writes the open packs (gamegui; F31, F32, F34-F38). Grid scenes owed (GPU on hold). |
| 2026-11-16 | 237 | A scanned player's story death runs the record strip: the dressed corpse stays in the level and is saved there (F543 fixed); every story level change reads the save's own section first; the session's arrival words and the story levels' fog follow the original (savegames2). Scenes owed (GPU on hold). |
| 2026-11-16 | 235–236 | The player controls' switches follow the assembly (Spirit Form's hold, the drag reach limit, the lifted item's turn and put-back, drop scripts, the power retry, the held GUI button's release, object tracks, the idle double click, the pointer warp, the walk-latch reach); two defects fixed: a used object was deleted, the device power-down had the wrong trigger (playerctl). Body part sounds from the race and suit, the step reach's order, the weightless share, held re-places, the drops and attach on the item's own body (motilepart). Scenes owed (GPU on hold). |
| 2026-11-16 | 234 | The base form's task rules in the live window manager: layer insertion and tree draw order, the I and J toggles acting on the window on top, the press's child raise, the remembered focus and the focus-loss deactivation, the close cascade with the reference sweep, the record release's landing bit, the open screens rescaled on a unit change (guiform). Grid scenes owed (GPU on hold). |
| 2026-11-16 | 233 | The live solver takes the scene's own grid (the level's grid chunk), with the original's flat cell index and position-overflow reset; the static collider grid's origin bug fixed (long walls were found only from their centre cell); the pair pass, friction and level impact gates in the original's precision (simsector). Determinism runs owed (GPU on hold). |
| 2026-11-16 | 231–232 | Cloth set-up and step follow the assembly: link walk, full-correction edge pass, integrate order, the scene's wind, the rustle, the draw update between ticks (cloth; an FPS check is owed, the walk roughly doubles the links). One AI route walk and path follower for every state; five duplicate walkers removed; F370 fixed (routewalk). Game checks owed (GPU on hold). |
| 2026-11-16 | 229–230 | Level lights are registered as scene light nodes; their aliases come from the light flood's cells and equal the old room lists on three shipped levels (scenenodes). The arena's company deaths, departures, hires, day board, hire list, continue fixups and kept bodies follow the assembly; won prizes reach the arsenal (arenapart). Game checks owed (GPU on hold). |
| 2026-11-15 | 227–228 | The path planner plans with each AI state's goal record (size class, alert shift, accept distance, end test) and the walks use the original's single-precision raster (pathfinder). The turn stall, the hard hit's frozen swing and the knockback along the striker's slot vector follow the assembly; a hovering player looks ahead by the straight walk (motiledyn2). Game checks owed (GPU on hold). |
| 2026-11-15 | 223–226 | Hook lists closed as engine with schedule-order tests; the deferred ring's wrap confirmed and left open (hookorder). Item panels, cards, sockets and the crystal drain, 17 findings (wireitems). A limb struck through a parry is wounded; the locomotion push enters before the lift (motiledyn). The gamepad's menu dispatch on gilrs (menupad). The GPU dropped again on driver 620: diagnosed as a hardware fault (the card was twice not found at power-on); GPU runs on hold (`context/tools/GPU_HOLD`) until the owner reseats it, so these rounds' game checks and determinism runs are owed. Ledger 51.9% → 52.6%. |
| 2026-11-15 | 221–222 | The arena's tables run through the ported table view, with selection bands and drag labels; the skill ring closes on any key (F76, F77); no table in 0.9.5 edits, chooses or grows, so those rows are unreachable (wireskills). NPC flight, leave, combat exits, the nearby gather through the rooms' sight test and the follower's closing walk follow the assembly, lockstep unchanged (motileai, B4). Ledger 51.6% → 51.9%. |
| 2026-11-15 | 219–220 | The line skin's control draws are the live painters (separators, labels, button glow, combo rail, text field underline; F25, F27, F72, F87); 27 runtime, data and core rows closed (content stream reset at quit, engine maths call sites, data-stream primitives as engine, the old grid layout unreachable). DEC-10 opened (single-instance guard, window icon, adaptive vsync). Ledger 51.2% → 51.6%. |
| 2026-11-15 | 215–218 | Morph transfer on the original's plane barycentrics, the heal sound, the came-loose hook (wiremorph); refraction for glass and gems, subsurface for candles, soap and ceramics, the fast-loop shadows (refract, B4); running powers saved with the player and rebuilt on load, the GUI reset on level change (wiresave); the held-tree trio, Spirit Form's disable, the respawn portal F542 (holdtree, B4). First rounds on driver 620, no GPU drop. Ledger 50.7% → 51.2%. |
| 2026-11-15 | — | The GPU fell off the bus again (Xid 79, the tenth since 2026-10-02) after the network card move. The drops began right after the 2026-10-01 update to kernel 7.2 and driver 615 and happen at idle as well as under load, so the driver is the leading suspect; the owner updated to kernel 7.3.3 and driver 620.70.06. `gpu_grid.sh` now refuses runs on a 615 driver. The five running lanes were restarted from their worktrees. |
| 2026-11-15 | 210–214 | Five B1 lanes: body settling, freeze and held-item park (wiredyn); the Sense Minds sight, the awakening glow and Loyalty's relation (wiremind); the journal's text box and the manual's item view, and a form defect that kept every record release from its callback (wireui); the shipped shader variants checked against the WESL, closing the template machinery as engine-replaced under DEC-8 and adding the translucent and fast-light programs (wireshader); the arena match board and session ends (wirearena). Ledger 48.5% → 50.7%. |
| 2026-11-15 | 208–209 | Spirit Form adopts and returns on the simulation step, and frame-sent events wait for the last step packet (B0.6); character data wiring (B1). Game run: both mind powers return the player, no undone writes; ledger 48.5%. |
| 2026-11-15 | 206 | GPU back after the PC fix (the network card had split the GPU's PCIe lanes; it fell off the bus 9 times since October; a stress test passed). Game smoke runs owed for rounds 204-206 done, no GPU errors. Phase B lanes restarted: spiritstep (B0.6), wiremind, wirechardata, wiredyn (B1). |
| 2026-11-13 | — | Phase A audits complete: all 90 packages accepted; coverage now published from the ledger (48.3% closed beside the 82.3% estimate); Phase B work list generated; 540 findings. Rounds 204-206 (arrival, new-game slot and saves, Control Mind on the step) pass their checks and wait for the GPU. |
| 2026-11-13 | — | UI, rendering, thaumaturgy and world audited. Two packages rejected and redone. The RTX 3090 dropped off the host after an out-of-memory error; `gpu_grid.sh` now queues GPU runs. Round 204 (arrival placement, B0.2) passes its checks and waits for the GPU. |
| 2026-11-12 | 203 | Rendering and UI fully audited: 26 packages accepted, ledger 20.7%, 151 findings. Round 203: the save slot kinds fixed (B0.1). |
| 2026-11-12 | — | 9 packages accepted, ledger 12.3%. The audits found 38 defects so far (`findings.md`), among them blank terrain blocks drawn and shadowing, glow link rings scrolling backwards, no refraction, the journal buttons unwired. |
| 2026-11-12 | — | A2 done (reachability through class tables and callbacks: 49 dead candidates left). Pilots accepted on review: physics-02, render-01, ui-01. Batch runner for the other 87 packages running. Ledger 6.9%. |
| 2026-11-12 | — | Phase A started: pilots physics-02 and render-01 audited. Both show the component ratings overstate: TVXGIManager is rated 97% but 58% of its bytes are open at function level; TMotileDynamics 97% with 88% of the package's bytes partial (mostly the two biggest routines, not re-audited block by block). Retail addresses removed from a code comment and five archived proposals. |
| 2026-11-12 | — | Roadmap written. Ledger skeleton built (8,783 functions); 90 audit packages cut; first reachability pass. Project still paused. |
| 2026-11-11 | 187–202 | 16 rounds: NPC concern and flight, wraith smoke, terrain holes, race fixes, parking, timer table, domain wheel, portal reset. Estimate 81.9% → 82.3%. Paused at the owner's request. |

## How to update

At the end of every round:

1. Run `python3 context/tools/ledger/build_ledger.py`. It rewrites
   `ledger-summary.md` and fails on a malformed audit row.
2. Copy the new headline numbers into the table above.
3. Set the state of the tasks that moved, and the accepted count of the
   sections that gained a package.
4. Add one line to the log: date, round, what changed, in plain words.
5. A new decision, or one the owner answered, goes into `DECISIONS.md`.
6. Commit this file with the round's coverage commit.

A task is `done` only when its "Done when" line in the phase file holds.
