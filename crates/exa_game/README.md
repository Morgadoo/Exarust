# `exa_game`

Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: the arena career (`career`) has the company, the roster, matches, a shop line priced at 1, 12, 144 or 1440 copper with a quartered sale, wages, the season, overnight rest and a trainer's lesson. One effect per thaumaturgy domain (`thaum`) and an Exarust session save (`session`) are in; `thaum::{casting, force, energy, displacement}` run the Force, Energy and Displacement casts with their gates, prices, slots and AI advice, and `thaum::{arcs, barricade, wisps}` the electric arcs, the force barricade and the displacement wisps, live in the game through `thaum::live` (`--debug-arcs`, `--debug-barricade`, `--debug-wisps`; [`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md)). The tendril emitter also has its world entry and removal and its node's update under a local-frame parent (`thaum::arcs`); the barricade starts cast motion 0x24 with the power 0x20206 and its focus reserve (`Barricade::add_to_world`), and its line is stamped on the navigation penalty while it stands (`exa_world::nav::stamp`). The force-shell aura (`thaum::ForceShell`, `shell_draw`) has its opening, hold and closing step, its eight ripples, the impact sound, the rigid copy, the `effectshell_v`/`effectshell_f` uniforms with the fade plane, and the skinned draw's placed vertices; a Shell cast spawns its aura node. The active power classes behind those casts are `thaum::power`: the base class every power derives from (its focus, state, HUD word, ending), the power-list, load, Power Glean and spirit-body routines, and one module per domain — Mind (Sense Minds, Mask Mind, Confusion, Enrage, Trust, Suspicion, Loyalty, Command Dead, Control Mind, the possession marker and Spirit Form), Force (shell, bubble, barricade, strikes, kinesis, throw), Energy (shock, shock weapons) and Displacement (the shared vanish effect, vanish, the long door, constant displacement, the summon). They are plain functions with tests pinned to the original's numbers. `thaum::cast` now creates them from the player's bar: `C` opens the bar, a digit picks, the arrows step and `Q` runs the picked slot through the bar's cast entry and the domain front ends, and each body tick walks the active list with `PowerBase`'s control and release. The five effect materials, their signatures, the noise picture's creation and upload and the four layered draw passes are in `thaum::effect_material` and `thaum::effect_pass`; a Shell cast now spawns its aura node (one sub-object, the noise picture), a Strikes cast its effect node, and a vanish, long door, summon or constant displacement cast its effect node, with a long door's move placing the body after the caster tick. The demon itself, the vanish's body show/hide on Exarust's drawn body and the effect shaders' GPU submission are still missing. A career match (`arena_match`) draws its arena with the variety count and the no-repeat lists, its track from eleven at their own volumes (the Beast plays its own), ticks once its first second has passed, ends rounds by extra rounds and lives (Elimination, Reserve), resolves falls (dead below 0.01, or below −0.15 of the maximum with a physician; saved otherwise below 0.01), writes health back as a word, and settles the return with the company's thirteen records; `ArenaMatchPlugin` runs the tick in play mode with an opponent. The hire list (`hirelings`) draws a candidate's profession and level from the counter, renews daily, snaps a trainer's skills and trims a combatant's loadouts, and picks the loadout a fighter takes into a match (half its levels capped by the tier, two under for a Captain's wing), repairs a broken list, copies a loadout up on a rank and values an outfit; the arsenal (`arsenal`) shelves items by kind. The staff (`staff`) hires with the purse, room and duplicate checks and their messages, recruits for 12 copper, stows and releases kits, renumbers fighter ids, and on a leaving drops the id from the entries and the log's saves; the daily report (`report`) titles the day and writes the log, training, rank, hire-list and tournament lines. A roster row dragged onto another takes its place and the moved ids follow in the log, the entries and the order list; the day's lessons teach the manager, then the fighters. An item sold from the arsenal (`arsenal`) leaves every box and holder when unique, else one copy (a 0x200 copy first), the holders keeping what is left. The outfit tables are the shipped arena data (`arena_outfit_data`): the large pools keep their ids, weights and masks, the small ones their rank/cost or class words, and the item-fact pass fills ranks, points and bonuses at run time. The two generators (`generate_profile`, `generate_by_level`) run the original's complete call sequence. The spawn plans (`arena_spawn`) give the body generator the beast's tier items and scale, a generic or hire-list opponent's level and eight-byte outfit, the manager's lift, role and team, and the hub's nearby-weapon query. `manage_icons` holds `arsenalico.rfi`'s seven 96-pixel cells and the runtime atlas they grow into (two cells wide, four tall); `arena_match::match_tick` and `arena_hub::enter_hub` are in. A fresh tournament match (`match_layout::raise_fresh_match`) stops a side at the first body the generator refuses, reports "Failed to generate match motile.", links the successes at familiarity 100 both ways with a 3000 ms grace on both AI clocks, and stands the player's own entry without spawning it again. A rematch (`match_layout::stand_rematch`) drops bodies in the dead mode, stores a tracked fighter's health on the full scale, and stands the survivors on the player's spread with a 10000 ms grace and a rolled jitter for everyone but the player. Creating the scene bodies is not in. The end screen (`arena_session`): a round's result sets state 8, 5 or 4 and opens screen 3 (next round) or 2 (end); its immediate Continue follows an auto-continue switch the game never sets. Continue sends a dead manager to the main menu, starts the next round (a tournament raises the bracket's next fight) while the round less the losses is under the extra rounds, and otherwise settles the match and returns to the hub with the 300 cm, half turn, 0x100 placement. The next round keeps the company's fighter and generates a fresh opponent at the match's level and tier, or, when the last round was lost and the fighter is dead, brings in the reserve entrant against the opponent who won. The return writes every company body's health word, books the result and its records, and on a tournament day with an entry drops the unfit and reassigns the pending events' rivals.

The random outfits (`arena_outfit`): rank windows, the quartered and halved weights of the two ranks under a limit, masked and max-rank picks, the stepped roll, the two-pass pick that refuses a piece already worn, the apparel bonus merge, the weapon, shield and apparel row fillers from item facts, the smooth unit look curve, the profession-level grant and a random NPC's curves. Every table is caller-supplied; the retail ids and names are data, not code. The save ids (`arena_ids`): before a save every manager, stash, arsenal and staff item is prepared, in that order (a record's hands, worn items, loadouts, then carried items); after a load the entries the object table rewrote take their new ids and, when flagged, the first stash item (or the first employee list holding it) and the loadouts, or the hand pairs and the carried items, follow. The fighter rank is the six skill levels summed, halved and capped at Master; the arena numbers the manager first and marks the classes' leaders. The company windows (`manage`): roster labels and name inks, Recruit and Hire closed at eight fighters and ten employees, report-line clicks, the arsenal form's pages, selection, steps, dismissal, rank slider and points line, record titles and texts, Company Status. A tournament event (`tournament`) settles the events the company skipped with a rival of 2 to 7 (2 to 6 when the return to the hub settles them on a tournament day), runs rounds less one as extra rounds and fills its bracket with `sides << (rounds − 1)` teams; forfeiting the tourney empties the open events; arriving at the hub and the rematch lineup are in. The hub now draws and opens them (`arena_hub_live`, `hub_windows`): its entry plays the hub music, places the manager, installs the click and fall hooks, and the click hook opens the board, Manage Company or the trade window on a `GuiTask`; the board's paint list, the roster rows and the shop stock draw through `gui_paint`. The name klean selects emblem background 52 and design 81. A practice fight (`practice`) advances by one and stops at 8: novice names opponent01 through opponent08, expert names opponent09 through opponent16, and red gains a tenth of the maximum. The partner is not spawned. An unwatched match draws a team from `(count × 0.5 + 0.5) × strength sum`; mode 16 leaves the player's team out. A bracket clears losing ids. The first group keeps a forced id and does not step the roll. A later group keeps the team whose weights pass the high half of the weight total times the stepped roll. A tournament field draws 16 of the 19 companies. Roll 0 ends at 0x4b157bf0. Roll 1 ends at 0xead32bb1. A day chooses six of seven templates. Rolls 0 and 1 end at 0x1210d4e5. Roll 4 ends at 0xb761a41e. A performance counter replaces the roll by reversing its low 32 bits. A copied match multiplies the fee and the prize by one more than the slot. A seeded unit draw tempers 624 words. Seed 5489 starts at 0xd091bb5c. A record copy writes plain fields, the gaps, and the tail. Managed strings are not in. Before a match the first specialist under 700 health is removed and the death count rises. Health from 700 through 64999 leaves without that rise. Health of 65000 stays. A combatant stays. Kit release is not in. In-game menu command 4 in arena dismisses a child whose flags include 0xce100000. That child releases a capture and clears style bit 0x20 when it holds one. A remaining capture publishes its code and name. Bit 0x8000 passes its low byte as the shape. Shape 0 shows the default image when it differs and is not zero, and stamps the clock. Shape 7 shows its record. The child clears a hover, closes the task's other children that name it, then closes its own children from the first to the last with argument 0, unless it is already closed. After the lists, a focus is counted once before argument 1 copies ten hand words, when a focus is set or the mask shares bit 0. A session mark of 1 becomes 0, a flag becomes 0, and a code becomes 4. Side 0 copies sixteen words from the first bank when its flag is set. A ready install of index 1 shows that record once and runs its hook. Index 0 shows a fallback and stamps the clock. A shown value is passed with two platform words. A shown value activates the action set. The Steam library is not loaded. The gate hook sets its flags to 1. The watch hook stores a stamp of 0 without bit 0x2000, and a leave enters the session. A clear gate counts a pick. A taken item sets flags 5. A hold byte does not pick. A parent equal to the world counts an open. A fast aim copies its place. A miss with the hold byte clears the aimed word. A taken item sets click flags 0x11. A hover stores flags 0x15. A record of kind 0x10010a or 0x10010d whose link is the world counts one open. A different link does not. An item id of 0 does not count a kind call. Bit 0x80000000 searches the high table and bit 0x40000000 searches the wide table, after those bits are dropped. A span that holds 0x20000 counts one kind call. A cursor at the view edge clears the hit. A marked item drops a held count above 0, and byte 1 closes a pointer. The first pane tagged 0xce1c2013 is shown. A scene keeps the last positive body. A new pane doubles the row before the slot and clamps the place. The ray starts at 4000 along z. A centre at 100 leaves reach 100, and a centre at 500 is left. A centre child sits at half the spare. A free camera ray, nested children, and the focus method body are not in. A cleared mark registers code 0x8001 unless the hold byte is set. Raising the mark drops the hooks. An open world sets the side from its byte. Raising the mark to 1 installs index 0 and does not copy a bank. A mask without a focus does not count it. A shown child hides and the redraw word gains bits 0 and 1. A pinned child raises a notice and still closes them. The command then runs that leave, asks for a save that does not abort on low health, marks the hub busy, and sets the phase to transitioning. Drawing the cursor image and the other menu commands are not in. Menus, the other emblem controls, and the original's packages are not.

The original's prelude helpers (`prelude`): the tips popup with its two
tables (eighteen rotating tips and six control hints; index 0 draws a
rotating tip under the `ShowGameTips` setting and, as the original does,
shows the draw-th tip rather than the draw-th least-shown one; index `n`
shows hint `n − 1`), the `actors.rpk` template reader with and without its
cache (records matched by their own sixteen-byte name, letter case
ignored; a nameless template takes the asked-for name), and the practice
opponent's plan (traits t0..t2 set to −100, 100 and 100, team `0x10`, a
3000 ms reaction delay; the original's fixed spot is recorded but not
used). The binary's `--opponent` builds its fighter from `opponent01`
when the actors are installed and gives it that team and delay.

The power tree (`thaum::tree`) is the whole screen's rules: the domain's
tree file and its marks, the technique a new power asks for, the click that
picks or drops a learnt power against the record's focus budget, the pan,
zoom and spring, the domain wheel's shares, the ten-slot power bar with its
load, store, cycle, drop and flash timings, the cast entry that dispatches
by domain and the persistent-slot budget; its store writes a domain's
learnt and picked nodes back into the record, plus the form's opening and
closing fades, the 17 ms view frames, the bar's 900 ms open/close
animation, the tooltip and label layout, the stretched link's own cubic,
and the caster's grant caller. A constantly displaced body
(`thaum::veil`) gets its shell effect (starting dead, fading the body's mesh and skin
parts), its displaced collision bit, its hum start point, its tendril
image, its visibility bits and the hit veto with the 500 ms collision
restore; none of it is wired into a running game yet. The running screen (`power_screen`) opens the tree with P and draws the
tree and the bar through `exa_ui::tree_draw`; the bar's keys are the hotkey
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
a tick), the redraw-flag idle wait (`idle_wait`: 1, 32, 64 or 1024 ms, none
for an immediate redraw, at least 32 ms minimised), the logger's
presentation levels (status line, timed overlay, message box, file only;
level 5 skips the file), the sixteen-entry newest-first status history and
the fifteen-line timed overlay whose `title : message` lines stay two
seconds plus fifty milliseconds a byte and go one a tick. `window` holds the
display-mode list (modes at least 640 wide, every refresh rate per size,
the original's exchange sort by area), the refresh rate (unknown below
30 Hz), work-area fitting, the 640 × 480 minimum track size, the full-screen
rule (`fullscreen_mode`: borderless over the desktop, exclusive only for a
listed `ScreenWidth`/`ScreenHeight` other than the desktop's), the swap
interval's present mode (immediate, or with vertical sync the adaptive
`AutoVsync` while Exarust's frames are slower than the display, where the
original uses plain Fifo: `ADAPTIVE_VSYNC`) and the Alt+Enter toggle.
Tests pin the arithmetic and the orders. `hw_config` applies the full-screen
rule and the present mode, `exarust::frame_loop` the pacing, the focus key
release and Alt+Enter; the other models document the original's loop, which
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
the original does (resetting a bad header), lists a number once with the
**first** file deciding its fate, deletes only story slots with an older
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
(with flag `0x400`, once), flushes a scene character's items and registers
its world record, and compacts the stash, employee and record lists. An
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
there instead of the level's own start. A live level change does not exist
yet (a change restarts the process) and the level loader keeps its own
scene model rather than the exporter's, so the bridge waits for the RFC to
export-scene conversion; see `docs/formats/SAVES.md` (Level transitions).

Character creation (`create`): the record's gear (four hand items, its
worn list and its carried list), the mode filters (story background 0 keeps
braies only and clears the hands; the arena keeps the first two hands only
when its kit offers them and keeps braies only; modes whose 2 bit is set
clear the fist placeholder and the carried list), the story presets and
their settings keys (the first key per background, one shared character
key), the free-create and entry plans (recruits are not filtered, story
records lose their experience, every non-continued record is set to full
health). The unknown background resets its gear and starts the
thaumaturge's two powers; a named background always wears its template's
gear. The chosen name, hands, worn and carried lists, skills and full pools
are written into the spawned record. The starting skills write Close
Combat, Armour and Shields at five levels with their first five
techniques and leave the rest of the record; the skills page caps them to
the account's experience (a knight or proctor has at least 10000), keeps
the first truncated slot's chosen technique, gives the leftover to the
skill in training up to its next level, and shows the account's worth less
the committed character's, converted back to experience (each level of
worth at weights 4, 5, …, only the first partial level, quartered toward
zero). The apparel and weapon kits come from the original's seven tables
by mode, skip what the record already holds and resolve aliases and new
instances. The two pages name their tabs and final button (RECRUIT, BEGIN,
READY or CREATE), list the equipment panels per mode, open the power tree
for a record with powers, and the continue screen toggles between a save
and a checkpoint only while both exist, with its list, details and name
cut to fifteen characters. Appearance sliders settle and copy with the
original's epsilon and 64-bit debounce (a NaN distance takes the write
branch, as the unordered compare does), the face variant steps through
seven values, and sex, hair visibility, the voice sample names
(`VHM01_GruntS`, `VHMDA01_GruntS`), the pitch (worked in doubles), the
voice preview's 300-unit throttle (a 64-bit difference) and its
non-repeating random variant are in. The look a record carries (skin tone, hair style and colour, beards,
normal-map layers, the look seed's skin-map offsets and the per-limb leg
scale) is `exa_character::look`; the spawned body builds its `CharExt` hair
with `exa_character::hair`. The creator icon sheet is four
96-pixel cells at a scale of eight units. The form's labels and page
captions live in `exa_ui::creation`. Of the form itself, the preview
frame (180 units in, 24 down, the parent less 360 wide and 28 high), the
two layouts (mode 12's ten-element table without editors, every other
mode's seven elements with the two editor sub-forms) and the routing of a
drop onto the preview (item 8 ignored; the mode's own list, game mode 8's
apparel list, game mode `0x1010`'s hook, else by the item's kinds:
apparel `0x1000`, weapons `0x4000`, and `0x8000` to the weapon list
without the placing step) are modelled. Drawing the form, loading the
model preview and sounds, the object database behind the kit, record
serialization, the power tree and the running mode transitions are not
in, and nothing here is wired into the running game yet. Rules are plain
functions with tests pinned to concrete numbers.

Story experience (`experience`): new item kinds, kills and explored map
cells (the 5 × 5 cells around the player on a 600-unit grid, the position
rounded down to its cell, a bitmap per
level) earn discovery points, once to the account pool and four times to
the player and each companion. Once a second a character converts a point
per 50 ms of its cost, `4 + skill levels`, into 2 points of its trained
skill; the player's paid cost levels the power in training, a companion's
level adds 10 combat skill and courage and picks its next technique; the
account pool converts at `(global / 5000 + 1) × 4` up to 25000. The
account experience's worth and its inverse, the companions' list, the
fighting estimate and the discovery chunk of a save are in; the arena's
lesson goes through the same skill lump. Nothing calls the tick in a
running story level yet. The shop (`trade`): a first stock of 18 to 20, the
rotation by size, candidates by kind, listing, model and rank weighed by
rarity through the original's quick logarithm and exponential (duplicates
divided by 3, arms halved), the draw, new apparel and weapon copies, the
merchant's category and rank options, and the transaction against the
company's coins (`Company::restock`, `Company::settle`). The shop window,
an apparel copy's description phrase and the weapon variant roll are not
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
arithmetic (limb pushes, the character knock and its stun quirk, rebound,
push-out, simulated points), their frames' sounds, colour bytes, texture
offsets and torrent light points, their geometry classes
(`effects::effect_geometry`) and multi-point lights (`effects::emissive`)
are in. The engine's mesh and sphere tests that find contacts, the ribbon
vertex builds and the GL passes are not. A blast or bolt cast runs its
blast as an entity (`thaum::blast_live`): the cast motion and hook as the
original's start sets them, and Exarust's own sphere test against the
other bodies' limbs. A barrier cast runs its dome as an entity
(`thaum::barrier_live`): cast motion 0x23 and the hook that ends it, the
focus it pays a tick, and its shell pushing other bodies' limb spheres out.
A bubble cast runs its dome where it was cast (`thaum::bubble_live`),
ending with its power. A wave cast runs its wave round the caster's body root
(`thaum::wave_live`): charging on the blast's hook, paying its drain each
tick, and thrown into other bodies' limbs. A torrent cast runs its streams from
the caster's right hand (`thaum::torrent_live`): charging and pouring on
its hook until the caster lets go or runs out, its points pushing other
bodies' limbs, and leaving once its frame would draw nothing. Cast hooks answer by event
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
sphere's shaping and the spirit screen's start. The overlay's and screens'
GL passes, the animated noise and the casting AI are not in.

The small effect nodes (`effects`) are in, on the original's game clock
in whole milliseconds: a level portal marker (the `0x3DD0E004` emitter
node, `_PortalPrt` in `uwportal01`) gets the soft blue 0.75 light of
500 cm 50 cm above it when its model loads (its `_PrtlPrt01` sparks are
not simulated), and a body stepping through one gets the seeded
`_PrtlDisp` dissolve burst (cells 6 cm apart, two fifths taken, each
particle's position moved halfway between the node's two frames,
jittered ±1.5 cm and pushed 1.5 % out from the body's centre, lifetime
41 ticks less up to a quarter) with `portaluse01` at 0.6. A lit held item
carries a flame row from 30 to 115 cm with the `_FlameMat01` shader,
fading in by 0.02 a tick and out by 0.015 in double precision, its
ribbon's points closing a fifth of the gap to 20 cm above its ends, lit
by a 5.0 warm light of 800 cm whose softness walk is 0.87–1.15; its
ability word and fuel decide ignition, `FireSizzleA` plays at 0.3 and the
`Flaming` modifier is 0x10020000 (no Exarust item carries that ability
yet, so nothing lights one). A displacement shell opens over 400 ms and
closes over 300 ms from its shown value, with the fade (1 − shown²)², the
texture offset, its fade plane and 900 over the squared distance as its
detail; a teleporter scan opens over 4800 ms, closes over 800 ms and
flashes its colour at three times the shown value when it loops; a bubble
grows over 1800 ms, rests 500 ms or holds and charges over 7000 ms
(flashing at 5600), then shrinks in 400 ms around a 120 cm sphere; the
domain wheel is a 280 px form of six domains (Mind, Force, Body, Energy,
Light, Displacement) clockwise from the top of a 76 px ring, with a
20.8 px hit, its hover, label, open ease, choice flash and confirm rules.
Rules of the Mind casts and sight and the two force effects:
[`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md) (Mind casts, Force blast
and torrent).
The reconstruction device starts the scan (`teleporters`); nothing
starts the shell's aura entity or the wheel yet: their callers (the shell
power's visual and the power tree) are not in. Drawing the flame ribbon's
vertices, the shell and scan mesh copies, the bubble and wheel geometry,
the portal particle simulation and the bubble's GI light values is not in.

The force and environment effects of the same powers are in as plain
rules: `effects::rain` (4096 drops in a 600 cm box ahead of the camera,
the gust walk, the fall, the tilt-dependent streak, the ground wrap on the
effect's own generator, the colour and the two shaders kept as data),
`effects::portal_effect` (one ellipse particle per call, its previous and
current positions, the 3- and 5-bit look draws), `effects::fire_node`
(the `_FlameMat01` node's 800 cm 1/0.68/0.43 light), `effects::force_dome`
(the eight-ripple list, the cast sound, the Barrier's damped height,
800 ms grow window, drain, drawn radius, frame and keep-out, the Bubble's
1200 ms grow, break and 700 ms fall and its two keep-outs, the character
contact and balance drain, the double-sided meshes), `effects::force_wave`
(32 chunks 80 cm out, the charge, release, thrown step, trails and
lives), `effects::danger` (what the AI sees of a blast, a wave and a
barricade, and the steps away), `effects::emissive` (the seven GI lights'
extents, colours and boxes), `effects::effect_geometry` (the geometry
flags, programs, uniforms, pass rows and fades) and `effects::vanish`
(hide, show, lend, phase hook and finish). A cast starts its dome or wave,
but none of these is drawn: the rain's caller is the scene's weather, the
vanish's is its power effect node, and the AI's sight is not registered.

The per-character item effects (`item_effects`) are the body's effect
list and its two impact counters, the held charge (the HUD shown while it
is under its source), the hand-effect dispatch (electric, glow, flame), the
power-curve glow with its level step, the worn item's light and the apparel
effect with their 0.9/0.1 hysteresis, the skin material fade (the square of
the value, first match) and the focus-item power slot. The golem guardians
are in `exa_ai::golem`. Nothing spawns a golem or dresses a character from
these yet.

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
from the body's physical turns, the original's clock rate for what the
body does (the base in its stance whatever its balance, twice it
kicking), the pose it holds while dazed, the sample taken in the
original's per-part pipeline (the Run clip overlaid on the Walk by
0.7 × the run blend, or by the blend with the sprint latch), the spine
turned by the torso twist (the lumbar half), the head turned toward the
look by its ramping weight (its anchors then on the physical thorax), the
thorax hunch on its pinned part, the foot pitched to the ground (a
20 cm toe, eased 0.01 rad a tick), with an identity slot for the IK
chain; the
clock of a locomoting body takes the locomotion's step; a get-up's clip ending into the stance writes the condition
word normal; sampler); its targets, the sampled parts on its character
frame, and its lift, its controller told to follow the frame (carry); a body asked to go limp keeps its motion unless given a
velocity (`limp`), and a level's placed dead are built limp from their
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
the characters' footprints on the navigation grid, kept each fixed tick where their feet stand (an NPC always, the player unless it walks; `footprints`),
falls to death and the kill plane (`fall`: the fall check of a living
body in the falls branch, in its posture stage, its scream and the player's pit sound, the
death fade's start, death and removal from the world; dead bodies below
−50 m taken once, the player's kept),
the world cursor's shape from the player's condition word (`cursor`), the
hand (`hand`: the left press named by `pick`, the drag of props with Ctrl
and of corpses through the solver's grab, the lift of a prop onto the item
cursor, the reach under the cursor and the refusal that keeps the movement
request; `docs/PLAYER.md`, The hand), the item cursor (`item_cursor`: the
lift's turn ramp and rotation, the cursor's placement and box refusal, its
release and the GUI drop's branch and queue), the health
bars' values and the low-health heartbeat with its sound, started by a
blow's low-health check on the player's body (`exa_combat::Weakened`;
`hud`), the
held torch (`torch`, which fills its hand and holds it by the grip
table's one-handed or off-hand grip for the holder's size and race; its
tests pin the "Torch" clips' arm on the body model), the teleporter and reconstruction devices (`teleporters`: a `UseDevice`
request finds the device's crystal slot and the placed object on it; a
teleporter asks for the destination screen with its rows and level ids, a
reconstruction device puts the effects' scan on the player, waiting
1600 ms of game time and closing with the flash (a scan already on the
player makes the use start nothing), plays `devicescna01` at 0.3, scales
the device's voxel lights by the glow pulse and raises the reconstruction
latch; `SlotReleased` powers it down with `devicepwra01` at 0.1;
`EndSession` reconstructs a scanned player who dies in the story,
stripping them the first time (hand and worn data cleared, health just
under half). The destination screen and its journey bubble, the grab and
use input that would send the requests, the level change, the corpse
record a strip leaves, the emissive half of the glow and the sound
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
soft height lock on a target's bone and its raised surface, the gamepad
aim's `250·|aim| + 50` cm point and its `(0.4·AimSpeed + 0.1)²` ease, the
12 m contact list and its 12-entry eviction, the strongest engaged enemy
(the enemy's own contact score for the player over 2), the character
nearest the cursor on screen, and the attack wind-up and recovery times.
The gamepad's aim runs through the eased world cursor each behaviour tick
(`player::AimCursor`, read by `player_combat`); the others have no caller
yet. The right button walks to the cursor in combat mode too
(`steering::Steering::combat_walk`), the chevrons showing while it is
held.

The controls' rules (`player_controls`) are the click's and the
double-click interaction's outcomes (a lock's key and first-unlock
reward, a container, an item use in hand, a button push, a corpse's scripts,
a character's talk condition, the player's inventory), the item-use values
(a pour's warmth and marks, the cleaning and repair kits' targets, their
sounds) and the timed list that writes only while it lowers a section or
arrives; the reach's navigation region and walk-to point; the movement
request's drag modes, the controls' gates, the key permissions and the power
bar's 500 ms hold. A click on the player's own body shows the HUD bars for
5000 ms; a double click on it (with Ctrl in combat mode) opens the
existing inventory screen; a double click on a corpse runs its roles'
corpse scripts through `exa_script` (the corpse as actor 0, the player as
actor 1) and sends their effects on, and the other windows have no form and
the living character's talk condition is `exa_ai::roles`'s. The controls' scene-node words are
`control_nodes`, a tested model (the runtime hand holds physics bodies, not
scene nodes): the active and hold words, the grab switch and its release,
the body-bearing subtrees, the frame refresh and orthonormalisation and the
placement frame's bits. The item cursor (`item_cursor`) is live: a lift
without Ctrl grabs the prop (strength 1, 0.003 m of slack) and rides it on
the cursor; the mouse turns it by `0.25·sensitivity·0.0025` rad a frame,
ramping to `1.0` after 300 ms of turning and falling back after 500 ms of
quiet, keys W/S/A/D standing in for a stick under 0.1; each frame the
cursor's ray places it at the nearest hit under the drag's 1000 cm reach
where its rest-point box stays within 280 cm up and −1000 cm down (the
spawn's first placement takes any hit; a miss lands 1000 m along the ray,
a climbing ray refuses),
and the release puts it back at the frames the lift saved, leaves it on the
give-back hover, or lets the GUI take it. The GUI drop's branch (an entry,
from a GUI, with a body) and its next-frame queue, and the put-down's
scripts within 5 cm² (`player_controls::nearby_scripts`), are tested rules.
The right press's use start, its target class, the GUI use and the used
item's removal, the point sound at 0.25 and pitch 1.0, and the door link's
stop bit are likewise rules; the wheel's set wraps its grip angle into ±π
(the single π over the double one) before turning toward it. The pad's
camera look takes the frame from the mouse-look capture, as the original's
gamepad branch returns before touching the pointer. The start and end of play set the AI's activation centre and
radius (`PlaySession`, which the player tick and the AI read); `PlayerControl`
is the player-control singleton's creation state (hotkey mask 31 and the two
constructor flags). The GUI character preview (`char_view`) turns the
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
and exposes the Continue and forfeit branches (`end_action`, `forfeit`), but
the play-mode match still has no end-screen input. `arena_hub_live` runs
`arena_hub`'s rules from the game. When a play level
named `arenahub` is in (or an arena session enters it), it calls `enter_hub`
once with the hub roll and the flow's alternative-music setting, writes the
track and its start offset through `exa_audio`'s level-music resources,
installs the click and session hooks and places the manager by
`arena_spawn::manager_plan` on the hub's known start. The session hook calls
`hub_fall` every frame; an answer ends the flow's session, and a wipe reaches
`slots::Slots::drops_after_death` when the host inserted `HubSlots`.

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
through `gui_paint`; Escape or a right-button release closes the window. A
board Enter, Begin or Continue takes the event's entry fee from the purse
(a short purse stops at 0; nothing refuses the match), warns once when the
board's Begin enters the manager, drops the first unhealthy specialist,
plans the match through `arena_match`, the layout and the spawn plans, and
sends `arena_hub_live::ArenaMatchLaunch`, which the launcher turns into a
restart on the drawn arena with `--arena-match`, the event's rank and gear
tier, and each opponent's drawn weapon variant. The play-mode spawner uses
the rank and tier for the layout and holds the weapon whose object id was
drawn.

Opening the board or Manage Company makes the hub busy; closing either
clears it. The notice boards and the shop counter are static scenery with no
rigid body; their colliders remember the scene node they were built under
(`exa_physics::ColliderNodes`), so a click on `\xaaArenaShop` or
`\xaaArenaMach` reaches the hook. The shop counter's node holds only a
`_null` box, which the original never draws but picks; it is kept as a
hidden pick-only collider that nothing collides with. In `play arenahub`
the flow runs Exarust's practice stand-in once the player spawns; with the
hub loaded the hook takes that (and the menu) as the arena mode. Not wired: the
shop's first stock build (`trade::update_stock` needs a live `ShopWorld`;
`trade::ensure_first_stock` and `career::Company::first_shop_stock` hold
the rule), the match's return to the hub (`arena_session::settle_return`
holds the rule), the windows' own close controls, the board's
pictures and prize items, the manage report lines and controls, and the
trade window's item icons and character preview.
The hub's shopkeeper and specialist placements are `arena_spawn`'s plans;
the game does not spawn those bodies yet.

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
and the tournament weapon tiers.

The hardware configuration (`hw_config`) runs the settings screen: it holds
the live options, the saved set and the open page's draft; opens the page
(reading the settings file again), applies, cancels and runs Defaults per
page; writes the settings file's lines; rebinds a Controls row of the page's
copy of the bindings from the next key or mouse button; and drives the GPU
benchmark (a 1024² target, a 512² texture and sixteen full-screen quads, the
original's half-second warm-up and three runs). It pushes the live options into `exa_render::quality`
(super-sampling on or off, shadow-map sizes, MSAA), the audio buses, the
frame-rate readout and the window's mode, size and present mode
([`docs/UI.md`](../../docs/UI.md#settings-options)).

The camera's navigation unit is `nav_camera`: the align routine's wrap
of the target yaw about the camera's, the engine's planar yaw, the orbit
rate (4.8 rad over the screen, in double precision) and pitch limits (0.8
rad to 1.5, 0.000125 steeper per zoom unit, the held pitch), the frame
clock's nine ticks, the camera placement with its height, distance and
pitch clamps, the free zoom's 0.2 % per wheel unit, the preset record's
numeric fields (the 59² view and size scales, the 59th roots of the
retention and the rate, the pitch bounds, the 10/10000 distance range and
both cameras' speeds; its GUI callbacks and editor scan are not
modelled), the cut-away segment and the kick offset. Its picking unit is
`nav_pick`: the screen ray's plane pick, the node walk with the class,
layer and option bits, the segment against each bounding sphere and box,
the nearest mesh within the best distance, the ground-plane fallback and
the kinesis pick; the cursor's aim plane and reach use them, and the
physics world answers as the mesh callback because Exarust keeps no CPU
triangles.

The arena's per-rank kits are in (`loadout`): `Member::loadouts` holds
six of the record's own loadouts (hands and worn items); the editor takes
the rank's kit, that rank's point limit (`arena::point_limit`) and the
shop's shelf as its arsenal, and its close writes the kit back in the
arena and match modes. The session save does not store the kits or the
emblem yet. The technique selector's bridge (`techniques`) reads a
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
inventory screen submits the layer over its pack panel. No item renderer
fills the atlas yet, so the panel is empty in the running game. The card
window (`CardWindow`) keeps the 200 ms request, its toggle and its 0x100 ms
close fade on top of the card timer; a double click classifies a
container's open, a card close and a use; a popup's inner panel and top
are `open_container`'s. The inventory
screen still has no bound item list.

The gamepad (`gamepad`) reproduces the original's Steam Input action sets on
Bevy's gamepad backend instead of Steam: the menu set and the interact and
combat gameplay sets with their digital and analog actions and manifest
names, a documented default binding table, the set table with its gameplay
layer, the menu reader's 0.25 dead zone, 0.1 analog bits and scaled
cursors, 400/150/30 ms repeats, three-millisecond set priming, the
directional control pick at the GUI unit, the focus stack, and the gameplay
reader's camera-relative move, aim, the look routine at the orbit rate, the
stepped zoom, the power-bar and dialogue step modes, the 500 ms power-tree
hold, the command word and rumble. The player reads it for movement
(`pad_keys`), combat steps, aim, attack and alternate attack, crouch,
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
(8, 8) or remembered place and is registered as an open container; the
journal keeps its session entries and stores its text on close. Escape
closes the focused screen (Exarust's). The rules are `exa_ui::game_gui`'s.
The screens draw through `gui_paint` and `exa_ui::gui_draw` with the words
their constructors write: the frame (no title bar, as no shipped form sets
the title bit), the inventory's 228 × 180 pack panel, and the control
records laid out by the two-pass layout (the inventory's Encumbrance label
and Primary button, the journal's New and Erase buttons) with captions
placed by the skin's own rule, and the journal's selected entry through the
text box. A dead body's double click in the world opens the body's loot
window (400, 36; 244 × 440) with its pack and equipment panels; it joins
the open-container registry under the body's tagged id and closes with the
focused screen. The character view, the preview bars, the pack's and the
loot panels' item icons, the journal's entry list, the controls' callbacks
and the inventory's unlocated second record are not wired; the text keeps the level font's metrics and
samples the base font's atlas while the Rayform faces are not shipped. The
skills key's screen and the power tree are not opened.

## Scope

- App states (boot → menu → loading → in-game).
- Arena: companies, fighters, matches, rewards; Story: levels Underworld 0..8; Practice/sandbox.
- Save games in the original's package and player-data shape, with our own
  payload for the session state.

## Game flow

The Energy domain's Shock creator builds its charge object with the cast
flag 1, the base cost, the clip's charge, the hand arcs (20 tendrils of 5
segments at −15, −5 cm), and the `thmnrg_chrga` and `thmnrg_shock` sounds
(`exa_game::thaum::energy::shock_power`).

`flow` is the original's game-main flow: boot into the title, the mode
switch (practice, story, arena, scripted modes), the session starters
(story from a background with its roles, record words, pose, light and
Derrin, practice from the ladder with its role, the story and arena
continues, the arena hub), the transition screen's 100 ms actions, the
per-frame dialogue and session hook (with the wake-up's whole animator
write: flags, blend, move and turn factors and the clip clock), the story's
wake-up, knockout and revival, death and the end-of-session menu, level
exits with their save and checkpoint, the level-load finish, the story
end-menu exit, the story respawn at `exanima05`'s portal, the application
quit's autosave, the menu form routines (show, hide, the help flag, the title logo),
the return to the menu form, and the pause. The rules return named steps;
`FlowPlugin` runs them on the 12 ms game clock, follows a play level's
player as a sandbox practice session, turns a teleporter's journey into a
level exit, and hides the session hook while F1's in-game page is up (the
menu screen calls `menu_opened`/`menu_closed`). F1 opens the in-game page
while a session runs. The camera shots (offsets added to the framed point)
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

`game_event` is the story scripts' `GameEvent`: memory fragments (head pain,
a flash, `XU_Memory`, a fragment bit and a potential point for a player with
the memory role), the raise-dead lines, Corlian's line, the three domain
awakenings (the potential plus ten less the fragments seen, the voice, wail
and swing, the mind domain for the second and third codes, the force domain
for the first and third, and the class word) and the background-unlock bit.

The main menu screen's F1 now takes the flow's mode, phase and page state,
and its in-game page in a session; while the page is open the flow's
dialogue and session hook are held, as the original's frame task does. The
menu rows' text program is `menu_text` (the unit's one shader, still to be
bound on the draw path).

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
PRACTICE, VERSUS, MANUAL and SETTINGS is named, not performed. Escape goes
back from the arena and practice pages and closes the screen elsewhere. The
rows are measured with the menu font's record (`titlefont.rfi`) and drawn in
the base font until its quads are wired.

## The trade window and the camera's wall probe

`shop_window` is the arena shop's `TShopWindow`: the 0x36c × 0x244 page
titled "Trade" with the character preview and the player's pack under it,
the quartered sell basket, the full-price buy basket and the stock at the
right edge. Each basket's callback re-sums its own total. The bottom row
reads "Pay:" or "Receive:", warns "You cannot afford this transaction." and
disables the Trade button on a short purse, or when paying with both
baskets empty; the merchant flag picks the label rows. A take-off from the
character view lands in the pack, a right release hides the window and the
close releases the preview and the trade-mode bytes.

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
and the audio listener with its global fallback) and the scene-node despawn
over `Scene::release_node`. Not modelled: the tick's copy of the camera pose
into the listener, the weather option calls, and five option bytes whose
callees were not identified. The actual camera, level and listener stay in
`exarust::viewer` and `crate::player`.

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
- `create`: the body form's and the hair form's constructor values, the
  edit page's update from a character record (`edit_page`), and the
  physique, skin, hair-colour and hair-style callbacks. The skin and
  hair-colour callbacks round half to even like the original.
