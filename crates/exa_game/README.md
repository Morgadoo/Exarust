# `exa_game`

Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: the arena career (`career`) has the company, the roster, matches, a shop line priced at 1, 12, 144 or 1440 copper with a quartered sale, wages, the season, overnight rest and a trainer's lesson. One effect per thaumaturgy domain (`thaum`) and an Exarust session save (`session`) are in; `thaum::{casting, force, energy, displacement}` run the Force, Energy and Displacement casts with their gates, prices, slots and AI advice, and `thaum::{arcs, barricade, wisps}` the electric arcs, the force barricade and the displacement wisps, live in the game through `thaum::live` (`--debug-arcs`, `--debug-barricade`, `--debug-wisps`; [`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md)). A career match (`arena_match`) draws its arena with the variety count and the no-repeat lists, its track from eleven at their own volumes (the Beast plays its own), ticks once its first second has passed, ends rounds by extra rounds and lives (Elimination, Reserve), resolves falls (dead below 0.01, or below −0.15 of the maximum with a physician; saved otherwise below 0.01), writes health back as a word, and settles the return with the company's thirteen records; `ArenaMatchPlugin` runs the tick in play mode with an opponent. The hire list (`hirelings`) draws a candidate's profession and level from the counter, renews daily, snaps a trainer's skills and trims a combatant's loadouts, and picks the loadout a fighter takes into a match (half its levels capped by the tier, two under for a Captain's wing), repairs a broken list, copies a loadout up on a rank and values an outfit; the arsenal (`arsenal`) shelves items by kind. The staff (`staff`) hires with the purse, room and duplicate checks and their messages, recruits for 12 copper, stows and releases kits, renumbers fighter ids, and on a leaving drops the id from the entries and the log's saves; the daily report (`report`) titles the day and writes the log, training, rank, hire-list and tournament lines. A roster row dragged onto another takes its place and the moved ids follow in the log, the entries and the order list; the day's lessons teach the manager, then the fighters. An item sold from the arsenal (`arsenal`) leaves every box and holder when unique, else one copy (a 0x200 copy first), the holders keeping what is left. The company windows (`manage`): roster labels and name inks, Recruit and Hire closed at eight fighters and ten employees, report-line clicks, the arsenal form's pages, selection, steps, dismissal, rank slider and points line, record titles and texts, Company Status. A tournament event (`tournament`) settles the events the company skipped with a rival of 2 to 7 (2 to 6 when the return to the hub settles them on a tournament day), runs rounds less one as extra rounds and fills its bracket with `sides << (rounds − 1)` teams; forfeiting the tourney empties the open events; arriving at the hub and the rematch lineup are in. No hub screen draws the windows yet. The name klean selects emblem background 52 and design 81. A practice fight (`practice`) advances by one and stops at 8: novice names opponent01 through opponent08, expert names opponent09 through opponent16, and red gains a tenth of the maximum. The partner is not spawned. An unwatched match draws a team from `(count × 0.5 + 0.5) × strength sum`; mode 16 leaves the player's team out. A bracket clears losing ids. The first group keeps a forced id and does not step the roll. A later group keeps the team whose weights pass the high half of the weight total times the stepped roll. A tournament field draws 16 of the 19 companies. Roll 0 ends at 0x4b157bf0. Roll 1 ends at 0xead32bb1. A day chooses six of seven templates. Rolls 0 and 1 end at 0x1210d4e5. Roll 4 ends at 0xb761a41e. A performance counter replaces the roll by reversing its low 32 bits. A copied match multiplies the fee and the prize by one more than the slot. A seeded unit draw tempers 624 words. Seed 5489 starts at 0xd091bb5c. A record copy writes plain fields, the gaps, and the tail. Managed strings are not in. Before a match the first specialist under 700 health is removed and the death count rises. Health from 700 through 64999 leaves without that rise. Health of 65000 stays. A combatant stays. Kit release is not in. In-game menu command 4 in arena dismisses a child whose flags include 0xce100000. That child releases a capture and clears style bit 0x20 when it holds one. A remaining capture publishes its code and name. Bit 0x8000 passes its low byte as the shape. Shape 0 shows the default image when it differs and is not zero, and stamps the clock. Shape 7 shows its record. The child clears a hover, closes the task's other children that name it, then closes its own children from the first to the last with argument 0, unless it is already closed. After the lists, a focus is counted once before argument 1 copies ten hand words, when a focus is set or the mask shares bit 0. A session mark of 1 becomes 0, a flag becomes 0, and a code becomes 4. Side 0 copies sixteen words from the first bank when its flag is set. A ready install of index 1 shows that record once and runs its hook. Index 0 shows a fallback and stamps the clock. A shown value is passed with two platform words. A shown value activates the action set. The Steam library is not loaded. The gate hook sets its flags to 1. The watch hook stores a stamp of 0 without bit 0x2000, and a leave enters the session. A clear gate counts a pick. A taken item sets flags 5. A hold byte does not pick. A parent equal to the world counts an open. A fast aim copies its place. A miss with the hold byte clears the aimed word. A taken item sets click flags 0x11. A hover stores flags 0x15. A record of kind 0x10010a or 0x10010d whose link is the world counts one open. A different link does not. An item id of 0 does not count a kind call. Bit 0x80000000 searches the high table and bit 0x40000000 searches the wide table, after those bits are dropped. A span that holds 0x20000 counts one kind call. A cursor at the view edge clears the hit. A marked item drops a held count above 0, and byte 1 closes a pointer. The first pane tagged 0xce1c2013 is shown. A scene keeps the last positive body. A new pane doubles the row before the slot and clamps the place. The ray starts at 4000 along z. A centre at 100 leaves reach 100, and a centre at 500 is left. A centre child sits at half the spare. A free camera ray, nested children, and the focus method body are not in. A cleared mark registers code 0x8001 unless the hold byte is set. Raising the mark drops the hooks. An open world sets the side from its byte. Raising the mark to 1 installs index 0 and does not copy a bank. A mask without a focus does not count it. A shown child hides and the redraw word gains bits 0 and 1. A pinned child raises a notice and still closes them. The command then runs that leave, asks for a save that does not abort on low health, marks the hub busy, and sets the phase to transitioning. Drawing the cursor image and the other menu commands are not in. Menus, the other emblem controls, and the original's packages are not.

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
interval's present mode (immediate or plain Fifo) and the Alt+Enter toggle.
Tests pin the arithmetic and the orders. `hw_config` applies the full-screen
rule and the present mode, `exarust::frame_loop` the pacing, the focus key
release and Alt+Enter; the other models document the original's loop, which
winit's event loop replaces. Rules:
[`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md) (Frame pacing) and
[`docs/UI.md`](../../docs/UI.md) (Settings options).

The save game (`session`, `slots`) writes `ExanimaNNN.rsg` or
`ArenaNNN.rsg` packages in the original's container: `#playerdata` with
the story or arena header, the journal chunk and an Exarust chunk holding
the session (mode, level, journal, used topics, company, roster, focus,
clock, seed, start background); saving over a save of the same mode keeps
its character placement, its other sections and the chunks Exarust does
not produce. Listing a folder opens each file as the original does
(resetting a bad header), lists a number once, deletes only story slots
with an older signature, orders the rest by the `Exanima.set` key and
moves a loaded slot to the front; a load opens the first file that exists;
a new slot takes the lowest free number at the front; deleting removes
both files. A checkpoint (`.rcp`) is written once per slot by copying the
save, and a manual or transition save refreshes it; a death deletes the
save and drops the slot unless a checkpoint exists, which the next load
copies back. The original's world databases, character placement export,
discovery and level sections are not written yet, so a save made from
nothing is not loadable by the original.

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
non-repeating random variant are in. The creator icon sheet is four
96-pixel cells at a scale of eight units. The form's labels and page
captions live in `exa_ui::creation`. Drawing the form, loading the model
preview and sounds, the object database behind the kit, record
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
The reconstruction device starts the scan (`teleporters`); nothing
starts the shell, bubble or wheel yet: their callers (the displacement
powers, the portal screen's journey and the power tree) are not in. Drawing the flame ribbon's vertices, the shell and scan
mesh copies, the bubble and wheel geometry, the portal particle
simulation and the bubble's GI light values is not in.

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
look by its ramping weight (its anchors then on the physical thorax), with
identity slots for the femur posture, the IK chain and the foot pitch; the
clock of a locomoting body takes the locomotion's step; a get-up's clip ending into the stance writes the condition
word normal; sampler); its targets, the sampled parts on its character
frame, and its lift, its controller told to follow the frame (carry); a body asked to go limp keeps its motion unless given a
velocity (`limp`),
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
and of corpses through the solver's grab, the reach under the cursor and the
refusal that keeps the movement request; `docs/PLAYER.md`, The hand), the health
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
ending with the condition word and the swing's rate, strength and state
word), and the debug tools ([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)):
the body overlay and joint readout (`body_overlay`, with the feet's
anchors and states), the joint trace
(`joint_trace`), the trace of contacts between body parts (`pair_trace`)
and the simulation controls (`sim_control`, with the scripted pauses at a
tick, a stretched link, the first blood jet, the player's first landed
blow or the first collapse).

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

## Scope

- App states (boot → menu → loading → in-game).
- Arena: companies, fighters, matches, rewards; Story: levels Underworld 0..8; Practice/sandbox.
- Save games in the original's package and player-data shape, with our own
  payload for the session state.

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
