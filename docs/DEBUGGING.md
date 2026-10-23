# Debugging bodies

Tools that show and measure what the body solver does
([formats/RSQ.md](formats/RSQ.md), Physical body): an overlay of every
simulated character body and every awake prop body, a readout of one
body's joints, a per-joint trace, a shove for the props near the player, and
controls to pause, step and slow down the simulation. They cost nothing
measurable while off: their systems wait behind run conditions, and the
only per-frame work is a check of their keys.

## Quick start

```bash
# The duel with the overlay on; F5 over the opponent inspects it.
exarust --debug-bodies --opponent play arena
# A limp body falling and settling, a quarter as fast.
exarust --debug-bodies --limp --sim-speed 0.25 character
# Every joint of both fighters for a minute, in a CSV.
exarust --opponent --joint-trace joints.csv play arena
# Shove the tableware on arenao1's tavern table (the table itself is level
# geometry), seen beside the table: the log says how far each moved once
# all sleep again.
exarust --no-gi --no-supersample --size 960x540 --at 11.2,4.5,12.8 \
  --camera 10.2,8.2,12.2,-135,-32 --shove-props 60 --frames 450 \
  --screenshot shoved.png play arenao1
# A blow and the get-up, repeatable tick for tick: run two builds and
# compare the traces whose first tick and blow row match.
exarust --fixed-frames --shove 60 --trace t.csv play arena
# Freeze the duel the moment a link opens 1 cm past its slack, and keep a
# picture of it from the side.
exarust --debug-bodies --opponent --pause-on-stretch 1 \
  --camera 5.5,1.3,4.5,90,-6 --screenshot fold.png --frames 30 play arena
```

In `play arena`, which has no opponent's start, the duellist stands 2.5 m
in front of the player, facing it (`--at` moves both); in an arena match
it takes the ring's other slot.

## Debug commands

The original has a named debug-command registry: units register a name and
a handler, a line is split at its first space, the command is matched with
ASCII case ignored, and the handler gets the rest. Two commands are
registered: `cshd` (reload the shader programs) and `msm` (smooth a
scene's meshes). The settings file's `KeyDef` lines hold lists of such
lines; Exarust runs them one per frame, which is a guess. `exa_game::commands`
is that registry and queue; `exarust` registers `cshd`
(`exa_render::shader_reload::reload` asks the asset server to reload the 12
embedded shaders) and `msm`, whose handler only logs: the subdivision itself
is the model view's `--msm`. Example:

```
shift+S: cshd; msm
```

## Flags

| flag | effect |
|---|---|
| `--debug-bodies` | start with the overlay and the readout shown |
| `--joint-trace <file.csv>` | write every joint of every simulated character body after every 83 Hz tick ([columns](#joint-trace)) |
| `--pair-trace <file.csv>` | write every contact between two body parts that each 83 Hz tick's body–body detection found ([columns](#pair-trace)) |
| `--corpse x,y,z` | in play mode, a dead human (a new record, in underwear) lies down here as a level's placed dead do (metres); repeatable. For contacts with corpses outside the story levels, e.g. `--corpse 0,0,3.4 --press W@150-420 play arena` walks the player over one |
| `--trace <file.csv>` | the body trace in the retail probe's columns ([formats/RSQ.md](formats/RSQ.md), the foot contacts' states and anchors among them), then Exarust's own (the sleep countdown and state, the frame, the blend and hold, the clip clock's `rate` this tick, its `progress` and `clip_end` (1 on the tick its end test fires), the ankles), the condition word in hex (`0x300` getting up), the swing: `swing_rate` (frames the attack's clip ran this tick), `strength` (P) and `attack_flags` (the state word in hex: `0x2` stalling, `0x10` released, `0x10000` chained, `0x80000` no Remise), and last the locomotion ([Movement](#movement)): `loco_flags`, `moving`, `free_walk`, `yaw`, `heading`, `turn_rate`, `align`, `speed`, `reach`, `stop`, `goal_x`, `goal_z`; character bodies only |
| `--trace-all` | with `--trace`, every simulated character body is traced, the player's among them (by default only the player's when there is one), e.g. to follow a duellist's distance from the player across `root_x`/`root_z` |
| `--shove <frame>` | a body blow to every simulated character body at that frame after the load |
| `--shove-props <frame>` | in play mode, at that frame after the load, a blow to every prop body within 3 m of the player (1.5 m/s away from it and 1.5 m/s up); the log counts them and, once all sleep again, says after how many ticks and how far each moved (a body that fell out of the world counts as settled). Props meet the level and, through the pairs, every other body: they knock into each other, and one landing on a table that is itself a body (exanima01's tables) rests on it |
| `--sim-speed <f>` | run the simulation at this speed (0.25 is a quarter as fast), from the end of the load |
| `--fixed-frames` | every frame advances game time by exactly 1/60 s, whatever it took, so the clips played by frames, the frame counts of `--shove`, `--press` and `--click`, and the ticks each frame runs repeat from run to run. Loading still takes a real time, so the tick a body is built and the frame the load ends can move by a few: two runs whose `--trace` starts on the same tick and whose blow lands on the same row give the same rows, and two builds compare row by row (only the `body` column, an entity id, may differ) |
| `--pause-at-tick <n>` | pause at the end of body tick `n` (the `tick` column of both traces, counted from the start, the load included; see below) |
| `--pause-on-stretch <cm>` | pause at the end of the tick in which a link of a body first goes this far past its slack; the log and the readout name the body, the part and the link |
| `--pause-after-jet <n>` | pause at the end of the `n`th body tick after the tick that first finds a blood jet started (a bleeding blow's, or `--debug-spray`'s), to catch a jet in flight ([Wounds](#wounds)) |
| `--pause-on-hit <n>` | pause n ticks (at least 1) after the tick in which the player's first blow lands, for a picture of a landed punch or cut; the log names the tick and the struck bone |
| `--pause-after-collapse <n>` | pause n ticks (at least 1) after the tick in which a killing blow first starts a body's collapse (its condition `0x800`), for a picture of the collapse that long after the blow (83 ticks is 1 s; death comes on the 250th); the log names the body and the tick ([formats/RSQ.md](formats/RSQ.md), The collapse) |
| `--press key@frame,...` | press keys or mouse buttons at frames counted from the end of the load (`.`, `[`, `]`, `Tab`, `Ctrl`, `Shift`, `F1`–`F12`, a letter, or `LMB`, `RMB`, `MMB`), e.g. `.@400,.@401,P@500`; each is released the frame after; `key@a-b` holds it from frame a and lets it go at frame b, e.g. `RMB@60-400` |
| `--click frame,...` | click the left mouse button at frames counted from the end of the load (the same as `LMB@frame` in `--press`): with `--weapon` in an arena fight, which starts in combat mode, the player attacks unattended |
| `--camera x,y,z,yaw,pitch` | in play mode, a fixed camera there instead of the chase camera (metres, degrees), for unattended shots; with `--fov` |
| `--cursor x,y` | in play mode, the cursor stays at this window point (logical pixels, from the top left) whatever the mouse does: the world cursor, walking to the cursor and combat facing use it ([UI.md](UI.md), World cursor) |
| `--health yellow,red` | in play mode, the player starts with this yellow and red health (a human's full health is 0.25), e.g. `0.176,0.226` for the retail reference shot of the health bar ([UI.md](UI.md), Health bars) |
| `--heartbeat` | in play mode, the low-health heartbeat may play in any level (the original plays it in story mode only, so never in the arenas) |
| `--undying` | in play mode, the player's yellow and red health are refilled to the most after every frame's blows and every body tick, so long scripted fights (`--opponent`) run on while the player stands idle; a single blow from full health can still kill |
| `--feral <f>` | in play mode, the player's record has feral level f (0 to 1): it spawns with a feral character's stats and rule bits (logged: "player: feral …, rule bits …, most energy …"), so blows take nothing from its red health and their red share comes off yellow ([formats/OTHER.md](formats/OTHER.md), Applying damage); its most energy is 1 − 0.1 f, its head loose |
| `--perks a,b,...` | in play mode, the player has these perks (names as the character records give them, case aside: `remise`, `feint`, `fend`, …; an unknown name stops the launch): Remise chains a held side attack into the other side's past 0.8 of its clip, Feint renews a released swing ([PLAYER.md](PLAYER.md), Combat mode); the log names them |
| `--still-opponent` | `--opponent`, but the duellist stands its ground: it faces the player and takes its stance (by its distance, [PLAYER.md](PLAYER.md), The combat stance) but never walks or strikes; with `--at` it shows the stances at a chosen distance |
| `--arena-match <id>` | in an arena level, `--opponent` laid out as the original's match template `<id>` (1 duel, 2 doubles, 3 skirmish, 4 fray, 5 reserve, 6 pugilism, 7 elimination, 8 challenger, 9 captain, 12 brawl, 13 valiance, 14 valour): the player takes the company's first place on the arena's spawn spot and a duellist stands on each opponent's place, all facing the spot; the company's other entrants and the beast are not spawned. The match tick logs each round's end ("arena match: round 0 won -> state 4"); with `--opponent` alone the match is a duel ([formats/OTHER.md](formats/OTHER.md), Arena matches) |
| `--debug-wound bone,radius_cm[,dx,dy,dz]` | a test wound on the player's or the character view's body ([Wounds](#wounds)); repeatable |
| `--debug-nav` | start with the navigation grid drawn ([Navigation grid](#navigation-grid)) |
| `--debug-path x,y,z` | play mode: plan a path from the player's feet to this point (metres) with the planner, again whenever the player has moved a metre; log each plan and draw it ([Navigation grid](#navigation-grid)) |
| `--msm` | in the model view, every mesh smoothed once as the original's `msm` debug command smooths a scene (Loop subdivision, [formats/RFC.md](formats/RFC.md), Editable meshes), e.g. `exarust model CharExt "hair hula01" --msm` for the hair the creation scene smooths; warns of meshes with too many faces |
| `--bevy-sun-cascades` | the sun keeps Bevy's two cascades in terrain levels too, instead of the original's one fitted map, to compare the two ([RENDERING.md](RENDERING.md), Sun shadow fit) |
| `--weapon-blood <a>` | the player's or the character view's held weapon holds blood a (0 to 1) from the start, so its stain shows without a fight ([Wounds](#wounds)) |
| `--debug-spray bone,R[,dx,dy,dz]` | a blood jet from the player's or the character view's body part every second, as a bleeding blow of size R swinging along d would spray ([Wounds](#wounds)); repeatable |
| `--debug-arcs` | in play mode, the Shock power's arcs on the player's right hand, kept while the run lasts: they end at the level and shock other fighters ([THAUMATURGY.md](THAUMATURGY.md), In the game) |
| `--debug-barricade` | in play mode, a force barricade raised 2 m in front of the player once its body is up: it rises, ripples, shoves and knocks back bodies, and falls once the player is 20 m away ([THAUMATURGY.md](THAUMATURGY.md)) |
| `--debug-wisps` | in play mode, the constant displacement's wisps on the player's body parts ([THAUMATURGY.md](THAUMATURGY.md)) |
| `--powers id,id,...` | in play mode, the powers the player knows (ids in hex or decimal, e.g. `0x20206,0x401`), filling the bar's slots in that order: C opens the bar, a digit picks and Q casts through the real front ends ([THAUMATURGY.md](THAUMATURGY.md), In the game) |
| `--physique x,y[,h,a,f,w]` | the character view's body (or the player's) and its apparel morphed by this physique: the two physique values, then height, age and feral level (0 when left out) and female when w is not 0 ([Physique](#physique)) |

## Movement

The movement decision is measured with `--trace` and scripted keys
([PLAYER.md](PLAYER.md), Movement; [formats/RSQ.md](formats/RSQ.md),
Movement decision). A light run that walks, stops, turns, jogs, runs to a
far cursor with Shift, backpedals and walks into the arena wall:

```sh
exarust --no-gi --no-supersample --size 960x540 --fixed-frames \
  --trace t.csv --cursor 480,40 --press W@100-250,D@400,D@430,A@520,\
W@650-700,Shift@650-700,W@800-900,A@800-900,RMB@1000-1150,\
Shift@1000-1150,S@1250-1300,W@1400-1700 play arena
```

The trace's last columns tell each tick's decision: `loco_flags` (hex:
`0x40` walking by the keys, `0x44` with the run bit after W + Shift, `0xc`
run and sprint latch with the mouse beyond 3 m and Shift, `0x70` the soft
stop braking, `0x42` backpedalling, `0x10`/`0x50` braking at a wall),
`moving` and `free_walk`, `yaw` and `heading` (degrees), `turn_rate`
(degrees a second: 120 for the standing quarter turn, 240 for a free walk
at rest, 300 in combat mode or with the mouse beyond 3 m, less over the
last 0.6 rad), `align` (the alignment, 0.4 to 1 moving), `speed` (cm a tick: 1.687 walking, 4.337
running, 1.476 backpedalling), `reach` (cm to the goal, 200 in free walk)
and `stop`: `moving`, `standing`, `braked` (a braking body under 1 cm a
tick), `arrived` (within 25 cm at 0.5 cm a tick), or the gate that held
it (`tired` after a blow until the energy is full, `cooldown`,
`stepping` during the interim combat footwork), then `goal_x` and
`goal_z` (cm, the request's goal: the look-ahead's end clamped by the
grid, or a soft stop's). Released, W walks on to its walk's key segment 0
or 4 (the `clock` column passes 0 or 40–50), then `0x70` and `braked`
about 30 cm on. On the third behaviour tick with a grid the log says how
far the spawn snap moved the player (`spawn snap:`).

## Attacks

An unattended attack: `exarust --opponent --sim-speed 0.25 --camera
4.2,1.4,0.6,90,-8 --press LMB@100-300 play arenao1` throws the unarmed
player's right straight at the duellist (held through the swing: a
one-frame press is a click, which checks it), seen from the side (an arena
fight starts in combat mode: Tab would turn it off). The log names each
attack: its moveset, the game time (ms, 12 a tick), how far left of the
facing the cursor was, the energy, the last attack tick and the cooldown
it waited; and "attack waits: energy …" when a standing request waits for
the energy to come back. With the chase camera and `--cursor` left of the
player (e.g. `--cursor 650,280 --press LMB@2`, before the body has turned
to the cursor) it is a left straight. A held button repeats the attacks,
330 ms after each one's last tick unarmed: `--press LMB@150-1500`. A press
of one frame (`LMB@100`) is a click: the button is up by the next tick, so
the swing is released and checked at once (the log: "attack released at
0.01 of its clip …: stall 0.300 a tick, stance at … ms", then "released
attack back to Cmbt - Idle"); hold the button to swing through. A double
click (`LMB@200,LMB@212`) releases the first attack and throws the
overhead at once ("… after a release"). With `--perks remise` a held side
attack chains the other side's past 0.8 of its clip (a punch after 55
ticks); a fighter struck mid-swing logs "struck mid-swing at … of its
clip: stall … a tick, … stance at … ms". The `--trace` columns
`motion`, `swing_rate`, `strength` and `attack_flags` give each attack's
length in ticks and its stalls.

Punches land from close: the duellist strikes from 1.2 m, where the
straight falls short, so step in (`W@100-1500,LMB@150-1500`), or stand a
still duellist close. Each hit's log line says whether a fist or a weapon
struck:

```bash
# A right straight landing on a still duellist 0.86 m away (chest to
# chest), paused three ticks after it landed, from the player's right
# (the button held, so the punch is not released).
exarust --still-opponent --at 0,0,-1.1 --camera 4.2,1.4,-1.0,90,-8 \
  --press LMB@120-200 --pause-on-hit 3 --screenshot punch.png --frames 10 play arenao1
# Held punches, a click, a double click and Remise's chains, traced.
exarust --no-gi --no-supersample --size 960x540 --opponent --perks remise \
  --trace t.csv --press LMB@200-600,LMB@700,LMB@760,LMB@772 play arenao1
```

The stances: the log names every change of the player's stance (with its
target's distance and the time since its last attack) and the duellist's
(with its distance and reach); `RUST_LOG=info,exa_combat::stance=debug`
also logs each clip the body tick switches to. `exarust --still-opponent
--at 0,0,6 --camera 8,1.3,2,90,-6 play arenao1` shows both fighters in
their guard 8 m apart, `--at 0,0,1.7` the duellist in its combat idle
while the player still guards (3.7 m), `--at 0,0,1` both in the combat
idle.

The log names the player's and the duellist's encumbrance at spawn, with
the run blend it allows and the step reach it leaves; `--outfit` gives the
player armour to weigh (`--outfit fptdhrnsa,fptdhelma,fptdlegsa`, a tourney
plate set: 0.625, run blend at most 0.8125; formats/OTHER.md, Encumbrance).

A body striking a character's limb hard enough to hurt it is logged at
info ("… bone b struck by …: impact …, damage …"). With
`RUST_LOG=info,exa_combat::impact=debug` every candidate impact is logged
first: the struck character's name, whether it lives and its condition,
the part, the striking body's name, bone, sleep state, countdown and
settling, and the pair's impact, squared travel, hits and push. A level's
corpse takes and deals none until its body first sleeps (it is still
falling into the pose the original places it in).

The world cursor's shape changes are logged at debug level
(`RUST_LOG=info,exa_game::cursor=debug`), and its size once it is drawn:

```bash
# The red ring by the player's feet in combat mode, which the arena fight
# starts in (with --frames 3000 the duellist has killed the player, and the
# red arrowhead shows).
exarust --opponent --cursor 808,452 --screenshot ring.png --frames 200 play arena
# Running to the cursor: two amber chevrons pointing up the screen.
exarust --press RMB@60-400 --cursor 800,120 --screenshot run.png --frames 200 play arena
```

The player's health changes are logged at debug level
(`RUST_LOG=info,exa_game::hud=debug`, "player health: yellow …, red …"),
as is every repaint of the health bars with its fade
(`exa_ui::hud=debug`) and the tick a fighter's regeneration reaches its
full rate, 40 s after it was last held or hit (`exa_combat::regen=debug`,
"regenerates at its full rate"); a heartbeat's start is logged at info
level:

```bash
# The health bar as in the retail reference (yellow 0.176, red 0.226), the
# player stood so that the floor lies under the bars.
exarust --at 0,0,1 --health 0.176,0.226 --press Tab@50 --screenshot bars.png --frames 200 play arena
# The yellow refilling out of combat mode: wounded (0.085 of red 0.203),
# combat mode off at frame 50; about 40 s later the bar is yellow to 0.18.
exarust --still-opponent --at 0,0,1 --health 0.085,0.203 --press Tab@50 \
  --screenshot heal.png --frames 2500 play arenao1
# The bars fading out: combat mode off at frame 300, 1.2 s later at 60
# frames per second (frames, not seconds: a slower run shoots later).
exarust --at 0,0,1 --press Tab@50,Tab@300 --screenshot fade.png --frames 372 play arena
# The heartbeat after the duellist's first blow (about 2.7 s after the fight
# starts, in combat mode); its border pulses for 7 s, faint at the start of
# every 800 ms, so a shot can need a second try: the log says when it
# started.
exarust --heartbeat --opponent --health 0.06,0.25 --screenshot beat.png --frames 200 play arena
# A feral player in a duel: every hit's log line keeps red at 0.250 while
# yellow falls by both shares; the killing blow collapses it as usual.
RUST_LOG=info exarust --feral 0.5 --opponent play arena
```

A fall to death logs each step at info level with its body tick
(`exa_game::fall`: "screams as it falls", "is lost", "hears the pit",
"falls to its death"), and the kill plane "fell out of the world":

```bash
# The player dropped 200 m over the arena: lost about 45 ticks after it
# loses its balance, the pit and the fade 84 ticks later, gone 334 ticks
# after the loss (the trace ends there).
RUST_LOG=info exarust --no-gi --no-supersample --size 960x540 --at 0,200,0 \
  --trace drop.csv play arena
# Halfway through the fade (1.5 s of game time after the pit), paused.
exarust --no-gi --no-supersample --size 960x540 --at 0,200,0 \
  --pause-at-tick 300 --screenshot fade.png --frames 30 play arena
```

With `--pause-at-tick`, `--pause-on-stretch`, `--pause-after-jet`,
`--pause-on-hit` or `--pause-after-collapse`,
`--screenshot` counts its `--frames` from the pause, so the picture shows
the paused body:

```bash
# The player one second into its collapse, from behind it: with a fifth
# of a human's health (0.05 of 0.25) the duellist's first blow kills it.
exarust --no-gi --no-supersample --size 960x540 --opponent --health 0.05,0.05 \
  --camera -0.25,2.4,7.0,0,-42 --pause-after-collapse 83 \
  --screenshot collapse.png --frames 10 play arena
```

The traces start while the loading screen is still up (bodies tick from
the moment they are built; the log's `loading took` line marks its end). A
pause before then holds the loading screen, since the lighting waits for
fires that run on the game clock, until P resumes or the load's one-minute
timeout passes; the log warns of it. Pick a tick after the load.

## Keys

| key | in any mode |
|---|---|
| F3 | show or hide the overlay and the readout |
| F4 | draw every body, or only the inspected one |
| F5 | inspect the body under the cursor, or the next body when none is under it |
| F6 | show or hide the navigation grid |
| F7 | start every keyframe track of the level ([Keyframe tracks](#keyframe-tracks)) |
| P | pause or resume the simulation |
| `.` | step one tick (pausing first if running) |
| `[` / `]` | slower / faster: ×0.1, ×0.25, ×0.5, ×1, ×2 |

Pausing stops the fixed step: the character controllers, the character
ticks (clips advance with their body's ticks) and the body solver, and
everything else on the game clock (Bevy's virtual time). Rendering goes on,
and the cameras run on real time, so a paused or slowed body can be orbited
(middle mouse in play mode, right mouse and WASD in the other views). The
bodies stay drawn where they were, between their last two ticks.

## The overlay

Drawn over everything, between the last two ticks as the meshes are, for
every character body and every awake prop body (the thousands of sleeping
props a level holds are left out; F5 picks only character bodies):

- each part's four particles (small crosses) and the six edges of its
  tetrahedron, thin and pale;
- each link from its point on the part to its point on the parent part:
  **green** within its slack (or at most 1 mm past it, the solver's
  softness), **yellow** up to 1 cm past it, **red** beyond; a link past its
  slack also shows its slack as a sphere around the parent's point, the
  room its point has;
- each pivot (a link without slack, every part's origin pin) as a cross in
  the same colours, with a segment when it has opened;
- a living body's link muscle anchors, faint cyan: where the clip's joint
  turn wants each link's point, with a line from the point to it;
- a living body's foot contacts: a line from each ankle to the anchor its
  foot is held at and a cross on the anchor, **aqua** while the foot
  touches (it grips there), **orange** while it is anchored (stepping to its
  stance spot), **grey** while it does not touch (walking, lying, or not
  yet on the floor) ([formats/RSQ.md](formats/RSQ.md), Feet);
- a white dot over the inspected body's head.

## The readout

The top-left panel shows the inspected body: the player's, or the one
picked with F5.

- The body: its `id` (the traces' `body` column), living or limp, its
  sleep state and countdown, support, energy, tone and balance, and each
  foot's state (hex, as the trace's `foot_*_flags`) and grip.
- The simulation: the tick, the speed, and whether and why it is paused.
- A row per part, by bone:

| column | meaning |
|---|---|
| total | the joint's turn from its rest, degrees |
| swing | the part of it that tilts the bone axis |
| twist | the part of it about the bone axis, signed (right-handed) |
| swing axis | the swing's axis in the part's own space |
| clip | how far the turn is from the one the clip asks for (a living body's) |
| range | the largest share of its slack any of the part's slack links uses (100 % at the joint's limit), and that link |
| pivot | how open the part's origin pivot is, mm |
| past slack | how far the part's worst link (pivot included) is past its slack, cm |

A row is yellow or red when its worst link is.

### What the angles measure

- The **rest** is the model's rest pose: each part's turn relative to its
  parent in the body model. The joint's **turn** is the rotation that takes
  the part from there to where its particles put it now, in the part's own
  space (posed = rest · turn).
- The **bone axis** runs from the part's origin (its joint) to the mean of
  its child parts' origins, or to its centre for a hand, a foot or the head.
- **Swing and twist** split the turn into a twist about the bone axis
  followed by a swing that tilts it: turn = swing · twist.
- The clip's turn is split the same way in the joint trace (`clip_*`).
- A joint's range is set by its links, not by angles: a link's point on the
  part must stay within its slack of its point on the parent. On the human
  skeleton, for example, each forearm has a link 12 cm out along its Z axis
  with 12 cm of slack, centred 60° round its Y axis from the straight arm:
  the elbow bends from straight (0°) to 120° about +Y on the right arm, −Y
  on the left, and the model's rest pose has it bent about 21°. A swing
  about the other way of Y, by more than the rest's 21°, is an elbow bent
  backwards, and its link reads over 100 % of range.

## Joint trace

`--joint-trace <file.csv>` writes one row per body part per tick, after the
solver, for every simulated body; a body in deep sleep is left out. Angles
are in degrees, lengths in cm unless named otherwise.

| column | content |
|---|---|
| `tick`, `body` | body tick since the start; the body model root's entity bits (the readout's `id`) |
| `player`, `living` | 1 for the player's body; 1 while the body has its muscles |
| `bone`, `name`, `parent` | the part's bone, its name, its parent part's bone |
| `total_deg`, `swing_deg`, `twist_deg` | the joint's turn from its rest, split as above |
| `swing_axis_{x,y,z}` | the swing's unit axis in the part's space |
| `anchor_error_deg` | how far the turn is from the clip's (living) |
| `clip_total_deg`, `clip_swing_deg`, `clip_twist_deg`, `clip_swing_axis_{x,y,z}` | the clip's turn from the rest, split the same way (living) |
| `worst_link`, `worst_gap_cm`, `worst_slack_cm`, `worst_excess_cm` | the part's link furthest past its slack (its place among the part's links, or `pivot`), its gap, slack and excess |
| `range_pct`, `range_link` | the largest share of its slack a slack link of the part uses, and that link |
| `pivot_gap_mm` | how open the part's origin pivot is |
| `shoved`, `shove_speed` | 1 when a blow (a hit or `--shove`) landed on the part in this tick, and its speed (m/s) |
| `sleep`, `countdown` | the body's sleep state (`awake`, `asleep`, `deep`) and countdown |
| `action`, `clock` | the body's clip and its clock (frames) |

A duel writes about 0.4 MB a second (36 rows a tick).

## Pair trace

`--pair-trace <file.csv>` writes one row per pair of body parts in contact
per tick, as the tick's body–body detection found them
([formats/RSQ.md](formats/RSQ.md), Contacts between bodies), after the
solver. A tick without contacts writes nothing.

| column | content |
|---|---|
| `tick` | body tick since the start |
| `body_a`, `bone_a`, `living_a` | the side that brought its spheres (the more mobile part): its body model root's entity bits, its part's bone (a prop's part index), 1 while its body has muscles |
| `body_b`, `bone_b`, `living_b` | the touched side, the same way |
| `own` | 1 when both parts belong to one body (a limp body's arm on its thigh) |
| `hits` | how many of A's spheres hit B's |
| `push_cm`, `deepest_cm` | the length of the hits' pushes summed, and the longest push |
| `x`, `y`, `z` | the mean touched point (cm) |

In `play arena --opponent` (the practice duel) the only contacts are a
body's own: the duellist's right hand on its thigh at the start (under
2 mm), the dead player's arms on its trunk and thighs after the kill (at
most 1 cm). With `--corpse 0,0,3.4 --press W@150-420 play arena` the
player's lower legs meet the corpse's thighs, chest and arms for 45 ticks
and wake it.

## Cost

Measured on the RTX 4000 without vsync, 3000 frames of the character view
at 800 × 450 without supersampling, GI or shadows (CPU-bound, about 340
FPS), two runs each:

| run | ms per frame |
|---|---|
| the build without the tools | 2.89, 2.95 |
| tools off | 2.90–3.04 |
| joint trace | 2.96, 3.01 |
| overlay drawn, readout hidden | 3.01, 3.05 |
| readout (Bevy UI text, about 1800 glyphs) | 3.78, 3.79 |
| overlay, readout and joint trace | 3.84–3.87 |

Off, the tools are within the runs' noise (under 0.1 ms). On, the readout
costs most of the 0.9 ms; in the arena duel with supersampling and GI
(GPU-bound, 59–66 FPS) no difference shows. The readout's rows are read ten
times a second while the simulation runs.

## Wounds

`--debug-wound bone,radius_cm[,dx,dy,dz]` paints a wound on the player's
body (play mode) or the character view's body, once it is dressed and its
skin bound, three frames later, with the painter wounds from hits will use
([RENDERING.md](RENDERING.md), Blood). The flag can be given several times.

- `bone`: the body part, by bone number (1–18) or by name: `pelvis`,
  `ruleg`, `rlleg`, `rfoot`, `luleg`, `llleg`, `lfoot`, `belly`, `chest`,
  `head`, `rshld`, `ruarm`, `rfarm`, `lshld`, `luarm`, `lfarm`, `rhand`,
  `lhand`.
- The contact point is the front of that part: of its visible meshes (the
  skin, or the garment over it), the vertex furthest along the way the body
  faces.
- `radius_cm`: the wound's radius (the original's is 30·R + 10 cm, R the
  blow's lasting damage on the part: 11.5 cm at the bleeding threshold,
  19 cm at R = 0.3).
- `dx,dy,dz`: how far the striking point travelled on past the contact
  point, in cm in the body's frame (+Y up, +Z the way it faces). Without
  it the wound is a ball; with it, a smear that runs on along the travel to
  the end of the mesh, as a blade's follow-through drags blood in the
  original.

```bash
# A chest wound and a cut on the left upper arm, close up.
exarust --debug-wound chest,20 --debug-wound luarm,12 \
  --camera 0.15,1.35,1.05,0,-8 character
# Blood running down from the belly over the braies and a thigh.
exarust --debug-wound belly,15,0,-40,0 --debug-wound ruleg,14,0,-30,0 character
```

The log names each test wound's contact point and, for every wound, how
many meshes it painted and copied and the copies' size. The tool costs
nothing without the flag (its system waits behind a run condition).

Wounds from hits log the same line seven body ticks after the blow; each
hit's line names the struck body, the body tick and the wound's size R
(it bleeds above 0.05). An unattended duel in which both fighters strike:

```bash
# The armed player (in combat mode from the fight start) holds the button,
# attacking again 426 ms after each attack's last tick; paused after the
# duellist's second blow, from the player's front.
exarust --opponent --weapon ub05ssworda \
  --press LMB@100-1400 --camera 1.3,1.7,3.4,135,-14 \
  --pause-at-tick 760 --screenshot hits.png --frames 20 play arena1
```

Each bleeding blow also names the blade that dealt it and the blood it
holds now ("blade … draws blood from wound R …: …"), and the first time a
weapon's blood passes 0.01 the log says how many of its meshes the stain
covers and the tip's height. `--weapon-blood` shows a stain without a
fight:

```bash
# The short sword soaked (blood 1): stained from the tip to below the
# guard; at 0.25 the stain ends 35 cm below the tip.
exarust --weapon ub05ssworda --weapon-blood 1 character
```

Each bleeding blow also starts a blood jet at the blow ([RENDERING.md](RENDERING.md),
Blood sprays); the log names it ("blood jet on … bone …: n droplets,
against a swing of … m/s"). `--pause-after-jet n` stops the simulation n
ticks after the first one, wherever the fight is by then:

```bash
# The duel, paused 12 ticks (0.14 s) after the first bleeding blow: the
# jet's first droplets fly off the struck part against the blade's swing.
exarust --opponent --weapon ub05ssworda \
  --click $(seq -s, 100 25 1400) --camera 1.3,1.7,3.4,135,-14 \
  --pause-after-jet 12 --screenshot jet.png --frames 20 play arena1
```

`--debug-spray bone,R[,dx,dy,dz]` fires a jet from the player's (play
mode) or the character view's body every second, once it is dressed and
bound, without a fight:

- `bone`: the body part, as for `--debug-wound`;
- the jet starts at the front of that part (as a test wound's contact);
- `R`: the size of the bleeding wound it stands for (over 0): 36·R
  droplets, 60 at most (R 1 sends 36, over 0.9 s);
- `dx,dy,dz`: how the striking point swung, in the body's frame (+Y up, +Z
  the way it faces; any length); the jet runs against it. Without it
  `0,0,-1`: a blow driving into the front, so the jet sprays out of it.

```bash
# A full jet from the chest toward the body's right, in the lit arena,
# 45 ticks after it starts.
exarust --debug-spray chest,1,1,0,0 --camera 1.3,1.7,3.4,135,-14 \
  --pause-after-jet 45 --screenshot spray.png --frames 20 play arena1
```

Droplets darken what lies behind them, so they show over bodies, walls
and floors and not against black. The tool costs nothing without the flag.

## Navigation grid

The level's navigation grid ([NAVIGATION.md](NAVIGATION.md)) is built when
the load ends; the log names its inputs and counts (`navigation grid:` and
`navigation grid built:`). `--debug-nav`, or F6 in any view, draws it
around where the camera looks:

- every floor cell within 8 m, a square 2 cm above its floor, 24 cm across;
- the lower layer green where its clearance is 0, through yellow at 180
  (the most a walker of size class 0 passes), red beyond;
- the upper layer (a floor over another: tables, galleries, bridges) cyan
  through blue, magenta beyond;
- cells with no floor, and open sky (wall tops, anything higher than 1.5 m
  under the level's grid top), are not drawn;
- the player's last line walk on the grid from its feet: the way asked
  faint (white the look-ahead, yellow a soft stop's goal, cyan a footwork
  step's target), the walk green when free and orange when blocked, with
  a ring at its end, and a magenta cross on a goal it walks to.

`--debug-path x,y,z` plans from the player's feet to the point with the
planner (`exa_world::nav::find_path_with`: size class 3, penalties doubled,
stopping at the goal's cell, the player's own footprint lifted while it
plans), again whenever the player has moved more than a metre, logs each
plan (`debug path from …`: points, cost, reached or ends short, then each
point) and draws it: yellow when it reaches the goal, orange when it ends
short, a ring at each point, a cyan cross at the goal. Nothing walks it.

`exarust --debug-nav --no-gi --no-supersample --size 960x540 play arena`
shows the arena's floor cut by its walls. Without a window, `exa-tool nav
<level> --png nav.png` builds the same grid, prints its counts and draws
both layers side by side, a pixel per cell (X right, Z down), shaded by
the floor's height:

```text
$ exa-tool nav exanima01 --png nav.png
exanima01: 640 × 800 cells, voxel 0 tops out at -22.10 m
inputs: 13384 meshes (4590297 points), 869 floor meshes, 20 terrain tiles; built in 236.29ms
columns by spans (0–3): [190963, 320141, 896, 0]
Lower: 219292 floors, 219225 passable (size class 0), 101745 open sky
Upper: 792 floors, 792 passable (size class 0), 104 open sky
```

## Keyframe tracks

Scene nodes can carry keyframe tracks that move them
([formats/RFC.md](formats/RFC.md), Keyframe tracks); the original starts
an object's tracks when the player uses it, and Exarust cannot use
objects yet. F7, in any view, starts every track of the level as a use
would: each start is logged at info level with the node, the mode it
plays in (the first start of a once track plays it in reverse) and the
node's position; `RUST_LOG=info,exa_anim::tracks=debug` also logs where
each ends. `exa-tool corpus tracks` lists the tracks of every pack (one
ships: the secret switch's brick in `exanima03`).

```bash
# The secret switch's brick seen from beside it, at rest, then sunk into
# the wall (slowed four times so the shot lands mid-press; the log says
# when it started and, with debug logging, when it ended 3 s of game
# clock later).
exarust --no-gi --no-supersample --size 960x540 --all-rooms \
  --camera -40.15,1.93,82.6,144,0 --screenshot rest.png --frames 110 level exanima03
exarust --no-gi --no-supersample --size 960x540 --all-rooms \
  --camera -40.15,1.93,82.6,144,0 --sim-speed 0.25 --press F7@20 \
  --screenshot sunk.png --frames 620 level exanima03
```

## Physique

Placed characters take their records' physique; the bodies Exarust makes
without a record (the player, the duellist, the character view) keep the
model's body, which every weight at 0 leaves as stored
([formats/RFC.md](formats/RFC.md), Morph sets). `--physique` morphs the
character view's body, or the player's in play mode, and whatever they
wear. `exa-tool morphs` prints the shipped morph sets (landmarks, how far
each target reaches); `exa-tool morphs --records` the weights every placed
character's record gives them, with each weight's range.
`RUST_LOG=info,exa_assets::morph=debug` logs every morphed model built and
how long it took; play mode logs how long the level's characters' bodies
took.

```bash
# The default body, a tall slight one and a short broad one, dressed.
exarust --no-gi --no-supersample --size 960x540 --screenshot neutral.png character
exarust --no-gi --no-supersample --size 960x540 --physique 0,1,-1 --screenshot thin.png character
exarust --no-gi --no-supersample --size 960x540 --physique 1,1,1 \
  --outfit leathjac_a,trouser_a,boots_b --screenshot heavy.png character
```

## Surfaces and shadows

`--debug-column x,z[/x,z…]` (play or level view) logs, 30 frames after the
load, every mesh whose bounds cover each floor point (metres), lowest first:
its name path, its height range, the heights where the vertical line crosses
its triangles (`^` facing up, `v` down), its shadow-bias level, whether it
casts and into how many cached cubes, and its material's alpha mode and
shadow cut. It finds which surface darkens a floor: the broken-floor slabs
of exanima01's start room showed up 1–2 cm under the terrain.

```bash
exarust --no-gi --debug-column "81.74,37.45/80.34,40.1" play exanima01
```

`exa-tool corpus render-words` tallies every mesh's render flags (the
shadow-bias level and its bits) across the packs, with examples, and lists
the `uw01_floor*` nodes ([formats/RFC.md](formats/RFC.md)).

## Map panel

F9 (Escape also closes it) opens a panel at the right edge of the level and
play views:

- the current view and map;
- switches that apply at once: point shadows, every room (no lids, no
  room hiding), the navigation grid (also F6) and the body overlay (also F3);
- every level of the installation (a `Resource.rpk` chunk tree whose
  top-level chunks hold a tile grid or placed props, 29 in 0.9.5), the
  current one highlighted, scrolled with the wheel.

Choosing a level restarts Exarust on it in the same view. The command line
is kept, less the flags tied to a place, a file or a scripted moment, each
with its value: `--at`, `--camera`, `--corpse`, `--torch`, `--screenshot`,
`--frames`, `--press`, `--click`, `--cursor`, `--debug-column`, `--shove`,
`--shove-props` and the `--pause-…` flags. On Unix the process replaces
itself, so a wrapper such as `context/tools/gpu_run.sh` keeps waiting on
it; elsewhere the new process starts and the old one exits. Settings that
apply only at load (`--no-gi`, `--no-supersample`, `--size`) carry over
through the command line.

While the panel is open the system pointer replaces the world cursor, and
mouse buttons and the wheel stay in the panel: a click on it does not
attack or walk, and the wheel does not zoom.

