# Debugging bodies

Tools that show and measure what the body solver does
([formats/RSQ.md](formats/RSQ.md), Physical body): an overlay of every
simulated body, a readout of one body's joints, a per-joint trace, and
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
# Freeze the duel the moment a link opens 1 cm past its slack, and keep a
# picture of it from the side.
exarust --debug-bodies --opponent --pause-on-stretch 1 \
  --camera 5.5,1.3,6.3,90,-6 --screenshot fold.png --frames 30 play arena
```

## Flags

| flag | effect |
|---|---|
| `--debug-bodies` | start with the overlay and the readout shown |
| `--joint-trace <file.csv>` | write every joint of every simulated body after every 83 Hz tick ([columns](#joint-trace)) |
| `--trace <file.csv>` | the body trace in the retail probe's columns ([formats/RSQ.md](formats/RSQ.md)), now ending with the sleep countdown and state |
| `--sim-speed <f>` | run the simulation at this speed (0.25 is a quarter as fast), from the end of the load |
| `--pause-at-tick <n>` | pause at the end of body tick `n` (the `tick` column of both traces, counted from the start, the load included; see below) |
| `--pause-on-stretch <cm>` | pause at the end of the tick in which a link of a body first goes this far past its slack; the log and the readout name the body, the part and the link |
| `--press key@frame,...` | press keys or mouse buttons at frames counted from the end of the load (`.`, `[`, `]`, `Tab`, `Ctrl`, `F1`–`F12`, a letter, or `LMB`, `RMB`, `MMB`), e.g. `.@400,.@401,P@500`; each is released the frame after; `key@a-b` holds it from frame a and lets it go at frame b, e.g. `RMB@60-400` |
| `--click frame,...` | click the left mouse button at frames counted from the end of the load (the same as `LMB@frame` in `--press`): with `--weapon` in an arena fight, which starts in combat mode, the player attacks unattended |
| `--camera x,y,z,yaw,pitch` | in play mode, a fixed camera there instead of the chase camera (metres, degrees), for unattended shots; with `--fov` |
| `--cursor x,y` | in play mode, the cursor stays at this window point (logical pixels, from the top left) whatever the mouse does: the world cursor, walking to the cursor and combat facing use it ([UI.md](UI.md), World cursor) |
| `--health yellow,red` | in play mode, the player starts with this yellow and red health (a human's full health is 0.25), e.g. `0.176,0.226` for the retail reference shot of the health bar ([UI.md](UI.md), Health bars) |
| `--heartbeat` | in play mode, the low-health heartbeat may play in any level (the original plays it in story mode only, so never in the arenas) |
| `--still-opponent` | `--opponent`, but the duellist stands its ground: it faces the player and takes its stance (by its distance, [PLAYER.md](PLAYER.md), The combat stance) but never walks or strikes; with `--at` it shows the stances at a chosen distance |
| `--debug-wound bone,radius_cm[,dx,dy,dz]` | a test wound on the player's or the character view's body ([Wounds](#wounds)); repeatable |

An unattended attack: `exarust --opponent --sim-speed 0.25 --camera
4.2,1.4,0.6,90,-8 --press LMB@100 play arenao1` throws the unarmed
player's right straight at the duellist, seen from the side (an arena
fight starts in combat mode: Tab would turn it off); the log names each
attack, its moveset and how far left of the facing the cursor was. With
the chase camera and `--cursor` left of the player (e.g. `--cursor
650,280 --press LMB@2`, before the body has turned to the cursor) it is a
left straight; `LMB@200,LMB@206` is a double click, an overhead.

The stances: the log names every change of the player's stance (with its
target's distance and the time since its last attack) and the duellist's
(with its distance and reach); `RUST_LOG=info,exa_combat::stance=debug`
also logs each clip the body tick switches to. `exarust --still-opponent
--at 0,0,6 --camera 8,1.3,2,90,-6 play arenao1` shows both fighters in
their guard 8 m apart, `--at 0,0,1.7` the duellist in its combat idle
while the player still guards (3.7 m), `--at 0,0,1` both in the combat
idle.

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
(`exa_ui::hud=debug`); a heartbeat's start is logged at info level:

```bash
# The health bar as in the retail reference (yellow 0.176, red 0.226), the
# player stood so that the floor lies under the bars.
exarust --at 0,0,1 --health 0.176,0.226 --press Tab@50 --screenshot bars.png --frames 200 play arena
# The bars fading out: combat mode off at frame 300, 1.2 s later at 60
# frames per second (frames, not seconds: a slower run shoots later).
exarust --at 0,0,1 --press Tab@50,Tab@300 --screenshot fade.png --frames 372 play arena
# The heartbeat after the duellist's first blow (about 2 s after the fight
# starts, in combat mode); its border pulses for 7 s, faint at the start of
# every 800 ms, so a shot can need a second try: the log says when it
# started.
exarust --heartbeat --opponent --health 0.06,0.25 --screenshot beat.png --frames 200 play arena
```

With `--pause-at-tick` or `--pause-on-stretch`, `--screenshot` counts its
`--frames` from the pause, so the picture shows the paused body.

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

Drawn over everything, between the last two ticks as the meshes are:

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
- a white dot over the inspected body's head.

## The readout

The top-left panel shows the inspected body: the player's, or the one
picked with F5.

- The body: its `id` (the traces' `body` column), living or limp, its
  sleep state and countdown, support, energy, tone and balance.
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
# The armed player (in combat mode from the fight start) attacks every
# 25 frames; paused after the duellist's second blow, from the player's front.
exarust --opponent --weapon ub05ssworda \
  --click $(seq -s, 100 25 1400) --camera 1.3,1.7,3.4,135,-14 \
  --pause-at-tick 760 --screenshot hits.png --frames 20 play arena1
```
