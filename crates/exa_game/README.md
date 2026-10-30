# `exa_game`

Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: the arena career (`career`) has the company, the roster, matches, a shop line priced at 1, 12, 144 or 1440 copper with a quartered sale, wages, the season, overnight rest and a trainer's lesson. One effect per thaumaturgy domain (`thaum`) and an Exarust session save (`session`) are in; `thaum::{casting, force, energy, displacement}` run the Force, Energy and Displacement casts with their gates, prices, slots and AI advice, and `thaum::{arcs, barricade, wisps}` the electric arcs, the force barricade and the displacement wisps, live in the game through `thaum::live` (`--debug-arcs`, `--debug-barricade`, `--debug-wisps`; [`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md)). The tendril emitter also has its world entry and removal and its node's update under a local-frame parent (`thaum::arcs`); the barricade starts cast motion 0x24 with the power 0x20206 and its focus reserve (`Barricade::add_to_world`), and its line is stamped on the navigation penalty while it stands (`exa_world::nav::stamp`). The force-shell aura (`thaum::ForceShell`, `shell_draw`) has its opening, hold and closing step, its eight ripples, the impact sound, the rigid copy, the `effectshell_v`/`effectshell_f` uniforms with the fade plane, and the skinned draw's placed vertices; a Shell cast spawns its aura node. The active power classes behind those casts are `thaum::power`: the base class every power derives from (its focus, state, HUD word, ending), the power-list, load, Power Glean and spirit-body routines, and one module per domain — Mind (Sense Minds, Mask Mind, Confusion, Enrage, Trust, Suspicion, Loyalty, Command Dead, Control Mind, the possession marker and Spirit Form), Force (shell, bubble, barricade, strikes, kinesis, throw), Energy (shock, shock weapons) and Displacement (the shared vanish effect, vanish, the long door, constant displacement, the summon). They are plain functions with tests pinned to the original's numbers. `thaum::cast` now creates them from the player's bar: `C` opens the bar, a digit picks, the arrows step and `Q` runs the picked slot through the bar's cast entry and the domain front ends, and each body tick walks the active list with `PowerBase`'s control and release. The five effect materials, their signatures, the noise picture's creation and upload, the material routines' node transitions (flags, state words, the aura's and shell's sub-object replacement, the `gi_multipoint` table and its shutdown) and the four layered draw passes are in `thaum::effect_material` and `thaum::effect_pass`; a Shell cast now spawns its aura node (one sub-object, the noise picture), a Strikes cast its effect node, and a vanish, long door, summon or constant displacement cast its effect node, with a long door's move placing the body after the caster tick. Every focus and AI-advice comparison passes a not-a-number, as the original's `COMISS`/`COMISD` refuse branches are behind `JP`. `thaum::effect_draw` submits the Force effects to the GPU: the blast-family and torrent ribbons with the ported `effectforce_f`/`effecttorrent_f`, a Shell cast's aura as copies of the caster's skinned meshes with `effectshell_v`/`effectshell_f`, and the `_ForceNoise` picture made and uploaded only while one of them is alive. The demon itself, the vanish's body show/hide on Exarust's drawn body and the other effect materials' programs (barrier, bubble, barricade, strikes, tendril, displacement shell) are still missing. A career match (`arena_match`) draws its arena with the variety count and the no-repeat lists, its track from eleven at their own volumes (the Beast plays its own), ticks once its first second has passed, ends rounds by extra rounds and lives (Elimination, Reserve), resolves falls (dead below 0.01, or below −0.15 of the maximum with a physician; saved otherwise below 0.01), writes health back as a word, and settles the return with the company's thirteen records; `ArenaMatchPlugin` runs the tick in play mode with an opponent. The hire list (`hirelings`) draws a candidate's profession and level from the counter, renews daily, snaps a trainer's skills and trims a combatant's loadouts, and picks the loadout a fighter takes into a match (half its levels capped by the tier, two under for a Captain's wing), repairs a broken list, copies a loadout up on a rank and values an outfit; the arsenal (`arsenal`) shelves items by kind. The staff (`staff`) hires with the purse, room and duplicate checks and their messages, recruits for 12 copper, stows and releases kits, renumbers fighter ids, and on a leaving drops the id from the entries and the log's saves; the daily report (`report`) titles the day and writes the log, training, rank, hire-list and tournament lines. A roster row dragged onto another takes its place and the moved ids follow in the log, the entries and the order list; the day's lessons teach the manager, then the fighters. An item sold from the arsenal (`arsenal`) leaves every box and holder when unique, else one copy (a 0x200 copy first), the holders keeping what is left. The outfit tables are the shipped arena data (`arena_outfit_data`): the large pools keep their ids, weights and masks, the small ones their rank/cost or class words, and the item-fact pass fills ranks, points and bonuses at run time. The two generators (`generate_profile`, `generate_by_level`) run the original's complete call sequence. The spawn plans (`arena_spawn`) give the body generator the beast's tier items and voice pitch, the rival company each tournament team wears (`rival_company`), a generic or hire-list opponent's level and eight-byte outfit, the manager's lift, role and team, and the hub's nearby-weapon query. `manage_icons` holds `arsenalico.rfi`'s seven 96-pixel cells and the runtime atlas they grow into (two cells wide, four tall); `arena_match::match_tick` and `arena_hub::enter_hub` are in. A fresh tournament match (`match_layout::raise_fresh_match`) stops a side at the first body the generator refuses, reports "Failed to generate match motile.", links the successes at familiarity 100 both ways with a 3000 ms grace on both AI clocks, and stands the player's own entry without spawning it again. A rematch (`match_layout::stand_rematch`) drops bodies in the dead mode, stores a tracked fighter's health on the full scale, and stands the survivors on the player's spread with a 10000 ms grace and a rolled jitter for everyone but the player. The scene body generator (`scene_bodies`) stands the round's bodies on the placed-character path: the player's own body, the company's other entries, and every opponent from `arena_records::generated_record` (the random NPC pass and the outfit tables built from the object database) through the merged character mesh; the Beast writes its own Ogre record (`arena_records::beast_record`: race byte 3, combat skill 100, its tier's hand item and worn pieces, a smoothed voice pitch, 50000 experience on the profession grid), and every generated or Beast record restarts the shared sequence from the byte-reversed performance counter; the company entries and the hub specialists keep actor templates where their records are not kept [guess]. The end screen (`arena_session` and `arena_live::ArenaEndScreen`): a round's result sets state 8, 5 or 4 and opens screen 3 (next round) or 2 (end); its immediate Continue follows the auto-match switch the game never sets (`--arena-auto` sets it). Continue sends a dead manager to the main menu, starts the next round (a tournament raises the bracket's next fight) while the round less the losses is under the extra rounds, and otherwise settles the match and returns to the hub with the 300 cm, half turn, 0x100 placement. The next round keeps the company's fighter and generates a fresh opponent at the match's level and tier, or, when the last round was lost and the fighter is dead, brings in the reserve entrant against the opponent who won. The return writes every company body's health word, books the result and its records, and on a tournament day with an entry drops the unfit and reassigns the pending events' rivals.

`div_card_screen` opens a divination reading from `OpenDivination` and
draws it, playing `PprMov` at a deal step and the first three turns; `emblem::preview_layer` draws a company emblem through its
shader. The character tick's physical side lives in `limp` and `stages`: a living body's clip is clocked, sampled and put on its character frame, its balance and feet measured. A spawn samples the first clip before it places the body (`limp::spawn_pose`: the pose-only sampler's key pick at the animator's clock and blend, `limp::placement_pose`, posed on the root's frame and put on the parts). With the play flag `--original-spawn` (`limp::OriginalSpawn`) a living body spawns in the original's order instead (`limp::original_spawn_pose`): the pose-only sampler's key pick, the pose lowered by the pelvis's posed height onto the frame raised by it (the same first placement), the placement's leg snap, the parts placed again with the pelvis at the snap's frame height and the snap run once more there; the controller keeps its place. Every spawned living body's pelvis height over its feet is logged at debug level for its first 120 fixed ticks (`limp::SpawnWatch`, "spawn pelvis: …"), and the original spawn logs its snap ("original spawn …"). The placement's leg snap that follows in the original (`Ragdoll::snap_placement`, with `Ragdoll::placement_feet` mapping its frame height to the feet) is a tested rule and is not called: run at spawn, with the controller moved to the snap's ground, it left the standing player squatting (pelvis 100 cm sinking to 51 cm for about a second on the RTX 3090 check, against main's steady 102 cm). A clip that replaces Walk or Run asks the tick for the trailing foot; the carry stage plants it after the sample (`Locomotor::plant_request`), and an unplanted request lapses after 500 ms; a body the movement decision moves drops it. The thighs take the body tick's posture: the combat turn while the behaviour is in combat and the body is neither walking nor off its feet, the clip's own turns (or the splay) otherwise. The muscles-off sampler (`limp::sample_limp`: 0.9 of the race's clip rate, the clip's full root travel, the gait overlay slerped in by √run, the height target (2·run + 2)·scale while walking and 2·scale otherwise plus the deepest contact under the sole (from −500 cm) and the root (`limp_height_target`), a queued clip's switch clearing the feet, every part's particles snapped to the pose on the character frame it had when its muscles went, feet planted from the keys' flags) is a tested function and is not scheduled: the original runs it only for a body whose muscles are off and which is not dead, and a dead body's tick samples nothing, so a corpse stays a free ragdoll.

The random outfits (`arena_outfit`): rank windows, the quartered and halved weights of the two ranks under a limit, masked and max-rank picks, the stepped roll, the two-pass pick that refuses a piece already worn, the apparel bonus merge, the weapon, shield and apparel row fillers from item facts, the smooth unit look curve, the profession-level grant and a random NPC's curves. Every table is caller-supplied; the retail ids and names are data, not code. The save ids (`arena_ids`): before a save every manager, stash, arsenal and staff item is prepared, in that order (a record's hands, worn items, loadouts, then carried items); after a load the entries the object table rewrote take their new ids and, when flagged, the first stash item (or the first employee list holding it) and the loadouts, or the hand pairs and the carried items, follow. The fighter rank is the six skill levels summed, halved and capped at Master; the arena numbers the manager first and marks the classes' leaders. The company windows (`manage`): roster labels and name inks, Recruit and Hire closed at eight fighters and ten employees, report-line clicks, the arsenal form's pages, selection, steps, dismissal, rank slider and points line, record titles and texts, Company Status. A tournament event (`tournament`) settles the events the company skipped with a rival of 2 to 7 (2 to 6 when the return to the hub settles them on a tournament day), runs rounds less one as extra rounds and fills its bracket with `sides << (rounds − 1)` teams; forfeiting the tourney empties the open events; arriving at the hub and the rematch lineup are in. The hub now draws and opens them (`arena_hub_live`, `hub_windows`): its entry plays the hub music, places the manager, installs the click and fall hooks, and the click hook opens the board, Manage Company or the trade window on a `GuiTask`; the arena arsenal's window (`arsenal_view`) paints its column frame, five item-panel columns, the control row and the embedded inventory and runs its keys and scroll presses, but nothing opens it yet and no arena store fills its columns; the board's paint list, the roster rows and the shop stock draw through `gui_paint`. The name klean selects emblem background 52 and design 81. A practice fight (`practice`) advances by one and stops at 8: novice names opponent01 through opponent08, expert names opponent09 through opponent16, and red gains a tenth of the maximum. The partner is not spawned. An unwatched match draws a team from `(count × 0.5 + 0.5) × strength sum`; its scene draw leaves the player's team out when the session is forfeiting (mode 16), spans one more than the weight total and takes the first running weight at or above the pick, stepping the roll even with no team left. A bracket draw keeps every team, spans the total and needs a running weight strictly above the pick; its first group keeps a forced id and does not step the roll. A lesson that crosses a rank copies the previous rank's kit. A tournament field draws 16 of the 19 companies. Roll 0 ends at 0x4b157bf0. Roll 1 ends at 0xead32bb1. A day chooses six of seven templates. Rolls 0 and 1 end at 0x1210d4e5. Roll 4 ends at 0xb761a41e. A performance counter replaces the roll by reversing its low 32 bits. A copied match multiplies the fee and the prize by one more than the slot. A seeded unit draw tempers 624 words. Seed 5489 starts at 0xd091bb5c. A record copy writes plain fields, the gaps, and the tail. Managed strings are not in. Before a match the first specialist under 700 health is removed and the death count rises. Health from 700 through 64999 leaves without that rise. Health of 65000 stays. A combatant stays. Kit release is not in. In-game menu command 4 in arena dismisses a child whose flags include 0xce100000. That child releases a capture and clears style bit 0x20 when it holds one. A remaining capture publishes its code and name. Bit 0x8000 passes its low byte as the shape. Shape 0 shows the default image when it differs and is not zero, and stamps the clock. Shape 7 shows its record. The child clears a hover, closes the task's other children that name it, then closes its own children from the first to the last with argument 0, unless it is already closed. After the lists, a focus is counted once before argument 1 copies ten hand words, when a focus is set or the mask shares bit 0. A session mark of 1 becomes 0, a flag becomes 0, and a code becomes 4. Side 0 copies sixteen words from the first bank when its flag is set. A ready install of index 1 shows that record once and runs its hook. Index 0 shows a fallback and stamps the clock. A shown value is passed with two platform words. A shown value activates the action set. The Steam library is not loaded. The gate hook sets its flags to 1. The watch hook stores a stamp of 0 without bit 0x2000, and a leave enters the session. A clear gate counts a pick. A taken item sets flags 5. A hold byte does not pick. A parent equal to the world counts an open. A fast aim copies its place. A miss with the hold byte clears the aimed word. A taken item sets click flags 0x11. A hover stores flags 0x15. A record of kind 0x10010a or 0x10010d whose link is the world counts one open. A different link does not. An item id of 0 does not count a kind call. Bit 0x80000000 searches the high table and bit 0x40000000 searches the wide table, after those bits are dropped. A span that holds 0x20000 counts one kind call. A cursor at the view edge clears the hit. A marked item drops a held count above 0, and byte 1 closes a pointer. The first pane tagged 0xce1c2013 is shown. A scene keeps the last positive body. A new pane doubles the row before the slot and clamps the place. The ray starts at 4000 along z. A centre at 100 leaves reach 100, and a centre at 500 is left. A centre child sits at half the spare. A free camera ray, nested children, and the focus method body are not in. A cleared mark registers code 0x8001 unless the hold byte is set. Raising the mark drops the hooks. An open world sets the side from its byte. Raising the mark to 1 installs index 0 and does not copy a bank. A mask without a focus does not count it. A shown child hides and the redraw word gains bits 0 and 1. A pinned child raises a notice and still closes them. The command then runs that leave, asks for a save that does not abort on low health, marks the hub busy, and sets the phase to transitioning. Drawing the cursor image and the other menu commands are not in. Menus, the other emblem controls, and the original's packages are not.

The original's prelude helpers (`prelude`): the tips popup with its two
tables (eighteen rotating tips and six control hints; index 0 draws a
rotating tip under the `ShowGameTips` setting and, as the original does,
shows the draw-th tip rather than the draw-th least-shown one; index `n`
shows hint `n − 1`; `Shown::text` names the banner's line from
`exa_ui::game_tips`), the `actors.rpk` template reader with and without its
cache (records matched by their own sixteen-byte name, letter case
ignored; a nameless template takes the asked-for name; each template keeps
its own item database, which its local-bit references name:
`template_item`), and the practice
opponent's plan (traits t0..t2 set to −100, 100 and 100, team `0x10`, a
3000 ms reaction delay; the original's fixed spot is recorded but not
used). The flow's `Step::Tip` now shows the popup (`TipBannerText`): the
index picks a tip or one of the six indexed messages through `Tips::show`,
and `TipsScreenPlugin` paints the banner — a translucent box centred 8 units
down, its text in `FONTBASE_24` — through the GUI draw path; `Step::ClosePrelude`
and `Step::HideEndMenu` hide it again (the original's end-menu hide also
hides the tips). The two tables live in `exa_ui::game_tips`. The binary's
`--opponent` builds its fighter from `opponent01` when the actors are
installed, dressed and armed with its own items, and gives it that team and
delay.

The power tree (`thaum::tree`) is the whole screen's rules: the domain's
tree file and its marks, the technique a new power asks for, the click that
picks or drops a learnt power against the record's focus budget, the pan,
zoom and spring, the domain wheel child's place, shares and fades, the
ten-slot power bar with its load, store, cycle, drop and flash timings and
the running powers' slot marks by class (`SlotMark`: handed out on the
bar's load and by the caster's walk when a power ends), the
cast entry that dispatches by domain and the persistent-slot budget. Its
store writes a domain's learnt and picked
nodes back into the record, plus the form's creation fields, the opening and
closing fades, the 17 ms view frames whose zoom and pan blend about the
cursor, the eased opening scale about the first node, the bar's 900 ms
open/close animation, the tooltip and label layout, the stretched link's own
cubic, and the caster's grant caller. The running screen (`power_screen`)
opens the tree with P, draws the tree and the bar through
`exa_ui::tree_draw` and the domain wheel child's own paint (backdrop, sector
bands, choice glow, icons, ring frame and label, hovered every frame)
through `exa_ui::wheel_draw`, draws the tooltip, labels and prompt with the
GUI skin's own font: the interface level's main font gives the advances,
glyph quad and line step, and its glyphs sample the level's atlas when the
install ships it, else the base `fontbase24r` atlas scaled to that quad (the
packs ship only the base one; a focused prompt button's caption gets the
skin's one-pixel outline). It plays the screen's `ThaumShow02`, `ThaumSel01`,
`ThaumSlot02`, `ThaumChnl01`, `ThaumChnl02` and `ThaumFail01` sounds through
the audio environment, and takes the arrows as the bar's cycle keys. A
constantly displaced body
(`thaum::veil`) gets its shell effect (starting dead, fading the body's mesh and skin
parts), its displaced collision bit, its hum start point, its tendril
image, its visibility bits and the hit veto with the 500 ms collision
restore; none of it is wired into a running game yet. The running screen (`power_screen`) opens the tree with P (while it is
shown its form has the keys: the arrows step the bar, any other key closes
it after the opening fade's first 50 ms) and draws the
tree and the bar through `exa_ui::tree_draw` into a kept canvas that redraws
only the changed batches' boxes (in row bands on the compute pool) and draws
over the HUD; the bar's keys are the hotkey
handler's, which sends `thaum::cast` the player's bar commands, and the
drawn bar takes the caster's record and slots. `thaum::visuals` makes the
creators' scene nodes, materials, noise, hooks and the teleport placement. Rules:
[`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md).

The platform layer's rules live here as plain models: `frame` holds the
original's frame-rate cap (`FramePacer`: unfocused 32 ms, else the
display's or the cap's period rounded to nearest with ties to even and
1 ms early), the real/game clock with its paused-time accumulator
(`FrameClock`), the periodic timer table (intervals scaled by the time
multiplier, re-armed from the frame time, rescheduled from the interval),
the 128-slot deferred-call ring (127 calls; once wrapped only its head runs
a tick; a call carries the procedure and data tags its cancel routines
match and can be dropped by either), the redraw-flag idle wait
(`idle_wait`: 1, 32, 64 or 1024 ms, none for an immediate redraw, at least
32 ms minimised), the logger's
presentation levels (status line, timed overlay, message box, file only;
level 5 skips the file), the sixteen-entry newest-first status history and
the fifteen-line timed overlay whose `title : message` lines stay two
seconds plus fifty milliseconds a byte and go one a tick. `hooks` holds
the engine's two callback lists: the priority-sorted method list (code,
data and priority records, insert before the first greater priority,
remove by code and data) and the procedure list (one entry per code,
duplicates refused), both dispatching in order and dropping every call
that answers false. `window` holds the
display-mode list (modes at least 640 wide, every refresh rate per size,
the original's exchange sort by area), the refresh rate (unknown below
30 Hz), work-area fitting, the 640 × 480 minimum track size, the full-screen
rule (`fullscreen_mode`: borderless over the desktop, exclusive only for a
listed `ScreenWidth`/`ScreenHeight` other than the desktop's), the swap
interval's present mode (immediate, or with vertical sync the adaptive
`AutoVsync` while Exarust's frames are slower than the display, where the
original uses plain Fifo: `ADAPTIVE_VSYNC`), the Alt+Enter toggle, the
full-screen window's minimise on losing the activation, the pointer warp's
four-move hold (`WarpHold`) and the GUI drag's pointer clip with its
endless edge wrap (`PointerClip`). `frame` also holds the logger's message
box (`MessageBox`: level 6 the error icon, 7 the warning, style words
0x42010 and 0x42030), the "minimum requirements" text and the
operating-system error report (level 5, "Windows Error"), which the save
writer logs after "IO Error : Failed to create package file." when a
package file cannot be opened; `hw_config` logs "Display mode not
supported." for a full-screen size the display does not list.
Tests pin the arithmetic and the orders. `hw_config` applies the full-screen
rule and the present mode, `exarust::frame_loop` the pacing, the focus key
release, Alt+Enter and the minimise; the other models document the original's loop, which
winit's event loop replaces. Rules:
[`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md) (Frame pacing) and
[`docs/UI.md`](../../docs/UI.md) (Settings options).

The save game (`session`, `slots`) writes `ExanimaNNN.rsg` or
`ArenaNNN.rsg` packages in the original's container: `#playerdata` with
the story or arena header, the journal, the story's always-written
discovery and global-experience chunks (an old save's chunks are kept and
put in the writer's order) and an Exarust chunk holding the session (mode,
level, journal, used topics, company, roster, focus, clock, seed, start
background, discovery, global experience worth, memory flags, transition
state); saving over a save of the same mode keeps its character placement,
its other sections and the chunks Exarust does not produce. `save_full`
writes every section in the original's order (world character database,
player data, world object database, level) and a "saving disabled" flag
skips the save before the death check. Listing a folder opens each file as
the original does (resetting a bad header), lists a number once (a number
whose first file was left out is tried again with its other file), deletes only story slots with an older
signature, shows the placed character's name from `#worldchrdb`, orders
the rest by the `Exanima.set` key and moves a loaded slot to the front; a
load opens the first file that exists, converts an old arena save (drops
`arenahub`, resets the placement, removes the skipped chunks, writes the
package back), copies the checkpoint over the save and back by name,
forces `exanima05` for a mid-transition story save and reports whether
the level section is present; the character screen's load prefers
the save, takes the checkpoint only when asked or when the save is
missing, and reads with mask `0x100010`. A new slot takes the lowest free
number at the front and is an empty file (`create_empty`; `create`
writes the session too); deleting removes both files. A checkpoint (`.rcp`)
is written once per slot by copying the save, and a manual or transition
save refreshes it; a death deletes the save and drops the slot unless a
checkpoint exists, which the next load copies back. A save's
`#worldchrdb` is read through `exa_formats::character_db`'s world
database (id-sorted lookup and lazy record), the same lookup the original
uses. The original's world
databases, character placement export and level sections are not exported
yet, so a save made from nothing is not loadable by the original. The
level-transition rules (`save_level`): the spawn height (108.18 less 11.1
per body extent, plus 5 or 13 cm over the probed floor), the six follower
offsets rotated by the spawn matrix, each follower's own lift
(`place_followers`), the duplicate-motile list and the
follower re-attach (UID and controls re-created, carry cleared, the
minimum distance raised to 5 cm), the menu background scene
(`menuscene.rfc`, fog 1.8, view (0, 500, 700), target (0, 0, −200)), the `<locale>.rft` terrain name and the scene-content classifier.
The arena's save-id pass also classifies a re-added item onto its shelf
(with flag `0x400`, once): `arsenal::ItemKind::of_item` reads the object
record's sections — a body makes the item apparel and its armour type byte
(3 through 6 armour, else clothing), otherwise a weapon section, else a
shield section — and `arena_ids::kinds_from_items` turns a saved world
object database into the lookup `regenerate_lists` uses. It also flushes a
scene character's items and registers its world record, and compacts the
stash, employee and record lists. An
arena session may carry `arena_save::ArenaTables` (arsenal, stash, shop
stock, company, day, employees, merchant, hireables, tournament); the
player-data writer then emits them in the original's order, removes the
optional lists the writer omits, and leaves an old save's tables alone
when the session has none. The tables are filled from the live arena state
(`arena_tables::from_live`, `Session::capture_arena`): the arsenal's
twenty-five lists with the entry's two words packed into the wire entry's
third word, the hub stash, the shop's shelf, the company's name, emblem and
counters, the day's log, the staff (a fighter's kit or a specialist's
carried, hand and worn items, its world id or custom flag) and the
tournament's six forty-eight-byte roster rows. A load reads them back
(`arena_save::read_tables`) and `Session::apply_arena_tables` puts them into
the live company, staff, arsenal and stash; a save with none of the arena
chunks leaves the session without tables. What Exarust has no live model
for (the hireables, the tournament's field, prizes and events, the
merchant's stock type, the day's specials and daily tallies, the company's
second string and three words) keeps the chunk the save already held, and
the merchant's specialisation is the writer's constant −1.

The `CheckpointSave` script function's save side is
`Slots::checkpoint_from_script`: it runs only for the player with
checkpoints enabled, and writes the checkpoint only when the slot has none.
The script side already exists as `exa_script::builtins::Global::CheckpointSave`.
The load also clears a player record's race byte
(`CharacterRecord::reset_loaded_identity`), leaving sex and the kept
profession bits. The world clock (`world_clock`) continues across a save:
a load restores the header's seconds at its tick (the reader's setter) and
a save stamps the current world seconds back into the header (the writer's
read), through `Session::world_clock`/`Session::stamp_world_clock`; the
running game applies a restore from the `RestoreWorldClock` message, and a
session start sends zero so the elixir's world time starts at the load.
The scene flush (object database begin, player flush,
migration list, end), the grid environment's zero arrays and the landing
plan joining spawn placement and follower placement are in `save_level`;
the load's global reset (old-settings flag clear, default settings tag,
zero saved word and sun height) is
`save_chunk::load_reset` plus `SceneEnvironment::reset_for_load`.

The level-change bridge (`level_transition`) ties the transition together:
`export_for_save` writes the current level's section with the save's
exporter and the story's settings-chunk gate, and `plan` pairs it with the
landing (the player, the followers' own lifts, the migration list and the
re-attach state). The loader takes a `PendingLanding` and starts the player
there instead of the level's own start. The level change itself is in
process (`exa_world::level_lifecycle`), so a landing plan can now be handed
to a live load; the level loader still keeps its own scene model rather than
the exporter's, so exporting the current level into a save waits for the RFC
to export-scene conversion; see `docs/formats/SAVES.md` (Level transitions).
The scene side of the load is `save_scene`: the three entry points (save
section or `<locale>.rfc`, by name, arena) as plans and as `load_level`
(which imports the chosen copy), the import order with the
terrain probe and the node, sector and grid-environment passes, the game
scene's creation constants, the menu background scene's load, the GUI reset
and the player's node set-up and spawn. The migration pass now keeps only
the travelling characters: a repeated address is logged as a duplicate and
is neither flushed again nor placed. The fighters' kit passes are
`save_kit`: the preparation resolves own item references in hand, worn and
carried order (with the original's dead second-pair replacements), and the
equip pass instantiates them, rewrites the attached hand nodes and the
carried duplicates, and frees the temporary database. A load imports the
save's world character and object databases into their world tiers
(`Slots::world_characters`/`world_objects`). `level_transition` now also
has the transition's ordered flow: the teardown and flush, the destination
read (save section or archive), the import, the spawn matrix choice, the
story respawn, the grid arrays, the followers and the post-load pass, with
a missing destination aborting and a failed archive import disabling saves (a section read from the save always lands). The
running game now runs that flow around every in-process level change: the
viewer stores `level_transition::LiveTransition` (the archive copy, the
portal or default matrix, the abort for a level the pack lacks, and the
save-disable flag a save passes to `WorldExport::save_world`). The import
and spawn steps themselves still go through `exa_world`'s own loader.
The save's writers have the pieces the original exports: the world
databases (`slots::export_world_databases`: the character database
flushed and saved, the object world tier and its string library, each
only when it holds a record) and the player's placement
(`session::export_player_placement`: the body under the first saved
record in save mode, the record flush skipped while a respawn is pending;
`Session::placement` writes it first after the header). The arena's load
regenerates its lists (`arena_ids::regenerate_lists`: each employee's own
list, emptied for class 4, and the manager's take their loadout items by
the unique append, lose their special duplicates and are shelved with the
saved-item flag; every arsenal box is then deduplicated). No live save is
written yet: the running game has no live databases or export character to
hand to these writers.

Character creation (`create`): the record's gear (four hand items, its
worn list and its carried list), the mode filters (story background 0 keeps
braies only and clears the hands; the arena keeps the first two hands only
when its kit offers them and keeps braies only; modes whose 2 bit is set
clear the fist placeholder and the carried list), the underwear pass after
a filter (flagged on the entry plan; the rule itself is
`exa_character::record::CharacterData::ensure_underwear`), which records
the entry makes and marks as new, the story presets and
their settings keys (the first key per background, one shared character
key), the free-create and entry plans (recruits are not filtered, story
records lose their experience, every non-continued record is set to full
health — all four halves, pools and the save seed — and the underwear
pass follows the filter). The unknown background resets its gear and
starts the thaumaturge's two powers; a named background always wears its
template's gear. The chosen name, hands, worn and carried lists, skills,
full pools and seed are written into the spawned record. The starting
skills write Close Combat, Armour and Shields at five levels with their
first five techniques and leave the rest of the record; the skills page
caps them to the account's experience (a knight or proctor has at least
10000), keeps the first truncated slot's chosen technique, gives the
leftover to the skill in training up to its next level, and shows the
account's worth less the committed character's, converted back to
experience (each level of worth at weights 4, 5, …, only the first partial
level, quartered toward zero). The apparel and weapon kits come from the
original's seven tables by mode, skip what the record already holds (by
id, or by base id and the entry's four colour-layer bytes) and resolve
aliases (the record's `@` marker: the target takes the variant and the
three tint bytes) and new instances (the variant, the four layer bytes each
marked with five, and the second and third tint bytes as dyes where set).
The apparel panel is bound as the current equipment form as soon as it is
made, and the skills panel takes its flag only when made. The continue
page's label selects the save manager's first entry and its name box takes
that entry's name; an empty list clears both. The two pages name their tabs and final
button (RECRUIT, BEGIN, READY or CREATE), list the equipment panels per
mode, open the power tree for a record with powers, and the continue screen
toggles between a save and a checkpoint only while both exist, with its
list, details and name cut to fifteen characters. Appearance sliders settle
and copy with the original's epsilon and 64-bit debounce (a NaN distance
takes the write branch, as the unordered compare does), the face variant
steps through seven values, and sex, hair visibility, the voice sample
names (`VHM01_GruntS`, `VHMDA01_GruntS`), the pitch (worked in doubles),
the voice preview's 300-unit throttle (a 64-bit difference) and its
non-repeating random variant are in. The preview is modelled as the
original builds it: the 96-pixel icon list whose selection follows a hover
on the old last entry, the material applied to every listed preview item,
the dummy-head renderer (`dummyhead.rfc`, the options word `0x00a0a040`,
the entry height two above the icon size, one reference added and the
material/icon/release hooks) and the four-call character refresh. The
continue page's character view loads or polls its model, attaches the
carried item list, walks the figure's item passes, cuts the name to
fifteen bytes, labels CONTINUE or VERSUS, binds both item panels and
refreshes the look or the hair button. The power tree opens over the
creator with the deferred-close and commit callbacks, the name warning
keeps its `Character Name` / `Please name your character` box and code
0xc1, the reset-to-portal and delete-confirmation dialogs take the creator
as owner and centre at -1, and the icon sheet carries the 0x203 word, the
`0x0100_2008` flags, an icon size of eight GUI units and the four 96-pixel cells cut at
288, 192, 96 and 0 from the 96 by 384 sheet. The creator form's opening (cursor clear, task removal, three
input actions) and the equipment page's back switch (the first built of
the weapons, skills and lobby forms hidden, the skills slide 0x2d only when
it is the one hidden, the versus forms hidden, and a non-continued
character's appearance panels shown again with both tab words set to
0x110) are modelled, as
is the equipment page's layout: the preview frame, the 172-wide apparel
and weapon panels and the 216-wide skills panel. The look a record carries
(skin tone, hair style and colour, beards, normal-map layers, the look
seed's skin-map offsets and the per-limb leg scale) is
`exa_character::look`; the spawned body builds its `CharExt` hair with
`exa_character::hair`. The form's labels and page captions live in
`exa_ui::creation`. Of the form itself, the preview frame (180 units in,
24 down, the parent less 360 wide and 28 high), the two layouts (mode 12's
ten-element table without editors, every other mode's seven elements with
the two editor sub-forms) and the routing of a drop onto the preview (item
8 ignored; the mode's own list, game mode 8's apparel list, game mode
`0x1010`'s hook, else by the item's kinds: apparel `0x1000`, weapons
`0x4000`, and `0x8000` to the weapon list without the placing step) are
modelled. Painting the form and its controls, loading the model and the
sounds' samples, the object database behind the kit, record serialization,
the power tree's own screen and the running mode transitions are not in;
the running creator (`creator_screen`) and save chooser (`save_screen`)
call these rules where they have the caller (the voice pick goes through
`select_voice`), and the panels and continue texts are not wired yet. Rules are plain
functions with tests pinned to concrete numbers.

Story experience (`experience`): new item kinds, kills and explored map
cells (the 5 × 5 cells around the player on a 600-unit grid, the position
rounded down to its cell, a bitmap per
level) earn discovery points, once to the account pool and four times to
the player and each companion. Once a second a character converts a point
per 50 ms of its cost, `4 + skill levels`, into 2 points of its trained
skill; the player's paid cost levels the power in training, a companion's
level adds 10 combat skill and courage and picks its next technique; the
account pool converts at `(global / 5000 + 1) × 4` up to 25000, the paid
cost divided as a signed word when it levels the power in training. The
account experience's worth and its inverse, its four-byte setting value and
its set and get in the binary settings store, the level-start hook (the
player's carried experience becomes the pool and both clocks restart), the
six hook slots in registration order, the companions' list and the
discovery chunk of a save are in; the fighting estimate is the one shared
implementation in `exa_ai::estimate` (exa_game's wrappers call it for the
practice lesson), and the arena's lesson goes through the same skill lump.
The running story (`experience_live`) now drives them: the launch's
settings store fills `AccountExperience`, the first frame of play seeds the
pool and both clocks from the player's record (`StoryExperience::begin`),
each of the flow's once-a-second `Step::Discovery` passes (`DiscoveryTick`,
game clock) converts the account, the player and the companions
(`advance`) and writes the player's experience, skills and powers back to
its record (the record on the player's body; a story player always has
one), the body takes the `TechniqueWord` component at the level start and on
a rise, `apply_technique_word` puts it on the body's caster and adds the
record's perks to its fighter, and play's end writes a changed account
experience back through a `SettingsEntry`. The explored cells still need the level's
sector grid, which the running game does not hand the system yet, and no
pickup or kill calls `discover_item`/`discover_kill`, so the pool only
holds the carried experience. The shop (`trade`): a
first stock of 18 to 20, the
rotation by size, candidates by kind, listing, model and rank weighed by
rarity through the original's quick logarithm and exponential (duplicates
divided by 3, arms halved), the draw, new apparel and weapon copies whose
condition byte also feeds the description phrase's share (the low two bits
plus the signed 31-part quality field), the merchant's category and rank
options, and the transaction against the company's coins, both on the
shelf (`Company::restock`, `Company::settle`) and on the player's own
basket (`trade::apply_to_basket`). The shop window's own dialogue and the
weapon variant roll are not
in.

The force blast and torrent (`effects::{blast, torrent}`) are ported
simulations on the original's 83 Hz tick: the blast's sixteen chunks on
the icosahedral direction table, held within 45° of the facing, charging
in the hand (the charge five sixths of the tick, the drain paid each tick,
running out fizzles), released with their forward-pushed directions, an
alert to the characters within 18 m and "PowerUsed", then thrown with the
original's growth, trail, retention, wear and separation; the torrent's
sixteen streams fed from the hand, spawning, pulling together, splitting
(up to 24) and removed as the original removes them. Their contact
arithmetic (limb pushes, the character knock and its pin-gain quirk, rebound,
push-out, simulated points), their frames' sounds, colour bytes, texture
offsets and torrent light points, their geometry classes
(`effects::effect_geometry`, including the per-pass object matrix) and
multi-point lights (`effects::emissive`, their box moved by the node's
place) are in. `effects::ribbon` builds both draws: the blast's eight
trail samples a chunk, its smoothed 0–1 coordinates, the camera-facing
bend in the camera's space and the eight quads chained across chunks; the
torrent's doubled samples (the last sample repeated, cubic Hermite
midpoints at the half, each stream its own strip) with their eye-facing
normals, per-sample widths and the double-precision texture walk. The contact walks are in too: the level's static rebound, the
bolt's sampled walk on static bodies that zeroes a chunk's weight, the simulated object's
stir and the torrent's surface push. The engine's GL passes are what
remains out. A blast or bolt cast runs its
blast as an entity (`thaum::blast_live`): the cast motion and hook as the
original's start sets them, Exarust's own sphere test against the other
bodies' limbs, the level sweep that rebounds a chunk, the release's alert
to the caster's known characters, the instability counter and the concern
a struck character takes, and its ribbon drawn with the glow material. A barrier cast runs its dome as an entity
(`thaum::barrier_live`): cast motion 0x23 and the hook that ends it, the
focus it pays a tick, and its shell pushing other bodies' limb spheres out.
A bubble cast runs its dome where it was cast (`thaum::bubble_live`),
ending with its power. A wave cast runs its wave round the caster's body root
(`thaum::wave_live`): charging on the blast's hook, paying its drain each
tick, and thrown into other bodies' limbs. A torrent cast runs its streams from
the caster's right hand (`thaum::torrent_live`): charging and pouring on
its hook until the caster lets go or runs out, its points pushing other
bodies' limbs at the reach of ten, the level sweep, the knock, counter and concern
a struck character takes, its ribbon drawn with the glow material, and
leaving once its frame would draw nothing. Cast hooks answer by event
(`thaum::casting::CastHook`): the shell's and the bubble's refuse another
cast until their raise clip ends, and a caster's animator leaving its cast
action asks the hook with event 4 (`thaum::cast::call_motion_hooks`).

The Mind domain (`mind`) has its casts (`mind::casts`): the sixteen-row
table, the front end (Control Mind's lock, the shared pricing), the 0.3
focus gate, the record kinds (1–2 living, 4–5 undead), Calm, Enrage,
Confusion, Trust, Suspicion, Fear, Embolden, Loyalty, Control Mind, Raise
Dead and Command Dead (with the revival's fixed 0.2 decay), Sense Minds
and Mask Mind (instant and persistent) and Spirit Form, each handing its
work to the `thaum::power::mind` classes on the shared `CastBody`; the
restore of a saved Mind power (wired into `thaum::power::restore_persistent`)
and the combatant's binding helpers. What it shows (`mind::sight`): the
mind-target pick, the Sense Minds overlay and its tracker (mood weights,
dwell windows, slot reuse), the entry colours, the billboard, the glow, the
sphere's shaping and the spirit screen's start. `mind::draw` draws the entry
spheres with the original's sight program (`exa_render::mind_sight`), its
per-entry uniforms and the 256² animated noise picture; nothing creates an
anchor yet, so the pass idles. The spirit screen's 512² picture and noise
setup are in (`sight::spirit_screen_init`); its own quad and the casting AI
are not in.

The small effect nodes (`effects`) are in, on the original's game clock
in whole milliseconds: a level portal marker (the `0x3DD0E004` emitter
node, `_PortalPrt` in `uwportal01`) gets the soft blue 0.75 light of
500 cm 50 cm above it when its model loads (its `_PrtlPrt01` sparks are
not simulated), and a body stepping through one gets the seeded
`_PrtlDisp` dissolve burst (cells 6 cm apart, two fifths taken, each
particle's position moved halfway between the node's two frames,
jittered ±1.5 cm and pushed 1.5 % out from the body's centre, lifetime
41 ticks less up to a quarter) with `portaluse01` at 0.6. A marker
loaded into a world whose flags carry 0x10000 is replaced by a fresh one
at its place (the 4-byte payload read and unused), and a dissolve fills
its system's ring slot by slot (life, look, position, previous position)
up to the capacity and clears the burst flag; nothing in the game calls
the seeding, the ring or the import handler yet. A lit held item
carries a flame row from 30 to 115 cm with the `_FlameMat01` shader,
fading in by 0.02 a tick and out by 0.015 in double precision, its
ribbon's points closing a fifth of the gap to 20 cm above its ends, lit
by a 5.0 warm light of 800 cm whose softness walk is 0.87–1.15; its
ability word and fuel decide ignition, `FireSizzleA` plays at 0.3 and the
`Flaming` modifier is 0x10020000 (no Exarust item carries that ability
yet, so nothing lights one). The ribbon is built per frame: 16 sections of
8 vertices swept from the shifted pair of ends toward the lagging ones
(the cross-section turned toward the lag, the far face wobbled by 0.2 of
the line, that share taken in double), with the fixed texture steps, black/black/white/white corner
colours and 192 indices, in `FlameRow::ribbon_positions`/`ribbon_mesh`;
`exa_render::flame_row` is the `_FlameMat01` program (four texture reads,
the vertical falloff, the `Anim` roll and the 1 × 5 palette) [guess: the
record's blend state is additive]; with no lit item, neither has a live
caller yet. A displacement shell opens over 400 ms and
closes over 300 ms from its shown value, with the fade (1 − shown²)², the
texture offset, its fade plane and 900 over the squared distance as its
detail; a teleporter scan opens over 4800 ms, closes over 800 ms and
flashes its colour at three times the shown value when it loops; a bubble
grows over 1800 ms, rests 500 ms or holds and charges over 7000 ms
(flashing at 5600), then shrinks in 400 ms around a 120 cm sphere; the
domain wheel is a 280 px form of six domains (Mind, Force, Body, Energy,
Light, Displacement) clockwise from the top of a 76 px ring, with a
20.8 px hit, its hover, label, open ease, choice flash and confirm rules,
and its constructor's form (`effects::wheel::wheel_form`, 280 square, the
child form word 0x10), its three program objects (`WHEEL_PROGRAMS`, the
backdrop, sector and glow sources with their uniforms and the `Tex`
sampler on unit 0) and the flamelets sheet's mip chain its programs sample
trilinearly.
Rules of the Mind casts and sight and the two force effects:
[`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md) (Mind casts, Force blast
and torrent).
The reconstruction device starts the scan (`teleporters`), which builds
its node's `_ScanAura01` aura (184-byte grey block, state 0x9000, one
sub-object of usage 35044, the 0x20 material flag and the shared
`_ForceNoise` picture) and rolls its two creation draws; nothing
starts the shell's aura entity yet (its caller, the shell power's visual,
is not in); the power tree carries the wheel as its domain wheel child and
uses its hit test, confirm and open ease; the wheel's paint is
`exa_ui::wheel_draw`. Drawing the shell and scan mesh copies, the bubble
geometry, the portal particle simulation and the bubble's GI light values
is not in.

The force and environment effects of the same powers are in as plain
rules: `effects::rain` (4096 drops in a 600 cm box ahead of the camera,
the gust walk, the fall, the tilt-dependent streak, the ground wrap on the
effect's own generator, the colour and the two shaders kept as data; run
live with the rain loop, thunder and the outdoor arenas' wind bed by
`weather`; the original's per-sub-frame and reflected draw passes (the
set's 2° tilt scaled by `sqrt(field/70)`, placed at the box centre and
multiplied by each sub-frame's composite, with its `EyeVec`), the
program's uniforms and frame-buffer unit are modelled but not drawn),
`effects::portal_effect` (one ellipse particle per call, its previous and
current positions, the 3- and 5-bit look draws), `effects::fire_node`
(the `_FlameMat01` node's 800 cm 1/0.68/0.43 light and the colour ramp
its object conversion derives from a record byte), `effects::force_dome`
(the eight-ripple list, the cast sound, the Barrier's damped height,
800 ms grow window, drain, drawn radius, frame and keep-out, the Bubble's
1200 ms grow, break and 700 ms fall and its two keep-outs, the character
contact and balance drain, the double-sided meshes, the Bubble's draw
(`effectbarrier_v`/`effectbubble_f`, its seven uniforms, the eight ripple
points, the camera in its frame and its fade), and the node attach
(flag 0x10000, the owner back pointer and the frame hook) and destroy
(clear hook, light, material, object) that the live bubbles and barriers
run), `effects::force_wave`
(32 chunks 80 cm out, the charge, release, thrown step, trails and
lives), `effects::danger` (what the AI sees of a blast, a wave and a
barricade, and the steps away; the live blasts and waves carry a hazard a
sight pass can walk, and a standing barricade carries one for its wall),
`effects::emissive` (the seven GI lights' programs, extents, colours and
boxes, registered as GI shapes while the blast, torrent, teleporter
bubble, Barrier, force Bubble or barricade lives, with shape builders for
the cuboid and tendril), `effects::effect_geometry` (the geometry
flags, programs, uniforms, pass rows and fades) and `effects::vanish`
(hide, show, lend, phase hook and finish, and the displacement unit's
phase table with its stage, reveal, show and hide entries, not run live).
A cast starts its dome or wave,
but of these only the rain is drawn (the scene's weather runs it); the
vanish's caller is its power effect node, and no AI reads the hazards.

The per-character item effects (`item_effects`) are the body's effect
list and its two impact counters, the held charge (the HUD shown while it
is under its source), the hand-effect dispatch (electric, glow, flame), the
power-curve glow with its level step, the worn item's light and the apparel
effect with their 0.9/0.1 hysteresis, the skin material fade (the square of
the value, first match) and the focus-item power slot. The golem guardians
are in `exa_ai::golem`; `golem_alarm` runs their alarm zones live: the AI
manager's zone check for the player (past 3 cm from its last checked
point), the pass item search, and the ring that wakes the statue golems
within 800 cm of the zone (statue bit and byte cleared, `golem_act`, home
taken, the player their only contact at concern 10) with the level start's
reset, and the alarm return the story revive runs before its save
(`flow::Step::AlarmReturn`): while an alarm is armed, every awake golem
within 3000 cm of the knocked-out player goes home (Stand 2, put at its
home facing its home angle) and sleeps (statue bit and byte, statue rest,
the sleep scales), and when one did the player is put 200 cm past the
third-axis face of the `Alarm_` box holding the armed point, facing back
across it, asking for motion 0x1d. A spawned body (the scene bodies, the arena's
fighters and a level's placed characters) gets the list its worn items
register (`item_effects::WornEffects`, from the armour pass's
`exa_formats::encumbrance::WornSpecials`): a healing item on a body that
takes remedies heals its red health by 0.004 of the maximum each second of
game time and spends a charge; the golem suit's and gauntlets' ticks step
their bytes but have no drawn light or garment switch. A worn mind ward
also sets the caster's power-refusing bit and a focus charm its focus
regeneration bonus (`thaum::cast::PowerCaster::apply_worn`).

It already runs the character ticks of physical bodies, the player's last,
each body's whole tick before the next, in the original's stages
(`stages`: the driver and one module a stage; the rules in `limp`): its
condition, dying tick, falls and get-ups, fall check, then its bearing
with its get-up's first quarter, its clip's progress, walking and off its
feet by its condition (its feet read them), a collapsing body's bearing
saying so (posture; a body that dies there skips the later stages); the
original's movement decision from its character's locomotion request
(walk or stop, brake, the gates, the turn by the table of rates, the
alignment), then the original's locomotion routine (run blend, gait
columns, the walk's start key or resume, drive, clock step) or the
standing run blend's fall, and the torso twist toward the look (its
capsule carries the decision out at Exarust's speeds), its walk's
lowest ankle while it moves and its balance against the level
(movement); its clip (the capture of a start
from the body's physical turns, a plain start's lumbar and thorax keeping
half of the twist (`limp::Lean`), the original's clock rate for what the
body does (the base in its stance whatever its balance, twice it
kicking), the pose it holds while dazed, the sample taken in the
original's per-part pipeline (the Run clip overlaid on the Walk by
0.7 × the run blend, or by the blend with the sprint latch), the spine
turned by the torso twist (the lumbar half), the head turned toward the
look by its ramping weight (its anchors then on the physical thorax), the
thorax hunch on its pinned part's own placement after the twist (its
head and arms keep theirs), the foot pitched to the ground (a
20 cm toe, eased 0.01 rad a tick from the height the IK wrote) and the leg
IK's knee, hip and thigh turns folded into the clip chain (the standing
IK while it stands, the locomotion IK while it moves); the
clock of a locomoting body takes the locomotion's step; a get-up's clip ending into the stance writes the condition
word normal; sampler); its targets, the sampled parts on its character
frame, and its lift, its controller told to follow the frame (carry); a
walk's foot plant sends a `Footstep` at the character plus `3·speed + 4`
cm along its facing for the race's set (`movement::gait_footsteps`), and
a shock on a releasing race ends its step and stops its clip rate
(`Ragdoll::force_release`, from the energy arcs' `shock_fighters`), with
its next shock 500 ms on, no draw and no burn; a body asked to go limp keeps its motion unless
given a velocity (`limp`), and a level's placed dead are built limp from their
saved pose and registered at rest (`exa_physics::body::PlacedCorpse`: one
tick, then sleep; only a `--corpse` without a pose falls),
the original's falls, staggers and get-ups from last tick's balance
(`balance`, in the posture stage after the condition: upright the Defence
stance or Rebalance, leaning a stagger (a get-up jumped in a few keys) or
Fall Forward/Backward, lying still a get-up by the pelvis's front axis
with ±0.2 of hysteresis once its feet give support or 100 ticks after,
turned the way it rises, from the whole lying body, its feet's contact
states cleared; unsteady (0.6–0.8)
the Defence stance or Rebalance; in the movement stage the get-up picked
again under 40 % and the turn toward a fall; a get-up's extra energy in
the readiness stage; a fall leaves the controller, and
`exa_physics::body::Fallen` marks a body off its feet), the
player (`player` and `steering`: tank controls as the original's
behaviour writes them into the locomotion request — the release finishing
the stride, the quarter turn a second after any turn key, Shift's sprint
latch spent with the keys and kept by a far cursor, the look-ahead on the
navigation grid and the soft stop — whose steering starts only the gaits,
the chase camera on the drawn pelvis, combat facing, its combat stance and the
original's attack request each behaviour tick (press, hold, release, the
double click and Alt) through the attack executor, with the clips of the
moveset its hands pick; a dropped request releases the swing, a released
main-side attack returns to the stance once its stall is over, and the
body turns at the swing's rate while it attacks; `--trace` shows each
tick's locomotion),
the characters' footprints on the navigation grid, kept each fixed tick where their feet stand (an NPC unless it walks a planned route at 1 cm a tick or more, the AI tick's gate; the player unless it walks; `footprints`), every new fighter's voice as the character init builds it (the race's first template and the record's voice number, or the second and the undead voice when feral, the sex picking the name, the wound set by race; a fighter without a record speaks as a new human record; `character_voice`),
falls to death and the kill plane (`fall`: the fall check of a living
body in the falls branch, in its posture stage, its scream through the
character's own voice request (line 2, its heavy groan, the request's waits
and gains) and the player's pit sound, the
death fade's start, death and removal from the world; dead bodies below
−50 m taken once, the player's kept),
the world cursor's shape from the player's condition word (`cursor`), the
hand (`hand`: the left press named by `pick`, the drag of props with Ctrl
and of corpses through the solver's grab, the lift of a prop onto the item
cursor, a hinged door's leaf pulled by its link (slack 1 cm, strength
0.003, three passes; `exa_physics::body::DoorBody`) 100 cm from its origin
toward the cursor until the release, a lever turned by the wheel drag
(`exa_physics::body::LeverBody`: no link, its hand angle set each frame from
the cursor on the plane across its axle, or across its arm at 30 cm while
the view lies across the axle, clamped to the swing; let go out of 300 cm),
the double-click interaction pushing a clickable push surface
(`PlateBody::push`), the sliding-door drag's record until the release, a cloth
vertex's grab and its per-frame target (the pick near the vertex, else the
plane through the press point facing the camera), the reach under the cursor and
the refusal that keeps the movement request; `docs/PLAYER.md`, The hand),
the item cursor (`item_cursor`: the lift's turn ramp and rotation, the
cursor's per-sphere placement sweep and box refusal, its level placement
markers and hover word, its release and the GUI drop's branch and queue), the health
bars' values and the low-health heartbeat with its sound, started by a
blow's low-health check on the player's body (`exa_combat::Weakened`;
`hud`), the
held torch (`torch`, which fills its hand and holds it by the grip
table's one-handed or off-hand grip for the holder's size and race; its
tests pin the "Torch" clips' arm on the body model; `--floor-torch` lays
one on the floor for retail comparisons; a story start without a carried
torch lays the start's torch, item 0x1f, at its pose: `StoryTorch`), the
tip banner and the manual in play (`help_live`: `Step::Tip` shows the
prelude's line, the menu's MANUAL rows open `manualkm.fds` over the
covered menu), the debug light listing around
the player (`light_probe`, `--debug-lights`), the teleporter and reconstruction devices (`teleporters`: a `UseDevice`
request finds the device's crystal slot and the placed object on it; a
teleporter asks for the destination screen with its rows and level ids, a
reconstruction device puts the effects' scan on the player, waiting
1600 ms of game time and closing with the flash (a scan already on the
player makes the use start nothing), gives the scan's node the device's
aura (the `_ScanAura01` material, its 184-byte grey block, the state bits
0x9000, one sub-object of buffer usage 35044 and the shared `_ForceNoise`
picture), rolls the node's two creation draws in order, plays
`devicescna01` at 0.3, scales
the device's voxel lights by the glow pulse and raises the reconstruction
latch, writing the same word the story session's death handler reads
(and the scanned record identity beside it);
`SlotReleased` powers it down with `devicepwra01` at 0.1 and clears that
word; `EndSession` reconstructs a scanned player who dies in the story,
stripping them the first time (hand and worn data cleared, health just
under half). A teleporter use reads whether the destination screen is
already open and opens nothing more then. The destination screen and its
journey bubble, the grab and
use input that would send the requests, the level change, the corpse
record a strip leaves, the emissive half of the glow, the aura's shader
signature and the sound
playback are not in),
the per-tick body trace
(`trace`, with the foot contacts' states and anchors and the ankles,
ending with the condition word, the swing's rate, strength and state
word, and whether the fighter is parrying and with which clip), and
the debug tools ([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)):
the body overlay and joint readout (`body_overlay`, with the feet's
anchors and states), the joint trace
(`joint_trace`), the trace of contacts between body parts (`pair_trace`),
the planner probe (`path_probe`, `--debug-path`: a plan from the player's
feet to a point, drawn and logged) and the simulation controls (`sim_control`, with the scripted pauses at a
tick, a stretched link, the first blood jet, the player's first landed
blow or the first collapse).

The player behaviour's own rules (`player_behaviour`) are the aim point's
soft height lock toward the nearest of a target's bones (the reference
point moved toward that bone by its projected fraction, a NaN fraction
staying NaN, and the 80 cm pixel radius at the reference point's depth)
and its raised
surface, the gamepad aim's `250·|aim| + 50` cm point and its
`(0.4·AimSpeed + 0.1)²` ease, the 12 m contact list and its 12-entry
eviction, the strongest engaged enemy (the enemy's own concern for the
player over 2) and the late-swing check that takes its fighting estimate
as the practice's lesson, the auto-target (the visible opponent within 500 cm
maximising `(500 − d)(cos + 0.5)` from a best of zero), the look snap
strictly beyond 30° (the gap wrapped in double precision), the camera's
follow step (kept within 0.1π of the body's yaw, on the camera's side),
and the character nearest the cursor on screen. The
gamepad's aim runs through the eased world cursor each behaviour tick
(`player::AimCursor`, read by `player_combat`), and the combat auto-target
stores its opponent on the player body each tick for the aim point's bone
lock, which `player::cursor_ground` reads together with the raised surface
(the bone lock reads the opponent's head, belly and pelvis part origins, a
stand-in for the original's own three points). Each behaviour tick, before the attack request, `player_practice`
rebuilds the contact list, runs the late-swing check (an attack past 0.6
of its clock, once a swing: the swing's state word gains `0x100000` with an
engaged enemy in front, else `0x200000`) and one practice tick
(`exa_ai::combat::practise`), keeping the paid practice count on
`PlayerPractice` (the player has no record). The swing's release is
`exa_combat::swing::SwingState::release`. The remaining rules have no
caller yet: the look snap, the camera's follow step and the character
nearest the cursor. The combat mode word
(`player_behaviour::combat_mode`: down, dazed under 0.85 energy, switching
or casting idle, the right button's walk, the left button's attack) gates
the right button's walk to the cursor in combat mode
(`steering::Steering::combat_walk`), the chevrons showing while it walks.

The combat footwork (`footwork`) is the original's step machine on
Exarust's interim feet: the movement word (`steering::MoveWord`, the 200 ms
tap persistence and tap flag) feeds a dispatcher that starts a step toward
the movement goal (`steering::key_goal_direction`, `goal_stride`, the
line walk and its 400 cm² test, then the grid snap), re-lays a young step
at the tapped stride, or queues the next step on the body (`key_step`);
a landing takes the queue by `exa_physics::body::queued_step`, and the
word takes the queued word back. Its gate is the condition mask `0x82C1`,
0.7 energy and no swing; the capsule walks toward the step's target and
lands within 20·scale cm (or after a second, Exarust's own).

The controls' rules (`player_controls`) are the click's and the
double-click interaction's outcomes (a lock's key and first-unlock
reward, a container, an item use in hand, a button push, a corpse's scripts,
a character's talk condition, the player's inventory), the item-use values
(a pour's warmth and marks, the cleaning and repair kits' targets, their
sounds), the usable-section reader (the operative class, kind, flags,
value, target and name, the copy's wear and dirt bytes and the model's
material byte), the deferred interaction list the double click's
consumable joins 2100 ms on (done entries shift the rest down, keeping the
order), the deferred drop's spawn run (free, content error, or place at
the drop's −100000 cm limit), the
timed list that writes only while it lowers a section or arrives (the
apparel section first; when it has one whose byte is not written, the
weapon section is not looked at that frame); the reach's navigation region and walk-to point; the movement
request's drag modes, the controls' gates, the key permissions and the power
bar's 500 ms hold. A click on the player's own body shows the HUD bars for
5000 ms; the HUD's compass shows while the player's pack carries its usable
item, turns with the chase camera's backward axis and fades out behind
the open power tree (the tree's window fade); the HUD's power
marker (`exa_ui::hud_status`) pulses while the player's caster knows powers
and none is a new pick (the skill-ready alert stays off: no skill record
is kept for the player); a double click on it (with Ctrl in combat mode) opens the
existing inventory screen; a double click on a corpse runs its roles'
corpse scripts through `exa_script` (the corpse as actor 0, the player as
actor 1) and sends their effects on, and the other windows have no form and
the living character's talk condition is `exa_ai::roles`'s. The controls' scene-node words are
`control_nodes`, a tested model (the runtime hand holds physics bodies, not
scene nodes): the active and hold words, the grab switch and its release,
the body-bearing subtrees, the frame refresh and orthonormalisation and the
placement frame's bits. The item cursor (`item_cursor`) is live: a lift
without Ctrl grabs the prop (strength 1, Exarust's stand-in) and rides it
on the cursor; the movement inputs (the pad's raw move stick past a squared
0.1, else the move keys) turn it by `0.25·frame ms·0.0025` rad a frame,
ramping to `1.0` after 300 ms of turning and falling back after 500 ms of
quiet; each frame the cursor's ray, the held item left out, places it at
the level hit when that is strictly over the height the player stands at,
and at the ray's crossing of that height otherwise (a miss too; a climbing
ray places nothing, `lift_point`), then slides it along the ray
from 600 cm before that point (never behind the eye) to the nearest
contact of every collision sphere of every part, each carried with the
placed grab point (a sweep starting inside the level is no contact),
and a placement holds only when its rest-point box (280 cm up, −1000 cm
down) fits and the item stays in the hip's reach (`item_cursor::lift_frame`);
a frame that fails puts the item back at the frame of the last placement
that held (the lift frame before any) and counts as out of reach. A placement marker (a model node whose name starts
with `[`, registered by `exa_world::markers` and gathered each frame of a
lift) whose eight hexadecimal name characters give the placed object's id
or template, whose sphere the cursor's segment meets, takes the held node's
frame whole and gives 0x10, or 0x30 when its link byte reads `L`; a marker
the item would overlap at that frame is refused (the item's spheres against
the other bodies' part spheres, Exarust's stand-in for the original's
sphere-against-mesh query) and the search goes on with the next one; a free
placement gives 1. A release in reach puts the item down; a cancelled one
puts it at its last fitting placement, leaves it on its marker (frozen for
`L`, thawed by the next press) or puts it back at the frames the lift saved
(`item_cursor::release`, `hand::end_lift`); every let-go queues the held
node's box at the frame the lift saved for the navigation grid's refresh
(`exa_world::nav::NavMoved`, `hand::send_let_go`), so the floor the item
stood on is refreshed; a fall ends a lift the same way
(`item_cursor::{hover_word, snap_to_marker}`), as are the
GUI drop's branch (an entry, from a GUI, with a body), its spawned record
(both drag events alike), object spawner and next-frame queue, and the
put-down's scripts within 5 cm² (`player_controls::nearby_scripts`).
The right press's use start, its target class, the use release's dispatch
(a locked door unlocks with its key or rattles; a liquid on a character
or a character view; a kit on a world item with an id; an item window's
GUI use), the GUI use and the used item's removal, the point sound at 0.25
and pitch 1.0, and the door link's stop bit (`hand::door_drag_frame`) are
likewise rules; the use cursor's begin opens the GUI item a class-0 kind-0x11
section names and otherwise lifts the cursor only while play is on (state
word 0x100, shared cursor mode 0x40), and its end answers only the
end event (0x80, which the world view hands its callback before the next
mouse event; answer 0x81), runs the use only for the left button (the desk
word's bit 0, a guess), sets the player's action word to 4, clears the use
state and restores cursor mode 4 (`player_controls::{use_begin, use_end,
UseCursor}`). `use_cursor` runs it live: the item panels' double click and
its class (`panel_use_class`) call the use hook; a world object with usable
class 1, double-clicked unmarked, lifts the cursor with its record id and
kind and stays in the scene (`object_click`); the next press of any
button ends the cursor and is taken, and the dispatcher's door unlock is
applied (the other branches are reported in `UseReleased`). `gui_drop`
runs the GUI drop: a pack drag held over the world in reach is claimed,
run on the next frame through `run_spawn`, put where the cursor's ray meets
the level (or the far placement limit's height) and lifted onto the item
cursor once its body exists; the spawned node carries the record's template
word, usable and container sections and mechanism records as a placed
object does (`exa_world::dropped_object`). The deferred drop
ends with the state word at 0x2000 and keeps the body's frame out of the
controls when the word already had that bit. The wheel's set wraps its
grip angle into ±π (the single π
over the double one) before turning toward it; a NaN view/axle dot is not
under 0.2 and takes its first grip. The controls unit also
appends the Character and Spectate game modes in that order, each id the
table's new length plus two, its 16 name bytes and its hooks (Character the start and end of play, Spectate its own pair);
`GameModes` is the live table and registers them at start-up
(`register_game_modes`). The pad's
camera look takes the frame from the mouse-look capture, as the original's
gamepad branch returns before touching the pointer. The start and end of play set the AI's activation centre and
radius (`PlaySession`, which the player tick and the AI read); `PlayerControl`
is the player-control singleton's creation state (hotkey mask 31 and the two
constructor flags). The character doll's equipment (`equip`) follows the record's out set:
after a weapon-set swap the record's hands become the moveset's hands (the
main hand's weapon, a second weapon, the off hand's shield or its torch;
an off-hand weapon that also has a light section is the light). The GUI
character preview (`char_view`) turns the
figure toward the mouse: 0.7 of the pointer's arc-cosine yaw clamped to
±1.5 rad, a 5 % step under 1 rad and a clamped 0.05 rad above (the wrap
and the products in double), then the head's look eases 5 % a tick toward
the stored aim vector blended 40 % toward the cursor direction, built
with the engine's fast sine and cosine and its height zeroed
(`PreviewTurn`; the stored aim vector's source is unknown, so it is an
input). A door, wheel or cloth
press holds the hand until the release without keeping the movement
request.

## The running hub

`arena_match::ArenaMatch` keeps the transition screen (3 next round, 2 end)
and exposes the Continue and forfeit branches (`end_action`, `forfeit`);
`arena_live` draws the arena's end screen through the GUI path (`NEXT FIGHT`
on screen 3, `EXIT;` on screen 2), takes Enter and pointer input from it,
resolves a tournament's bracket on Continue, opens the arena pause page on
Escape, and settles its FORFEIT row as a return to the hub. The event weapon
row's variants travel with the in-process launch so later bracket fights draw their
weapons from the same row. The return writes each company body's health word
back into its record before the hub reopens: the manager's body is participant
0 and a roster member takes the id its plan was spawned with. The round's
experience pool (`arena::xp_pool`) multiplies `tier · 0.25 + 1` by the
template's experience and rounds once, takes the opponents' missing health over
their total maximum (every body in the Fray), rounds again and adds half for a
trainer; each company body's share is `round(pool · f32(damage / total))`, the
ratio divided in double. The match tick hands them out live at every round's
end (`arena_match::award_round_xp`): the bodies' combat scores
(`exa_combat::CombatScore`) are the damage, the match's slot the tier, and
each credited body's share goes to its record as a skill lump
(`career::grant_lump`, which copies the kit on a rank-up); the credited scores
are cleared, and a fighter who died that tick is off the books. A
tournament's next round names the watched fight's winner with the scene draw
over the match's bodies (red health as strength, collapsing and dead bodies
aside); its end and forfeit draw again, clear the brackets until one group is
left and name the first open event's winner from the surviving team
(`arena_session::finish_bracket`). The manage roster and the board's roster
draw their health gauges where the owner-drawn cell puts them
(`manage::health_marker`; the roster's four columns are Name, Rank, a 16-pixel
gap and Health). `ArenaCareer::reset` and `ArenaStores::reset` are the
original's single reset, run before an arena slot load reads its header.
`--arena-auto` sets the original's dormant auto-match switch: Continue runs
by itself and 4 s (40 timer ticks) after the return the hub replays the last
board match with the combatants after the manager, without fee or day limit
(`arena_hub_live::ArenaRematch`). `arena_hub_live` runs
`arena_hub`'s rules from the game. When a play level
named `arenahub` is in (or an arena session enters it), it calls `enter_hub`
once with the hub roll and the flow's alternative-music setting, writes the
track and its start offset through `exa_audio`'s level-music resources,
installs the click and session hooks and places the manager by
`arena_spawn::manager_plan` on the hub's known start. The session hook calls
`hub_fall` every frame; an answer ends the flow's session, and a wipe reaches
`slots::Slots::drops_after_death` when the host inserted `HubSlots`.
`slots::SlotList` is the slot manager's cached list (count −1 until
scanned, current slot −1, then 0 after a delete or a dropped death) with its
new-slot, select, touch, delete, death and current-slot load and save.
`save_live` runs the save unit in the game when the host gives a folder
(`--save-dir`): `--save-at` saves the session (the story at the loaded
level, or the arena company, staff, arsenal and stash as the arena tables)
to a new slot and never writes over a file that holds bytes; `--arena-slot`
makes the entered hub load that slot as the original's full load does
(tables applied, the reader's list regeneration with the object records'
classification, the world clock restored, the player record's race byte
cleared before the import, a converted old save's record repaired first,
the hub's permadeath slot set). The header's camera target and zoom are
stamped from the host's optional `SaveCamera` (not set by the running
game yet, so the defaults are written; both readers drop the pair),
saving is refused while the last level change reports it disabled, and a
checkpoint step saves before it copies. The level
section and the world databases are not written yet.

A left click that is not a drag sends `hand::WorldClick`. A static mesh
under a named scene node is that node (`hand::Hit::SceneNode`), the way the
original's scene pick returns the node; plain geometry stays
`hand::Hit::Level`. The hub reads the node's stored name bytes (the library
keeps `\xaaArenaShop` and `\xaaArenaMach` with their leading 0xAA) and
side, or a character's condition and merchant profession, calls
`hub_click`, and opens the answered window on `hub_windows`. The board comes from
`arena_board::open`, Manage Company from `manage_refresh`, and the trade
window from `shop_window` with the shop's or the merchant's stock. The board's
own paint list, the manage roster rows and the shop's panels and stock draw
through `gui_paint`. Manage Company, Company Records, Company Status, the
loadout form and the dismiss and yes/no dialogs are built from their
constructors' record tables (`hub_forms`, on `exa_ui::record_form`): the
records lay out at unit 4, rescale to the live unit and draw in the shown
palette and line skin; the pointer goes through the control messages and the
callbacks run the arena's rules (the manage refresh and End Day, the arsenal
form's rows, steps, slider, shelves and dismissal, the tourney forfeit).
`--open-hub-window <name>` opens one for a scripted screenshot. Escape or a
right-button release closes the top window. A
board Enter, Begin or Continue takes the event's entry fee from the purse
(a short purse stops at 0; nothing refuses the match), warns once when the
board's Begin enters the manager, drops the first unhealthy specialist,
plans the match through `arena_match`, the layout and the spawn plans, and
sends `arena_hub_live::ArenaMatchLaunch`, which the launcher keeps in
process and turns into a level change to the drawn arena, carrying the
event's rank and gear tier and each opponent's drawn weapon variant. The
play-mode spawner uses the rank and tier for the layout and holds the weapon
whose object id was drawn.

Opening the board or Manage Company makes the hub busy; closing either
clears it. The notice boards and the shop counter are static scenery with no
rigid body; their colliders remember the scene node they were built under
(`exa_physics::ColliderNodes`), so a click on `\xaaArenaShop` or
`\xaaArenaMach` reaches the hook. The shop counter's node holds only a
`_null` box, which the original never draws but picks; it is kept as a
hidden pick-only collider that nothing collides with. In `play arenahub`
the flow runs Exarust's practice stand-in once the player spawns; with the
hub loaded the hook takes that (and the menu) as the arena mode. The
`\xaaArenaMach` side is the x of the node's world bounding centre (its
`PartBounds` centre through the node's transform), and only a side above
zero opens the board. A click on the shop, or on the merchant, whose shelf
is empty builds its first stock (`shop_stock::LiveShopStock`: the resource
pack's `objdb.rdb` read once, each table entry's flags word as its kinds,
apparel and weapons stocked as new instance ids remembering their source):
the shop asks the default kinds and any rank, the merchant its stock type
and specialisation (`ArenaCareer::merchant_kinds`, `merchant_rank`, kept by
the save's merchant chunk). The shelf's names come from the item
catalogue, an instance under its source's name. The copies have no item
body yet (no condition or variant is rolled onto one). Not drawn: the
board's pictures, prize items and reward slots as images, the manage report
lines and controls, and the trade window's item icons and character
preview; a stock row taken into the buy basket and the basket's items
settling across the shelf are Exarust's list model in `ShopWindow`
(`take_stock_row`, `offer_item`, `confirm` through `Company::settle`), not
yet called by the live hub, whose baskets stay coin-only. The board's own close, day-end
and forfeit buttons are on its form host and report their actions; the hub
closes it through the window manager.
The hub's shopkeeper and specialist placements are `arena_spawn`'s plans;
once the manager stands, the hub asks the scene body generator for the
shopkeeper and one body per employed specialist.

## Arena match board bridge

`arena_board` turns a career and a generated daily board into
`exa_ui::match_board::Board`: the player is pool slot zero and the roster
follows in hire order; a member's rank is the six skill levels halved and
capped at Master; combatants get class zero and the staff roles classes above
the board's combatant limit (a guess); and each daily match instance becomes
an event record field for field: kind, flags, rank window, side size and
weapon row, the day slot in the rank column, the rolled rank in the tier
column, the slot-scaled secondary value as the entry fee and the cash as the
prize. It also carries the rival company names, the four seasons' reward ids
and the tournament weapon tiers. The hub keeps the board's close state and
restores its tab and list selections on the next opening.

The hardware configuration (`hw_config`) runs the settings screen: it holds
the live options, the saved set and the open page's draft; opens the page
(reading the settings file again), applies, cancels and runs Defaults per
page; writes the settings file's lines; rebinds a Controls row of the page's
copy of the bindings through the key-capture screen
(`exa_ui::modal_screens::ReadKeyScreen`: the key's scan code against the
page's saved-table rows, "ALREADY IN USE" for another row's code, Escape
closes, the middle and extra mouse buttons as `0x8004`–`0x8006`, the code
read back through the saved table's conversion); opens the shader warm-up
screen after an Apply that changes a shader option and steps it each frame
(`ShaderWarmScreen`); and drives the GPU
benchmark (a 1024² RGBA8 target, the original's 512² texture of one byte
value and sixteen full-screen quads through its own fragment program,
`exa_render::benchmark`, with the original's half-second warm-up and three
runs, four quads a warm-up frame and sixteen a measured one, all at one
depth so each is shaded); Defaults runs the detection (the benchmark, or
the Deck preset), and `steam_deck`/`video_memory_mb` are the Deck and video
memory probes. It pushes the live options into `exa_render::quality`
(super-sampling on or off, shadow-map sizes, MSAA), the audio buses, the
frame-rate readout and the window's mode, size and present mode
([`docs/UI.md`](../../docs/UI.md#settings-options)).

The Settings form (`settings_form`) is what opens that page: the menu's
SETTINGS rows (title, in-game and pause pages) open it over the menu, it is
drawn through the GUI draw path (`gui_paint`: the frame, buttons, labels,
check boxes, sliders, rules and the scrolled Controls page) and takes the
pointer, wheel and Escape while the menu waits under it. Its events go to
`HardwareConfig` (slider steps and labels, checkboxes, Defaults, Apply,
Cancel) and `BindingEditor` (a key cell opens the key-read screen); Apply
commits the bindings to the running game and sends `BindingsApplied`, which
the launcher saves to the store. Hiding the menu closes the form. The
key-read and warm-up screens draw their text over a black backdrop at their
fade. `--open-settings settings|controls` opens it once loaded.

The camera's navigation unit is `nav_camera`: the align routine's wrap
of the target yaw about the camera's, the engine's planar yaw, the orbit
rate (4.8 rad over the screen, in double precision) and pitch limits (0.8
rad to 1.5, 0.000125 steeper per zoom unit, the held pitch), the integer
frame clock (`FrameDriver`: the millisecond tick clock against the frame
stamp, at most nine ticks, the backlog dropped, the fractional position
computed in double), the camera placement with its height, distance and
pitch clamps, the free zoom's 0.2 % per wheel unit, the preset record's
numeric fields (the 59² view and size scales, the 59th roots of the
retention and the rate, the pitch bounds, the 10/10000 distance range and
both cameras' speeds, the distance raised to the smallest, and the
editor-only target scan for the `0xA9` nodes with its `0xAE` marker), the
controller's construction and tear-down (`NavController`: the 45° angle,
the scene and viewport back-links, the node and task hooks, the saved
viewport callback and the preset, with the clock set to the frame), the
cut-away segment and the point the probe centres on (the stored point,
the followed object's own point or its position), the kick offset and the
123-point jittered disc table; the preset's editor target scan keeps a node
only when its class byte is the editor's and its class word's low twelve
bits are one, its marker raising the extra flag. Its picking unit is
`nav_pick`: the screen ray's near-to-far form, the plane pick, the node
walk with the class, layer and option bits, the segment against each
bounding sphere and box, the nearest mesh within the best distance, the
ground pick with its 5 cm/1 mm grown sphere and top-of-sphere ask, and the
kinesis pick; both record the sphere-top crossing plus the distance the
mesh callback writes back, shortening the segment so a nearer surface met
later still wins; the ground-plane fallback is the miss. The cursor's aim plane and reach use them, and the
physics world answers as the mesh callback because Exarust keeps no CPU
triangles for a picked node; where a mesh does carry the engine's kd-tree,
`KdMesh` answers with its own front-face walk. Its free-camera drag and zoom handlers are `nav_drag`: the
absolute drag's yaw and arc-sine pitch with its pixel bounds and
quarter-turn release snap, the stepped wheel zoom (the chase camera's
wheel goes through it), the stepped gamepad zoom (`gamepad_zoom`: only a
stepped camera moves, then `gamepad::zoom_step`'s
move, which the pad drives live, then the pitch floor and held pitch), the vertical zoom slide, the zoom's cubed
pitch offset and the editor's grab drag and face-aligned target. The
editor's viewport attachment keeps the handler set a preset's flags add,
saves the viewport's wheel handler while its slot is empty, and restores
it while removing the free camera's press and move handlers when the
controller detaches; the scene editor's camera switch drives that state
as its followed record comes and goes. The free camera's own rules are
tested there too (no caller yet): its A/D/S/W movement vector (the right
and forward axes added and subtracted in key order, the controller's
vector added, the height zeroed and the vector normalised past 2⁻²³ when
asked), its follow tether (an axis past its limit held at
`sign(offset)·limit + anchor`, or the radial form that scales onto the
radius and keeps the height), the drift (change carried at 0.96, total at
0.99, 1e-7/1e-4 squared gates in double), the zoom spring (the target
within 0.001 snaps the shown zoom; otherwise `(target − motion)·rate −
last` held to its step, carried, damped by the double 0.998 or snapped
at 0.0002, never under half the range) and the two cut-away scene walks:
the lift out of every sector's meshes (sphere reject and oriented-box
reach through `exa_physics::intersect`, then each mesh's contacts through
`lift_out_of_geometry`) and the wall probe's segment walk (the visible
sectors, the flag words, `exa_physics::intersect::segment_sphere`, the
blocking word, then each face).

The arena's per-rank kits are in (`loadout`): `Member::loadouts` holds
six of the record's own loadouts (hands and worn items); the editor takes
the rank's kit, that rank's point limit (`arena::point_limit`) and the
shop's shelf as its arsenal, and its close writes the kit back in the
arena and match modes. The editor runs as a screen (`loadout_screen`):
`OpenLoadoutEditor` shows it, a press from the arsenal drops the item into
the kit and a press on the kit releases it, `LoadoutCloseRequest` closes it
(the arena and match closes save the kit, a champion stores its record)
and the paint lays the backdrop, the arsenal, the equipment and the
preview out at the constructor's rectangles through the GUI draw path. The
shelf marks the items a kit has equipped (`mark_stock`), and an item
dragged back to the arsenal asks the company arsenal's lend query only when
the form's own entry was not lent (`return_to_arsenal`: `Arsenal::release`,
which gives back the first lend of every box), then writes a new entry with
no flags for an item of another category that neither had lent; nothing
calls `return_to_arsenal` from the live screen yet. The
session save carries the company emblem (`arena_tables`) but not the six
per-rank kits; they start empty on load. The technique selector's bridge
(`techniques`) reads a
member's six skills (`skill_xp`, `techniques`) and its `xp` as the pool
and writes them back with the levels. The company emblem is an
`exa_ui::EmblemRecord` on `Company`. The teleporter's destination screen
(`destination`) opens on `OpenPortalScreen`, hit-tests and hovers
`PortalInput`, and a click closes it and starts a `TravelJourney`: the
bubble 98 cm above the teleporter (stepped by the journey, not by
`tick_bubbles`), the teleporter's `DeviceGlow`, the small crystal's
charge on its `PlacedItem`; `TravelToLevel` when the resting bubble is
told to shrink, and for the adapter the teleporter's single-row flag and
`JourneySave` when the bubble is freed. The screen's drawing, the player's
controls, the level change and the save are not in.

The skill screen's rules live in `exa_ui::skill`; its experience pass
takes the experience system's conversions (`experience::global_xp_worth`
and `global_xp_for_worth`) as `ExperienceRates`. No play mode opens the
screen yet; the character creator and the in-game menu still show only
stand-ins.

Item windows (`item_container`): the free-form panel's entry array with
its tint, clamps and layouts; the container record and character-list
codecs, both total and round-tripping; loads, saves, drops, drag ends,
rescaling, painting and the icon-alpha hit test; add-at-centre, replace,
held marks and the torn-out AR crystal trigger; the item card's request,
its 200 ms delay timer and its description box. The panel's icons are drawn through the GUI path (`item_panel_draw`): the
manager's atlas and cache are a resource, each visible entry is drawn at its
cell in the item panel's highlight program with its flags' tint, and the
inventory screen submits the layer over its pack panel. `item_icon_render`
fills the atlas from off-screen item renders, at most two icons a frame:
it loads the item's ground or factory model, draws it with a private camera
and sun on the off-screen layer (marked `Offscreen`, which the world's
camera, light, GI, shadow, fog and post systems skip), reads the image back, resolves it (the
alpha global's screen pass, else the Kaiser downsample and halo) and fills
the cache slot and atlas cell; a headless App, a blank readback and a lost
one fall back to the CPU paint of the same scene. When the interface scale
changes, the manager is rescaled: its atlas is rebuilt at forty units times
the new unit and every cached icon is drawn again (its half extents are
half its rendered size, rounded to even, plus the new unit). The card
window (`CardWindow`) keeps the 200 ms request, its toggle and its 0x100 ms
close fade on top of the card timer; `item_card` now draws it: a right
click on a panel entry builds the item's page from the catalogue's card
facts through the card's own page, value, tint, row and description rules
(the description first rewritten into the generated quality and condition
sentence from the executable's phrase tables, the apparel title taking its
variant word),
the card holds the panel manager's icon and fades on the popup's squared
curve, and leaving it starts the fade (from the moment it leaves, without
reversing a fade-in, and only once the card takes the pointer and is not
kept open). The card takes the pointer only once its shown flag is set; a
left press over it keeps it open and a right press over it fades it out,
reaching no panel under it. Closing the inventory or a loot
window queues its icon handles and the manager frees their references and
atlas cells. A body whose `Possessions` changed has its `Physical` stats'
encumbrance recomputed by `equip::refresh_encumbrance` from the worn
burdens, the held shield and its record's techniques (both the physical
and the fighter copy; a new body keeps its spawn value). A double click
classifies a container's open, a card close and a use; a popup's inner
panel and top are `open_container`'s, but no live container popup is drawn
yet. The inventory screen still has no bound item list.

The gamepad (`gamepad`) reproduces the original's Steam Input action sets on
Bevy's gamepad backend instead of Steam: the menu set and the interact and
combat gameplay sets with their digital and analog actions and manifest
names, a documented default binding table, the set table with its gameplay
layer, the menu reader's 0.25 dead zone, 0.1 analog bits and scaled
cursors, 400/150/30 ms repeats, three-millisecond set priming, the
directional control pick at the GUI unit, the focus stack, and the gameplay
reader's camera-relative move, aim, the look routine at the orbit rate, the
stepped zoom, the power-bar and dialogue step modes, the 500 ms power-tree
hold, the command word and rumble. The player reads it for exploration
movement (`pad_keys`), combat steps (its stick vector is the footwork's
analog word once its squared length is over 0.01), aim, attack and alternate attack, crouch,
sprint, weapon swap, combat mode and the chase camera's look and zoom; the
controls page's `InputMode` drives its controller flag. There is no Steam
library, no binding panel and no floating
gamepad keyboard ([`docs/PLAYER.md`](../../docs/PLAYER.md#gamepad)).

## Debug commands

`commands` is the original's named debug-command registry: units register
a name and a handler (`cshd` from the shader programs, `msm` from the mesh
subdivider), a line splits at its first space into the command and its
argument, the command is matched with ASCII case ignored, and the first
registered match runs. The settings file's bound `KeyDef` hotkeys
(`settings_ini::Hotkey`) become `KeyCommands` and run their lines from
`CommandQueue`; running one line per frame is a guess (the code that
advances a binding's list was not traced). `exarust` registers `cshd`
(reload the 12 embedded render shaders) and `msm`, whose handler only logs
(the model view's `--msm` does the subdivision).

## In-game screens

`gui_screens` runs the in-game unit's screens for the player: the I and J
hotkeys (through `hotkeys`, so their permissions apply) send a
`ScreenRequest`, and the inventory and journal open as forms on an
`exa_ui::gui_core::GuiTask`, so their focus follows the window manager.
The inventory toggles when active, is activated otherwise, sits at its
(8, 8) or remembered place and is registered as an open container; it is
laid out at the live GUI unit (`InventoryLayout`): the constructor's
design at unit 4 (244 × 644, border 8, the character view 228 × 384 at the
client's top, the pack 228 × 180 at its bottom, the records between them)
rescaled by `exa_ui::gui_rescale::rescale_form`, so at unit 8 the form is
488 × 1288 at (16, 16) and its pack and records follow (the pack's hit
rectangle in `panel_live` too). The loot window (`ScreenLayout::loot`: 484 ×
876 at x 800 at unit 8, its panels 452 wide) and the journal
(`ScreenLayout::journal`: 484 × 1028, its text box 456 × 824) are rescaled
the same way, their 2-pixel borders kept. The character view's panel frame
is drawn and `doll_view` draws the player's dressed figure inside it while
the inventory is open: a copy of the player's body on a render layer of
its own, drawn by an off-screen camera at the view's pose into an image
twice the view's size, shown by a UI node under the GUI canvas (the
inventory's painter clears the canvas over the view and the camera clears
to the colour it had there); the figure turns toward the pointer
(`char_view`). The inventory's second record is the rating-dots control,
showing ten times the player's encumbrance with the dot sheet's whole and
half dots. The view draws no preview bars (only character creation and the
arena switch them on). The
journal keeps its session entries and stores its text on close. Escape
closes the focused screen (Exarust's). The rules are `exa_ui::game_gui`'s.
The screens draw through `gui_paint` and `exa_ui::gui_draw` with the words
their constructors write: the frame (no title bar, as no shipped form sets
the title bit), the inventory's 228 × 180 pack panel, and the control
records laid out by the two-pass layout (the inventory's Encumbrance label
and Primary button, the journal's New and Erase buttons) with captions
placed by the skin's own rule, and the journal's selected entry through the
text box. Every window painted through `gui_paint` draws with the line
skin's palette as a shown form's fade scales it (`gui_paint::palette`, form
group 170, other groups 125) and its buttons with the line skin's atlas
frame (`gui_paint::paint_line_button`, shared with the Settings form); the
hub's board and Manage Company fade in over 256 ticks (`hub_windows::HubFade`)
and the item card draws at its squared fade (`docs/UI.md`). A dead body's double click in the world opens the body's loot
window (400, 36; 244 × 440) with its pack and equipment panels; it joins
the open-container registry under the body's tagged id and closes with the
focused screen. The window's show rule reads the body's record flag word, its
profession word (`& 0xff00 == 0x300`, race 3, refuses), and
`body_inventory` fills its panels from the body's `Possessions`: the pack
through the character list's own saved places, the equipment panel with the
worn items and the shields in the hands (held weapons are not listed)
through the worn/scatter pass, its generator started from `DOLL_SEED` as a
stand-in for the engine's shared one. The player's inventory pack is its carried list the same way.
`panel_live` runs a left-button drag between open panels with the item
panel's press, accept, drop and drag-end rules; a drop on another pack moves
the possession entries, a drop on the same panel only repositions, and a
drop on a body's equipment panel or outside every panel is refused (a loot
window has no running character view and there is no GUI-to-world drop
yet). A drop on the inventory's figure is `doll_view`'s: apparel is worn
through the equip conflict pass (displaced pieces back to the pack, the
braies dropped, the outfit re-dressed), a weapon goes into the hands by
the take-in plan and a shield into the off hand, the player's hands and
held weapon following. No drag leaves the
player's pack while the player's body swaps its weapon sets (a swap
requested or its hand plan pending), the inventory's drag permission.
Taking a worn item off a corpse does not re-dress its mesh.
`object_click` takes the hand's clicks on anything but a character
(`ObjectClick`). The single click (outside combat mode) on a placed object
clears its first-touch bit, runs a device's use (`player_controls::device_use`,
its clock in `DeviceClock`) and then, by `player_controls::click`, sends a
`item_card::WorldCard` (the card at once, or on the panel's 200 ms card
timer; a world object's card is not closed with the last panel; the card
reads the scripted pointer of `--cursor` as well as the mouse, and logs at
debug level where it opens, or why it has none) or has the
narrator say the device's description through
`exa_script::Conversations` (`narrate_text`, 1000 ms for device kind 3,
100 ms otherwise). A point on plain level geometry is looked up among the
placed objects by their node box (a placeholder's 50 cm half box, else the
model's root node bounds; sphere test, then at most 0.1 cm² off the box);
the single click then only narrates a used device, and the double click
uses the object found (an Exarust stand-in: the original's pick always
returns the object's node). The double click's usable class 0 object leaves
the scene (despawned, its static colliders removed through
`ColliderNodes::take_under`) with its use sound (`exa_audio::PointSounded`,
volume 0.25, pitch 1); a consumable's doors open 2100 ms later.
A placed chest, barrel or bag opens in its container popup from the
interaction's double click (with Ctrl held in combat mode, in reach): the
object's own use runs first (`player_controls::double_click_object`, the
usable section's consume, take or push; the class-1 hand item stays in the
scene and lifts the use cursor, which ends the interaction there) and a
record with a container section then sends a
`ContainerRequest`. `open_requested_containers` cancels the item card's
pending request (a shown card fades out) and opens the popup as the
original's opener does: a popup already open on the container comes
forward; otherwise the container joins the open-container registry under
its tagged id, its panel is `2 · 4 + cells · unit` each way, six units plus
the icon area (twice the container icon's half height, at most the icon
size) below the popup's client top, the popup `2 · 8 + panel` wide and
`2 · unit + panel top + 8 + panel` tall, centred across on the pointer
(kept inside the screen's width) with its top eight units above its height
over the pointer (at least one unit down), and the panel loads the record's
children (positions in 1/80 unit). The popup paints its frame with the container
constructor's words (attributes `0x830023`, centred-title style `0x3000`)
in the shown form's palette (`exa_ui::line_skin::shown_palette(1.0)`),
the container's icon by the plain card's placement and the panel's entries
(a level's own records resolve their icon model and bulk through
`ItemCatalogue::set_level_items`, fed from `exa_world::LevelLocalItems` by
`register_level_items`; without it every level-local item, the chests' own
and 128 of exanima01's 307 container children, had no icon);
`panel_live` drags between it and the other panels (its side has no
possession list), and a right click, Escape or the screen close shuts it:
the panel saves its entries back into the object's container section
(`exa_world::ObjectSections`, the session's copy of the record) and its
icons are released. A consumable (usable class 0, kind 1) the double
click uses joins the controls' deferred list (`player_controls::PendingUses`)
with its section's target word, due 2100 ms on; `run_pending_uses` hands
each due entry, in order, to `exa_world::mechanisms::OpenDoorsUnder`.
When a level has loaded, `container_fill` replaces the placed containers'
random-table children (every shipped chest's children are "DropTable"
records) with the rows their tables give, through
`exa_items::randomise::Fill` (292 children in exanima01; the chest of
drawers 0x80000054 gets boots twice, a shirt, a belt and trousers): a
picked child takes the row's item with zero flags, an empty slot id 0
with `0x100`. Stand-ins: the row's item id is used without the
instance copy and condition word, only placed containers are walked
and the generator starts from `FILL_SEED`. Opening a popup logs each
child's id, flags, model, icon slot, whether its icon has rendered and its
pixel rect at debug level. `item_icon_render` keeps one GPU icon job in
flight at a time (`start_budget`): every job's scene and sun sit on the one
icon layer, and two at once framed the second item on the first's model. An apparel item's run-time (`&`) materials are painted from the
record's appearance block before its scene is baked, as the original's
streamed icon paints the item first: the paint job runs on the async
compute pool (`IconPainting`) and the icon waits for its material, so the
chest's clothes no longer render as the unpainted shade map (white). A GPU icon also waits (at most `TEXTURE_WAIT` frames) until
every texture its materials bind is in the image store and is not the
streamed load's 1 × 1 placeholder (`scene_textures_ready`), so a model with
its own materials is not baked over the placeholder. An empty readback (the
camera drew nothing: the session's first icon waits on its pipelines) is
asked again up to `READBACK_RETRIES` times before the CPU's flat stand-in
fills the slot; each icon logs at debug level whether it was painted or
plain and whether the GPU or the stand-in made it.
`--open-container FRAME` (Exarust's debug helper) opens
the container nearest the player that many frames after the level load ends.
`npc_search` hosts the original's follow state and the companion's search
in the running game: a level brain whose roles choose the follow follows the
player (past its 400 cm band a route through the navigation grid on its own
planning turn; inside the personal stop it stands) and, inside the band,
runs its search once a body tick over the level's dead bodies (their 600 cm
cells, their possessions as packs; reachable is the level's sight test, a
path the grid's route), carrying out each take, wear or wield on both sides'
`Possessions` and logging comments (no dialogue engine). The items' values
come from the game's object database; level-local ids, containers and loose
items are not searched, nothing is re-dressed and no kneel clip plays.
`--searcher <record id>` gives a level character the Explorer role with its
Activity variable set to the follow (its log lines start `search:`).
`role_scripts` runs the encounter scripts an AI's target pass reports: the
observer's roles' encounter entries for anyone, for the partner in its
locale, or for one of the partner's roles, with the observer as actor 0 and
the partner as actor 1 through `exa_script` (`Conversations::with_scene`),
and the effects go out as `ScriptEffect`; the meeting time is renewed on
the AI's relation and the cast's record (`LastEnc`) before they run. The world's own press is held while
the pointer is over an open screen. The journal's entry list and the
controls' callbacks are not wired; the text keeps the level
font's metrics and samples the base font's atlas while the Rayform faces
are not shipped. The skills key's screen and the power tree are not opened.

## Dialogue in play

`dialogue_live` lets the player talk. On the frame the Talk key is pressed
with no dialogue open it finds the character to talk to as the original's
talk routine does: of the player's nearby characters (its room and the
rooms behind its exits, seen, within 30 m, the sixteen nearest), not
collapsing or dead, the one whose chest projects nearest the cursor (no
range on screen, no facing test); `hotkeys` then sends `Talk` for it. The
reply panel (`exa_ui::dialogue_window`) follows the engine's panel, the
pointer hovers its bands and a released left click picks one
(`PickReply`); the pointer over it counts as over the GUI. After the open
screens the bubbles are placed and drawn (a character's over its body
position lifted 120 cm) and then the panel, both in the main GUI font of
the window's level. `script_cast` makes the player body's character id
the session's player and keeps a level character sharing that id out of
the cast. A new story in `exanima01` places the unknown background's
companion, Derrin (`xac_derrin`), at the story start's companion point
(`exarust`'s viewer), so the first level has someone to talk to.

## Scope

- App states (boot → menu → loading → in-game).
- Arena: companies, fighters, matches, rewards; Story: levels Underworld 0..8; Practice/sandbox.
- Save games in the original's package and player-data shape, with our own
  payload for the session state.

## Game flow

The Energy domain's Shock creator builds its charge object with the cast
flag 1, the base cost, the clip's charge, the hand arcs (20 tendrils of 5
segments at −15, −5 cm), the `thmnrg_chrga` and `thmnrg_shock` sounds, the
`0x401` active-power membership and the player-controls call for a player
owner (`exa_game::thaum::energy::shock_power`, `ShockPower::created`). The
shock weapons creator arms every held weapon with its 0x301 arcs (40
tendrils of 4, scale 0.75, light 0.02, `electricity03` at 0.15;
`armed_weapon_arcs`, not yet spawned), holds the cost and joins `0x2040b`
with the weapon-change hook; a put-away weapon's
node takes its end flag and loses its hook. A blade strike shocks the
struck character by 0.01 and queues the spark burst at the contact point
in the struck body's frame (`blade_strike`, `spark_at`; no caller yet).
Shock's release spawns its burst at the hand (`arcs::shock_burst`: ten
tendrils of fourteen points, growth 0.25, lift 0.5, light 1 and a shock
strength of 0.05; its aim at the controller's target is not passed).

`flow` is the original's game-main flow: boot into the title, the mode
switch (practice, story, arena, scripted modes), the session starters
(story from a background with its roles, record words, pose, light and
Derrin, practice from the ladder with its role, the story and arena
continues, the arena hub), the transition screen's actions at black, the
per-frame dialogue and session hook (with the wake-up's whole animator
write: flags, blend, move and turn factors and the wake clock; the first
animation ends on the 0x20 bit of the player's state flags,
`exa_combat::StateFlags`, not on the condition word), the story's
wake-up, knockout and revival, death and the end-of-session menu, level
exits with their save and checkpoint, the level-load finish, the story
end-menu exit, the story respawn at `exanima05`'s portal, the application
quit's autosave, the menu form routines (show, hide, the help flag, the title logo),
the return to the menu form, and the pause. The story start records the
level's start point, the pre-load view and the loader's twelve-byte start
record; a level exit that makes the sixth level's checkpoint stamps the
level's time; the next practice fight clears the condition word; the death
screen's CONTINUE writes the player back with its 0x100 record flag; and the
boot applies the settings store's expert and background unlock words.
The running game holds what follows a level load in a step batch
(`AfterLoad`) until the load has shown and ended, the player stands in the
level and its placement is applied, as the original runs it after its
synchronous load; the story start's record writes go on the record the
load spawns the player from, and the spawned player takes the spawn word,
roles, gear and start motion (`StartRecord`).
The player controls' combat-mode setter is its own step (`CombatMode`):
the practice start and each next fight turn combat mode on, the revive off.
A new story's unknown background and the story respawn lay the player down
to wake (`KnockOut`: down, flags 0x30, tone and stance 0, the wake clock
3000 ms or 0 ms on), the revive raises the dead body (`ReviveBody`, the
original's dead-body revive: yellow health half the red) and holds its
wake, and `exa_combat::condition::track_wake` runs the wake tick live on a
body with a knock-out while its flags hold 0x20, which clears that bit once
the body is ready (so the story's first wake-up animation ends when the
body is up). The story respawn and the story start attach the controls
again (`Controls(true)`), which gives them back after a story death. A
camera shot turns the chase camera to the shot's bearing
(`ChaseCamera::face_shot`). The rules return named steps;
`FlowPlugin` runs them on the 12 ms game clock, follows a play level's
player as a sandbox practice-mode session (without the practice round
hook), turns a teleporter's journey into a
level exit, and hides the session hook while F1's in-game page is up (the
menu screen calls `menu_opened`/`menu_closed`). Every producer queues its
steps in `FlowSteps`; the app applies the menu, end-menu, title-logo,
game-view, controls, health and music ones, and turns `LoadLevel` and
`TeardownWorld` into `exa_world::level_lifecycle` requests. The viewer loads
the destination through the same staged `LevelLoad` as a start-up load, and
`PlacePlayer` is carried across it in `level_change::PlayerPlacement`.
The transition screen runs live (`run_transition`, `transition_screen`):
`exa_ui::transition`'s state machine on the real clock, the action at
black, the level exit's long hold from its fade bit, each caller's music
fade (the track's gain, its stop at black and the screen's audio fade), the
world fade reset, a black hold while a level change it asked for loads, and
its alpha drawn as a black form over the game.
`BootToMenu` keeps the launcher from covering the level or the splash with
the boot title; the splash path shows it at the hand-over. F1 opens the
in-game page while a session runs. A practice start and each next fight
place the player at the spawn table's point (x 0, z 500) with only its
vertical replaced by the body's height. A requested motion (`RequestAction`:
the story start's and the respawn's get-up, the delayed wake, the practice
start and each next fight) is the body's unforced action switch: a body
already in that action keeps it, otherwise the clip starts on the player's
animator through the same rule the cast pipeline uses
(`exa_anim::Motions::start_requested`: the numbered clip's name, looping
when the clip loops, else once back into the combat or idle action, with no
blend). In the practice mode only, the end menu's practice branch shows tip 5 on a
novice win and writes the expert-ladder settings word once (the flow keeps
the original's written flag, seeded at boot from the stored word); the
step becomes a `SettingsEntry` message that the binary writes and flushes
into the user's store, as the original's settings writer does, so a later
launch greys EXPERT differently. The camera shots (offsets added to the framed point)
with their cursor warps, the spawn turns (the engine's angle builder), the
exit-trigger nudge, the practice scene filter, the practice and menu music,
the unlock words and the save managers' signatures are pinned by tests.
Rules: `docs/UI.md` (The game flow).

`splash` is the start-up splash screen: the black backdrop, the one-second
renderer warm-up, the first logo fading in from black over 600 ms and held
to two seconds, the second logo with `BME Critters`, its 200 ms hold and
300 ms fade to black, the hand-over to the main menu, the one-second wait
and the one-second fade of the backdrop, with the logos centred at
`round(H/135)·0.125`, on the original's two clocks. `SplashPlugin` draws it
when `SplashAtStart` is set (`exarust --splash`) and opens the main menu at
the hand-over.

`game_event` is the story scripts' `GameEvent`: memory fragments (the head
pain action, the body's voice request on line 2, the post-process flash,
the `XU_Memory` narrator topic after 2.5 s, a fragment bit and a potential
point for a player with the memory role), the raise-dead lines (the power
grant for Raise Dead picks the successful one), Corlian's topic, the three
domain awakenings (the potential plus ten less the fragments seen, every
fragment marked seen, the voice, wail and swing, the mind domain for the
second and third codes, the force domain for the first and third, and the
class word) and the background-unlock bit. The potential is the record's
powers-chunk float and the class word the record word after the experience.
`flow` consumes the calls (`Flow::game_event`): the unlock sets bit 4 in the
background word and writes the whole word to the settings store once, and
the story state keeps the fragments seen and the background (a new story
clears the fragments and keeps its background; a story save writes both and
a load restores them). `game_event_live` reads the live player (its roles,
record and voice) and applies the rest in the handler's order: the requested
action through the unforced action switch, the voice line through the pain
voice's path, the narrator topics through `exa_script::Narrate`, the
potential and class word on the record, the Raise Dead grant on the
record's known powers (and the body's caster), and the sounds as
`PointSounded` with the handler's flags. The flash, the witness event and
the awakenings' body effects are logged, not applied. `--game-event
CODE@FRAME,...` raises codes after the load (a debug aid).

The main menu screen's F1 takes the flow's mode, phase and page state, and
its in-game page in a session; while the page is open the flow's dialogue
and session hook are held, as the original's frame task does. The page is
painted through the GUI draw path with the menu font's sheet, and the
selected row runs `menu_text` through `exa_ui::skin::SkinMode::MenuText`.
`end_menu_screen` draws the end-of-session menu over the game the same way;
its rows follow the end-session dispatch (`exa_ui::menu::dispatch_end_session`:
mode, then row, then menu number): the next practice fight, practice's RESTART back to the menu form,
the story respawn or exit, and the returns to the title behind the
transition. A scripted mode the `EndHandlers` table holds hands the clicked
row to its registered handler (`ScriptedEnd`) and leaves the menu to the
script, as the original's registered table does; nothing registers one until
the script crate runs. The title's BEGIN opens the story's background picker and
the creator (`creator_screen`), as do the practice rows, NEW CAREER and
VERSUS; the saves' character screen (CONTINUE) and the manual are not
wired; SETTINGS opens the Settings form (`settings_form`).

## Main menu screen

`menu_screen` is the running app's main menu, opened and closed with F1 (the
original opens it from its session flow). It routes the pointer through an
`exa_ui::gui_core::GuiTask` whose one form is the menu, so the hover and
focus follow the window manager's rules, and uses `menu_pages`' page table,
hit test, paint pass and click handler: the pointer selects the row under it
and Up and Down move the selection, a press marks the row and its release
over it runs the command, and Enter runs the selected row. The title page's
EXIT quits; page changes, the resume and the exits to the title are the
screen's own; the session work behind BEGIN, CONTINUE, NEW CAREER,
PRACTICE, VERSUS and MANUAL is named, not performed; SETTINGS opens the
Settings form over the menu, which then takes the input until it closes. Escape goes
back from the arena and practice pages and closes the screen elsewhere. The
rows are measured and drawn with the menu font's record (`titlefont.rfi`),
the selected one through the menu text program, and the title page with
`maintitle01.rfi`'s art at the row scale and its (230, 230, 230, 153)
tint; a missing sheet falls back to the base font through the same draw
path. A newly hovered row plays `MenuSel` (at most once per 200 ms) and a
press or Enter plays `MenuClick`, both one-shots of the audio environment
at gain 0.06 with the interface flags 0x4060 and a 0.8–1.2 pitch drawn
from the run-time library's Mersenne twister.

## The trade window and the camera's wall probe

`shop_window` is the arena shop's `TShopWindow`: the 0x36c × 0x244 page
titled "Trade" with the character preview and the player's pack under it,
the quartered sell basket, the full-price buy basket and the stock at the
right edge. Each basket's callback re-sums its own total. The bottom row
reads "Pay:" or "Receive:", warns "You cannot afford this transaction." and
disables the Trade button on a short purse, or when paying with both
baskets empty; the merchant flag picks the label rows. A take-off from the
character view lands in the pack, a right release hides the window and the
close releases the preview and the trade-mode bytes. The opener's layout
binds the stock list (the shop's or the merchant's) to the stock panel and
takes the list's own number when one is supplied, writes the preview
camera's turn factor 0.05 and view word 0x1000, and re-runs when an item is
picked in the preview. A basket row now carries its catalogue item beside
its price word: a taken stock row leaves the bound list and joins the buy
basket with its price, an offered carried item joins the sell basket, and
the confirmation moves the bought items off the company's shelf and the
sold ones onto it through the transaction settle.

`nav_camera` has the stepped camera's per-tick wall probe: from 50 cm under
the followed point along the camera's yaw, alternately a 30 cm probe at the
present step (a hit climbs, to four) and a 20 cm one at the step below (a
miss comes down), over 38.5 m, the tilt eased in double toward −0.15 rad per
step and added to the axis pitch, held to −1.54. `ChaseCamera` runs it every
tick, but nothing answers it in the live game: Exarust's level colliders
include ceilings, which the original's mesh filter leaves out, so the tilt
stays 0. The pitch ease (in double, only over a gap above 2e-6), the eye
blend and the field of view are the original's.

`scene_editor` is a model of the scene runner's task (`RF_SceneEditor`,
`TSceneTaskInt`): nothing draws from it yet and no other module uses it. It
holds the camera-mode switch (a plain mode word; modes from 3 up run a leave
callback, a controller is created once and a record sets or drops its
target), the frame step (the step is `now - base` less the last stamp as a
signed 64-bit word; a step over 50/3 ms, compared in double, stores the f32
`0x41855555` and a scale of the cap over the step, otherwise the scale is 1
and a negative step becomes 0; the audio mark subtracts half the step,
rounded to even, from the audio clock when the viewport count is positive),
the task tick's repaint and message rules (a cleared flag posts to the GUI
task and repaints, all of the low three bits across the two message words
repaint, message 2 follows while the pending count is under its limit and the
primary word has none of `0xf0`), the maximised-viewport count, the
run-scene set-up (viewport rectangle, flags and 15-character title, the
renderer minimum that only rises, the camera record, the weather option bits
and the audio listener with its global fallback, the task's copy of the
seven-word camera record (the default record itself when the options name
none), the option bytes for physics (start the simulation, or refresh an
existing one's grid size), the maximised viewport and the view toggle (read
as a show flag, a guess), the listener pose step (the camera's two vectors copied into the
listener, and the listener's point set to the half-way blend from the camera
to the object it follows while its flag word carries `0x10`) and the
scene-node despawn over `Scene::release_node`. Not modelled: the weather option calls and the
renderer toggle byte, whose callee's effect was not identified. The actual camera, level and
listener stay in `exarust::viewer` and `crate::player`.

## The remaining screens' game side

None of this is wired into the running game yet; it is tested models of
routines that were read from the original.

- `flow::SaveFamily`: the save manager object the boot builds (file prefix
  and its length, the identifier word, two words that start at -1, whether
  the third hook is installed, and the magic word the boot writes).
  `story_family` and `arena_family` give the two; the arena's magic word is
  not the story's package magic, and what reads it is not traced.
- `manage::close_manage`: what closing Manage Company leaves behind (the
  hub's busy flag cleared, the window pointer forgotten, then the base
  form's close).
- `emblem`: the Emblems library name, the company-name confirm and its
  message box, the cursor position the new-career screen uses, the
  remove-layer button, the picture-step sum, the texture request with its
  loader flags, the dialog's constructor values and paint quad, the steps
  that close or finish the dialog, and the `klean` preset applied to the
  editor.
- `create`: the body form's and the hair form's constructor values (the
  body form's table word, the hair page's style list placed under its
  picker: `hair_list`), the edit page's update from a character record
  (`edit_page`: the `V` + appearance-label caption, the hair controls only
  with a hair picker), and the physique, skin, hair-colour and hair-style
  callbacks. The skin and hair-colour callbacks convert in single precision
  to a 64-bit integer, ties to even, keeping the low word or byte. The
  creation form's own parts: the preview view's words, camera and floor
  scene (`preview_view`, `PREVIEW_*`), the ten-element table's words by
  game mode, the editors' places, the close (smoothing off, apparel
  refresh, child close, edit close), the drop record's release, the shared
  change callback's steps and the mode-container drop.
- `creator_screen` runs the creator in the game: the menu's BEGIN opens
  the background picker (`TClassSelect`'s cards from `startcharsxa.rfi`,
  the chosen name and description), its confirm the creator for that
  background; the practice rows, the in-game NEW GAME, NEW CAREER and
  VERSUS open the free creator in their game modes. The form fills the
  window and lays out the original's three record tables
  (`exa_ui::creation_forms`) at unit 4, scaled to the live unit; the body
  and hair editors, their pickers (physique cross and marker, the tone
  field, the hair field and its shade, the attached gloss trackbar) and
  the hair style list are painted with the shown palette and the line
  skin, and their answers run the `create` callbacks into the record.
  NEXT asks for a name and shows the equipment page (its panels drawn
  empty), whose BEGIN or CREATE commits the record
  (`write_created_record`): it is kept as `CreatedCharacter`, the old
  player is dropped and the story or practice starts through the flow,
  the level's player taking the record's physique, skin, hair and name.
  BACK goes back a page or to the title. `--open-creator <mode>` opens it
  once loaded. The arena career, versus lobby and recruit commits, the
  power tree, voice previews and the kit lists are not wired.
- `creator_view` is the creator's preview: an off-screen camera at the
  preview's eye over `sgccflr01.rfc` with its own lights and the figure
  built from the creator's record (body morphed by its physique, skin,
  hair, underwear), rebuilt when the record changes, shown under the GUI
  canvas in the viewport.

## Scene bodies and the live arena round

`scene_bodies` is the scene body generator: a `SceneBodyPlan` names the
actor template, record or generated fighter a body looks like, its place
and facing, its team and role words, its condition words, the item in its
main hand (a drawn weapon variant's id) and its brain, and
`spawn_scene_body` builds the merged body model morphed by the record's
physique, the skinned parts and skin look, the garments, the physical
body's stats, its fighter and held item, the record data and possessions,
and a duellist or placed-character brain — the same components a level's
placed characters carry. The physical body's race muscles take the character
init's race presets over the race record's balance bonus and arm relaxation
(`character_muscles`; a human keeps its record's). A `SceneCommand::Despawn` naming the root or the
body model releases the character capsule and removes the root's whole
subtree. The generator reads the game's tables once, on its first command;
frames without one cost nothing. The arena's generic and hire-list
opponents take `BodyLook::Generated` plans (`opponent_plan`,
`hire_opponent_plan`) and write their own record in `arena_records`: the
random NPC pass at the match's level, then the level-derived outfit or the
hire profile (with the event's company bytes and weapon). The shopkeeper is
the exact `arenashpkp` actor template. The Beast takes `BodyLook::Beast`:
its record is the Ogre race with its tier's main-hand item and worn pieces
(`arena_records::beast_record`). Before every generated or Beast record the
shared sequence restarts from the byte-reversed performance counter
(`scene_bodies::clock_seed`), and a tournament side armed from the event row
draws its variant inside the generation, after the random NPC pass
(`hire_row_plan`). The random NPC pass's variant is the record's voice number,
its 0.9…1.07 curve the voice pitch and its appearance draw the undead voice.
Company entrants and hub specialists still take actor templates where their
saved records are not kept [guess].

`arena_live` runs the match's scene. The hub's launch changes to the drawn
arena in process (`exa_world::level_lifecycle`): the career is the running
`ArenaCareer` resource and the board's setup (with the event weapon row) is
kept in `PendingArenaLaunch`, so no hand-off file is written or read. The
match stands the first round from `match_layout::layout`, and the round tick
ends the round as before. The round's result opens `ArenaEndScreen`, which
paints the arena's single-row end menu through the GUI path (`NEXT FIGHT`
on screen 3, `EXIT;` on screen 2) and takes Enter or a pointer release over
its first row: a next round stands the winner, brings `regenerate_round`'s
fresh generic opponent, or the reserve entrant against the opponent who
won, after the 13 ms yield; a tournament's next round resolves
`arena_session::BracketState` and stands the first `sides` surviving teams
(the player's group leads), each rival side wearing the company the week's
field (`ArenaCareer::rivals`, drawn when its first company is 0 and saved in
the tournament chunk) names for its team; the end of
the match resolves the company's records, writes the healths, books the
result, drops the unfit on a tournament day and asks for the hub's level,
with the settled career and the return placement kept in process. Escape
opens the arena pause page and its FORFEIT row settles the match as
forfeited. The hub (in arena mode or under the play view's practice
stand-in) then puts the manager at the saved 300 cm, half turn placement
and asks for the shopkeeper and one body per employed merchant or physician
at their `arena_spawn` spots.

The hub's career now keeps all thirteen company records
(`career::Company::records`/`book_records`): the match return books the
counts it changes (wins, losses, forfeits, Beast wins, and the company
deaths the match's falls counted) onto the career's current records, the
match start counts a Beast match entered, and the records window reads the
lifetime coin totals, which the night rolls from the day's earned and spent
(the day's spent then holds only the wages). The dismiss dialog's Dismiss
books one more fired, logs the leaving name and drops the id from the
tournament entries and the log's saved records. The hire list is live: a new
career fills ten candidates (`ArenaCareer::fill_hire_pool_at`, one
counter-seeded `hirelings::draw_candidate` plus random-NPC pass each, the
trainer's snap), the loadout form's hire page lists them with their fee and
upkeep, HIRE runs the company's funds, room and duplicate checks
(`Company::hire_listed`), books the hire and the joining and removes the
candidate, and End Day renews the list. A candidate's level and the boards'
rank rolls read the career's one run-time library generator
(`ArenaCareer::random`, the Mersenne twister apart from the engine's LCG);
the per-candidate counter spacing is a guess. The candidate's item outfit is
not generated (the hub holds no item-fact table), so its price has no gear
share [limitation]. The Arsenal window and the kit release a dismissal makes
in the original are still not built. End Day rests the manager and gives him
the first trainer's lesson before the staff, books a tournament lost when the
company entered a week a rival took, then, in the original's order, asks the
live shop world to rotate the shop's shelf and the employed merchant's stock
(`hub_restock`, on the next frame), renews the hire list, rolls a new week's
board the day after a tournament day and rolls the day's six standard
matches. The week's board (`ArenaCareer::regenerate_tournament_at`) clears
the event entries and draws six event rows, the weekly prize that avoids the
last eight and the sixteen-company field, which becomes the career's rival
field that the board's winners table and the match launch read. The arena
reset keeps the hire list and the week's events, as the original's does, and
ends by rolling the day's matches. The arena save's company chunk carries
all thirteen records (the two coin totals as coin words), the day chunk the
day's spent and earned coin words and the tournament chunk the prize
history; the week's event rows and the hire list are not yet saved.
