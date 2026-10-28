# Player control, camera and the carried torch

How the original steers the player and frames the dungeon, and how Exarust
(`exa_game::player`, `exa_game::torch`) does it. Established by REA on
`TPlayerBehaviour`, `TNavCameraControl` and the body tick; the notes, with
addresses, are in the git-ignored `context/re/player-control.md`.

## Movement (exploration)

The original has no camera-relative keyboard movement. Every character
moves through one **locomotion request** its behaviour writes (the
player's keys and mouse, an NPC's AI; in Exarust the player's so far): a heading, the head's look, a flag
word (move, backwards, run, sprint latch, brake, stopping, heading-driven,
turn-about), whether it moves, free walk or a goal point. The body tick
decides from it each tick (below). Outside combat:
- **W** walks the way the character faces (its heading); neither the camera
  nor the cursor changes the heading. The walk is **free**: along the
  facing, with no goal, so it never brakes.
- **A / D** steer the heading by ±0.025 rad per body tick (83 Hz, about
  119°/s) while moving. With W held this walks a circle of about 0.67 m by
  design. Standing still, A or D turns a quarter (90°) once more than a
  second of the game clock has passed since **any** turn key (a quarter
  turn, or A or D steering a walk); A and D together cancel but count. An
  object held by the left button blocks it (The hand, below).
- **Keys as they are.** Exploration reads W, A, S and D as they are held;
  the movement word's 200 ms persistence belongs to the combat footwork
  (Combat mode, below). A quick tap still takes a step because the walk's
  release waits for a double-support key segment.
- **Releasing W** does not stop at once: the request holds until the
  playing walk reaches key segment 0 or 4 (its keys run every 10 frames
  from 10 to 80; segment 0 is the wrap before key 10, segment 4 the one
  after frame 40: the cycle's two double-support moments), then stops
  softly: a goal farther than 8·v + 20 cm in three dimensions (v in cm a
  tick: 33.5 cm at a walk) is brought back along the facing, where it
  brakes to a stop.
- **S** backpedals; faster than 1 cm a tick it first brakes, turning the
  brake into the backpedal once it is slower; released it also finishes
  its stride. It never turns the character about.
- **Right mouse** walks toward the cursor's point on the plane 80 cm above
  the player's ground (the original's aim plane, about hip height): the
  heading is whichever of the smoothed cursor direction and the facing is
  nearer the cursor's, and a reversal of more than 120° turns before
  walking. Beyond 3 m it runs, and the exploration's slow turn is off (the
  body turns at 300°/s while blocked or standing). The point is found
  through the scene camera's own viewport: with supersampling the scene is
  drawn into an image twice the window's size, and window positions are
  scaled to it. The cursor's direction is kept once it is 50 cm or more
  away and smoothed by a second-order yaw follower (each tick the error,
  at most 90°, adds 0.0157 of itself to a turn speed that keeps 0.7: it
  settles in about a second without overshoot).
- **Shift** sets the sprint latch every tick. With the keys the latch is
  spent on the run bit: **W + Shift jogs and never sprints**, the pose
  staying 0.7 of the way to the run. The right mouse beyond 3 m runs and
  keeps the latch, so with Shift it can take the sprint row (the
  original's, REA). The run blend rises 0.05 per tick, at most to 1 − 0.3
  × the character's encumbrance (1 − 0.4 × with the latch; what it wears
  and the shield it holds, [formats/OTHER.md](formats/OTHER.md),
  Encumbrance), bleeding while it runs by √alignment × the stance level
  and always by 0.005 per cm a tick of vertical travel; walking it falls
  0.015 (or follows the speed down while braking), standing 0.02.
  A new character in braies runs at the full blend; in a tourney plate
  harness, helm and leggings (encumbrance 0.625) the blend stops at 0.8125
  and the run at about 3.2 m/s instead of 3.6. With the latch kept the
  pose uses the run blend itself (the sampler reads the latch alone);
  with the latch the walk clip's clock slows by 1 − 0.1 × encumbrance on
  any forward row; once the stance level is also above 0.9 and the step
  is faster than 3.5 cm a tick the gait columns (stop distance, clock)
  take the sprint row (the feral character's own, toward the jog by its
  feral level), and the capsule's run end is the jog speed times the
  feral sprint thrust over the jog thrust.
- **Cursor Look (V)** captures the mouse while held: the pointer and world
  cursor are hidden, horizontal mouse motion turns the facing (the body's
  yaw and the request's heading) by 3.6 × Δx / window width radians per
  pixel (no drag needed), and the camera target turns behind the new
  facing. Releasing restores the pointer where it was. The settings
  screen's binding template shows SPACE, but the shipped table's slot is V.
- **Crouch (X)** sets the crouch amount to 1 while the body is not
  moving (by its last movement decision), or while it is down (condition
  bit 0x200); every tick the amount first falls by 0.02 while above 0, so
  a held key keeps it at 1 and a released one eases out over 50 ticks.
  There is no crouch clip: the body lowers its pelvis toward 60 cm over its
  ground, its chest shifts 10 cm back and its footwork's stance narrows.
  Above a crouch of 0.1 the locomotion clip clock runs × (1 − 0.1·crouch)
  (the float crouch widened to a double, so 0.1 itself already slows);
  above 0.5 the body turns at 0.6 of its rate and the footwork's steps are
  0.7 of the step length; a crouched attacker always picks the first side
  attack.
- **The head looks** 30 % of the way from the heading toward the smoothed
  cursor while walking by the keys, along the heading walking by the
  mouse; standing it eases 5 % a tick toward the heading moved 40 % toward
  the cursor (mixed 40 % with the facing). The torso twists toward that
  look: its yaw within a radian of the body's, followed 0.05 rad a tick
  standing and 0.02 walking, the lumbar taking half, lowered as the body
  crouches ([formats/RSQ.md](formats/RSQ.md), Torso twist). The head
  turns toward the smoothed cursor itself: 0.8 of the cursor's yaw from
  the body's (at most 72° of it, so 57.6°), at most 1.8° a tick, level
  (the cursor has no height), blended in over 100 ticks (Head look).
- **The look-ahead**: 75 cm + 6·v² cm ahead along the facing (70 cm behind
  backwards). As the original does, it walks the navigation grid from the
  standing point toward that goal ([NAVIGATION.md](NAVIGATION.md)); the
  walk stops before the first blocking cell (crates, table legs and rubble
  block cells as walls do). Blocked, the walk is no longer free: it walks
  toward the clamped goal and brakes within its stop distance; within
  100 cm of it the run and the sprint latch are cut too. Without a grid a
  ray from hip height stands in, its goal clamped a capsule radius and a
  30 cm grid cell short of what it meets (Exarust's own; edges lower than
  the hip are missed).
- **The clips.** The body tick's locomotion starts the walk in the tick
  it first moves the character, forwards or back, from whatever clip
  plays: more than 600 ms after it last walked at its start key (2 ahead,
  4 backwards, 5 or 8 turning hard one way, none the other), its clock at
  the key before; within 600 ms where its clock was. The steering also
  plays the walk while the body moves (a braking body keeps its gait) and
  ends it in Idle when the body tick stops it. A walk or a run blends into
  that Idle over 20 frames with the fast ease; a backpedal ends at once. Its
  clock steps by the locomotion's gait clock, × (0.8·speed + 1) and the
  alignment's phase
  envelope ([formats/RSQ.md](formats/RSQ.md), Locomotion); backing (the
  backpedal leg mode) runs it backwards at the backpedal rate, and the
  run is an overlay on the walk. It starts no other clip: what a standing character plays is its
  stance's ([The combat stance](#the-combat-stance)), so a player that
  turned combat mode off in Cmbt - Idle keeps it, also while turning in
  place, until it walks. (REA: the original's exploration steering reaches
  the clip start only through the set-stance helper.)
- **The movement decision** (every body tick, after the posture): the move
  vector is the facing × 200 cm in free walk, else toward the goal; a
  walking or backpedalling body brakes within its gait's stop distance
  (50 cm walking, 200 jogging, 250 sprinting, 50 backpedalling, blended by
  the run blend), a running one never does. A body under its most energy
  (dazed) cannot walk, and one that stumbles walking cannot for 40 ticks.
  A braking body slower than 1 cm a tick stops; any body stops once within
  25 cm of its goal at no more than 0.5 cm a tick
  ([formats/RSQ.md](formats/RSQ.md), Movement decision). The original also
  holds a body under 0.99 balance standing or 0.9 walking; Exarust's
  standing balance can settle under 0.99 with no recovery step to restore
  it yet, so those gates read a steady body until it can.
- **The body turns** toward its heading by the original's table: in free
  walk (1 − 0.1·v)·240°/s (v in cm a tick: 201.6°/s at 1.6), standing or
  walking to a goal in exploration 120°/s (a quarter turn: 0.84 s to 90 %
  of it), attacking by its swing's law (below), otherwise (1 − 0.6·enc)·
  300°/s ÷ √scale; ×0.6 crouched beyond 0.5, slower over the last 0.6 rad
  (×|err|/0.6, so it settles onto the heading), ×0.9 moving by the keys,
  the step ÷ √scale. There is no turn-in-place clip. How well the pelvis
  faces the heading (the alignment, 0.4 to 1 moving, smoothed 0.97 a tick)
  scales the walk's drive.
- **Speeds.** A body with muscles walks by its carry: the drive (thrust,
  phase push, braking, the 0.4·tone drag) is added to the carry with S²,
  the push column and the feet's lag, so walking settles near 1.6 m/s
  where thrust balances the drag, retention and foot friction. Its capsule
  only follows its frame (a tenth of the way a step, at most 2 cm), as the
  original has no capsule. A body without muscles is carried by its
  capsule at Exarust's speeds: walking 1.4 m/s, a full run 3.6 m/s,
  backpedalling 1.22 m/s; braking it keeps the original's brake drive and
  limb retention. NPCs (the duellist) still write their capsule's velocity
  directly through their path driver when they have no muscles.
- Moving firms the body up: its stance factor rises 0.01 per tick from the
  race's 0.5 to 0.7 walking and 1 running (it keeps what it reached until
  the character stops), so its link muscles pull with tone × 0.7 or × 1
  instead of × 0.5 and damp with k = 50, and its pose drive turns the
  pelvis, spine and feet with a gain of 0.0497 or 0.0836 instead of 0.0314.
  In the walk and run cycles the arms hang loosely (stiffness 0.2); while
  the body turns toward its heading they stiffen with the turn left (0.4
  per rad, at most 0.4) ([formats/RSQ.md](formats/RSQ.md), Physical body).
- The body stands on its feet, as the original's does: a lift holds its
  pelvis at a height over the ground under its feet (1.7 cm over the
  target, 101.5 cm for an idle human), its height comes from the clip
  root's over each ankle, and it falls by gravity once its feet have no
  ground. The controller only moves it across the ground and turns it
  (Exarust's own). Walking, the pelvis bobs with the walk's root (about
  2.5 cm). A duellist's hits daze the player without swaying its balance:
  the lift and the legs carry it ([formats/RSQ.md](formats/RSQ.md),
  Physical body).
- The animated pose is put on the body's character frame, as the
  original's is: it rides on the pelvis while the player walks, stays put
  while the player stands steady (support 0.95 or more), and follows the
  pelvis once it is 5 cm off while the player is unsteady, 5 cm while it
  lies, 10 cm as it starts getting up. A steady player is held on its
  stance spot; a player shoved off balance stays where it was pushed. The
  controller follows the frame, a tenth of the way a tick and at most
  2 cm (Exarust's own), so the player's position stays with the body when
  it is shoved, falls or gets up, gliding instead of jumping.
- The feet are planted, as the original's are: standing, each foot grips
  the floor where it landed (within half a centimetre while it stands,
  its lower leg and foot rubbing at 2.0), and a foot whose spot in the
  stance drifts 5 cm off steps there, one foot at a time, as after a walk
  stops; walking, the feet do not grip (until the walk's own plants). A
  standing turn still carries the planted feet round with the body
  (Exarust's own, while the controller turns it). A heavily dazed player
  sinks onto its planted feet rather than toppling
  ([formats/RSQ.md](formats/RSQ.md), Feet).
- Stairs: the lift reads each foot's own ground. The controller's floor
  does not raise the pelvis. A standing leg bends onto a step under its
  foot. A walking foot still does not swing onto the step ahead (the
  gait's plants come later), so a fall in the stair room is that missing
  probe ([formats/RSQ.md](formats/RSQ.md), Physical body).
- Falling: a player off its feet that sinks faster than 5 cm a tick
  screams once; with no ground within reach under it (25 m) it is lost:
  1 s later the pit sounds and the screen fades to black over 3 s, and 4 s
  after the loss the player dies and leaves the world, wherever its body
  is by then. The clock runs only while the body still falls or tumbles
  off its balance, as the original's; slowing down does not save it
  ([formats/RSQ.md](formats/RSQ.md), Falling to death; [UI.md](UI.md),
  Death fade). `--at x,200,z` drops it from 200 m.

## Bindings

The original's default binding table (16-byte slots whose first word is
the virtual-key code; read from `Exanima.exe` and matched slot for slot)
has 23 slots in this order:
left mouse (primary), W, A, S, D, X (crouch), shift (run), alt (alternate
attack), middle mouse (orbit), V (Cursor Look), control (interact), I
(inventory), tab (combat), K (skills), R (switch equipped), F (flip weapon),
J (journal), T (talk), C (power bar), P (power tree), Q (use the picked
power), right arrow (next power), left arrow (previous power): the key
handler (Hotkeys, below) opens the power bar with slot 18, the power tree
with slot 19 and uses the picked power with slot 20. Shift, alt and
control accept either side. The settings screen's template labels Cursor
Look as SPACE; the shipped table's slot holds V.

The original does not save that table. It saves a second one, the saved
table: 23 rows in the Controls page's order (each key cell's action code
is one more than its row), holding keyboard scan codes (bit `0x100` for an
extended key) and, for a mouse button, `0x8000` with its virtual-key code.
The shipped saved table is W, S, A, D, left shift, X, middle mouse
(`0x8004`), V, left control, tab, I, K, J, left alt, R, T, C, P, left arrow
(`0x14B`), right arrow (`0x14D`), Q, 0 and F. After a load the original
converts it into the slots: rows 0–20 and 22 fill slots 1, 3, 2, 4, 6, 5,
8, 9, 10, 12, 11, 13, 16, 7, 14, 17, 18, 19, 22, 21, 20 and 15; row 21
fills none and slot 0 (the primary button) keeps its shipped code. A row
with either of the bits `0xC000` gives its low 14 bits; any other gives
the system's scan-to-key mapping of its low byte (the extended bit is
dropped), which gives the modifiers their left keys' one-sided codes
(`0xA0`, `0xA2`, `0xA4`), so a loaded table reads them on one side
(Exarust's reading; how the original polls a one-sided code was not
traced). The two shipped tables convert into each other row for row.
Exarust keeps a US-layout scan-code table for the mapping (a guess outside
the shipped keys; the keypad's keys map to the navigation keys, as the
system's mapping ignores Num Lock).

The saved payload is little-endian: version word 101, the controller-mode
flag, the count 23, then the 23 rows. The original's loader reads the flag
and then the count when the version is above 100, and otherwise the count
with no flag (off); it copies as many rows as the count gives over the
saved table from its first row, with no bound (Exarust drops rows past 23),
and clears row 21 before converting. Exarust's `Bindings` reads and writes
this shape (its writer turns each slot back into its row's code); the
primary attack, movement, combat, orbit and interact controls read the slot
table, and an unbound key simply does nothing. The Controls page writes
virtual-key codes into the slots.

The table is not a file of its own: it is the payload stored in the
settings store (`<project>.set`) under the tag path `0x6440A401 /
0x6440CB01` ([formats/SETTINGS.md](formats/SETTINGS.md), Settings store),
and the store is written right after it. Exarust's launcher reads it from
there.

## Hotkeys

The key handler compares a pressed key's low byte with the bound keys. A
dead player's keys only end a dialogue. With the power bar open, a digit
picks a power ('0' the tenth, '1' the first), Q uses the picked one and the
arrows step through them; with a dialogue open, '1'–'9' choose an answer
and the Talk key ends it; with neither, the Talk key starts talking, if
allowed. Any other key then goes on, in this order: Q uses the picked power
(if powers are allowed), Tab toggles combat mode, I opens the inventory (if
allowed; in the modes that hide it, only while collapsing), R asks for the
weapon sets to swap (if allowed), F lights or puts out the held light, K
the skills and J the journal (if allowed and not hidden), P the power tree
(if powers are allowed and the player has some), and C the power bar (when
the player, or else the companion, has powers). The permissions are a mask
the game sets: skills and journal 1, inventory 2, powers 4, talk 8 and swap
0x10, all five in play.

Entering combat mode lets go of what the hand holds and clears the
request's backpedal, keys' heading and turn-about bits. The swap is refused
while the body is busy (its condition word has any of 0x10202) or a lift is
on the cursor. F toggles the record's light flag of the first held item
when it gives light and the body is not switching weapons (0x10000), and
then asks the grip runtime to start the held item's flip, as the original
does in the same branch. (The original's command itself reads the weapon
section's flipped-side kind byte, and the grip runtime refuses the flip
for a section whose byte is zero; Exarust keeps the light test for whether
F sends the request.)

Wired in Exarust: Tab, R and F, and C with the digits, Q and the arrows:
the hotkey handler sends the cast plugin the bar's commands, so C opens and
closes the bar, a digit picks, Q uses the picked slot and the arrows step
(the bar's slots are the player's known powers). Not yet: the windows and
the dialogue the other keys open; the extinguished light itself (the
record's flag changes, the torch burns on); the original's cancel of a lift or drag
on entering combat, which puts a lifted item back where its lift saved it
and leaves a drag's link in place (Exarust's lift moves the item on the
item cursor; entering combat lets go of the drag rather than leave a link
no release would end); the flag that keeps a player out of combat mode.

## Gamepad

The original's gamepad is Steam Input: three action sets whose actions the
game reads each frame. Exarust does not link Steam. `exa_game::gamepad`
reproduces the sets, their readers and the player's use of them on Bevy's
gamepad input (`bevy_gilrs`), and maps each action to a default Bevy
control. The manifest's physical bindings live in the game's Steam Input
configuration, not in its code, so the mapping table below is Exarust's
own; it is the one the running game uses. The action names are the
original's manifest names, kept as data.

### Sets and actions

`Set_Menus` is the focused form's set. `Set_Interact` and `Set_Combat` are
the gameplay sets: their actions end in `I` and `C`, and the game switches
between them with its combat mode. `Inventory` and `Skills` are only in the
interact set (read only while it is active); `CrouchC` and `AltAction` are
only in the combat set. Exarust reads whichever variant the player's combat
mode names. The set table registers `Set_Interact` and `Set_Combat` in that
order (1-based indices 1 and 2; registration checks no bound) and starts
on the interact set. A focused form activates the menus, or the set its
push word names (bit `0x8000` with the index in the low byte), directly.
The player controls instead select the gameplay *layer*, which only
becomes active while no form has the focus; popping the last form
reselects that layer and primes its reader, rather than leaving the menus
active (`gamepad::SetRegistry`, `gamepad::FocusStack`). While the
camera stick looks (outside its dead zones), the mouse-look routine does
not start, keep or end the mouse capture: the original's gamepad branch
saves the view's basis and returns before the pointer code
(`gamepad::look_takes_over`), and the chase camera takes the stick's turn.

| action | set | kind | default Bevy control |
|---|---|---|---|
| MenuNav | Menus | analog | left stick |
| MenuScr | Menus | analog | right stick |
| MenuSel | Menus | digital | A (South) |
| MenuBak | Menus | digital | B (East) |
| MenuCur | Menus | analog | right stick (resolved, never read) |
| MenuLMB | Menus | digital | right trigger (resolved, never read) |
| MenuRMB | Menus | digital | left trigger (resolved, never read) |
| MenuExit | Menus | digital | Start |
| MoveI / MoveC | both | analog | left stick |
| CameraI / CameraC | both | analog | right stick |
| AimI / AimC | both | analog | right stick |
| SprintI / SprintC | both | digital | L3 |
| CombatI / CombatC | both | digital | right trigger (RT) |
| WepSwpI / WepSwpC | both | digital | Y (North) |
| PowersI / PowersC | both | digital | D-pad up |
| UsePowerI / UsePowerC | both | digital | X (West) |
| PowerTreeI / PowerTreeC | both | digital | D-pad down |
| TalkI / TalkC | both | digital | D-pad left |
| SelectI / SelectC | both | digital | A (South) |
| LookI / LookC | both | digital | R3 |
| Zoom_InI / Zoom_InC | both | digital | RB |
| Zoom_OutI / Zoom_OutC | both | digital | LB |
| CamModeI / CamModeC | both | digital | Mode |
| pause_menuI / pause_menuC | both | digital | Start |
| Inventory | Interact | digital | Back (Select) |
| Skills | Interact | digital | D-pad right |
| CrouchC | Combat | digital | B (East) |
| AltAction | Combat | digital | LT |

The three menu actions the original resolves but its menu frame never reads
(`MenuCur`, `MenuLMB`, `MenuRMB`) are defined, and the reader leaves them
alone, as the original does.

### The controller-mode switch

The Controls page's controller-mode switch writes `InputMode` (0 for the
keyboard, 51 for a controller). Exarust copies it into the saved bindings'
controller flag, and the player reads the pad only while that flag is set;
the keyboard stays live in either mode. A gamepad connected with no
settings file turns the flag on, and so does a pad used while the saved
mode is the keyboard (a button pressed, a stick or trigger pushed past
half): Exarust's own convenience, since the original's Steam Deck detection
and Steam Input switch are not reproduced.

### The menu set

The reader runs each frame while a GUI form has the focus and the menu set
is active; the focus stack holds up to eight forms and decides the active
set. Activating a set makes the reader wait three milliseconds and prime
the held keys (`MenuSel`, `MenuBak`, `MenuExit`), so a button that was
already down is not a press.

- **Navigation.** The `MenuNav` stick's pair is the cursor `(x, −y)` (the
  interface's `y` is down). Its squared length over 0.25 is a direction:
  vertical unless the horizontal axis is strictly larger (a tie or a NaN
  is vertical), up or right only for a positive raw axis (zero or NaN is
  down or left), emitting `0x1001` left, `0x2001` right, `0x4001` up or
  `0x8001` down. The first step is at once, the next after 400 ms, then
  every 150 ms while the stick stays out; a stick back inside 0.25 resets
  the first delay. In a form whose push word has bit `0x2000` the original
  writes the fast 30 ms delay to the *scroll* stick's deadline and leaves
  the navigation deadline alone: the navigation then steps every frame,
  which Exarust reproduces. This is a quirk of the original, not a
  rounding.
- **Analog bit and cursor.** Independently of the dead zone and the repeat
  clock, a squared length over 0.1 (compared in double precision) adds the
  analog bit `0x10` and scales the cursor by 0.0625 of the milliseconds
  since the previous frame; under it the cursor is the unscaled pair. So a
  pushed stick sends `0x10` with its cursor every frame, a step or not,
  and the form moves analog cursors (sliders, item views) with it.
- **Scroll.** The `MenuScr` stick is read only in a frame without the
  navigation's analog bit (otherwise its cursor keeps its last value). It
  uses the same rules with `0x1002`, `0x2002`, `0x4002` and `0x8002`, the
  analog bit `0x20` and its own 400/150/30 ms clock.
- **Keys.** `MenuSel` emits `0x100` on its first frame, `MenuBak` `0x200`
  and `MenuExit` `0x400`; a key held over from the frame before is not sent
  again. The form is called when the newly pressed keys or the direction
  and analog bits are not zero. A form with an exit hook takes a newly
  pressed `MenuExit` itself and nothing is dispatched that frame
  (`MenuFrame::exit_hook`); the hooks' owners are outside this component,
  so Exarust always uses the normal dispatch.
- **Picking.** A form's key method may answer the press message, and the
  original then picks the control under the stick, in client pixels at the
  GUI unit (4 by default). The cursor is normalised (each part times the
  reciprocal of its length) and its axis chosen as above. A control is a
  candidate when its centre's offset from the current control's centre,
  along that axis, times the direction's part is at least the unit (a NaN
  passes). A vertical pick takes the smallest rounded vertical distance
  (half to even) within 80 units sideways (320 pixels at unit 4), ties by
  the smaller sideways distance. A horizontal pick stretches the offset's
  vertical part eight times, normalises it, needs an alignment (dot
  product with the direction, compared in double precision) of 0.3 or more
  and takes the smallest length over alignment. The current control and
  controls whose records do not allow a pick are skipped; the picked
  control is focused with its 1-based index, plus `0x10000` when the form
  has a control list. `gamepad::pick_control` has these rules; Exarust has
  no drawn form to call it yet. The original also offsets the centres by
  the focused form's parents and walks child forms, which `pick_control`
  leaves to its caller.

### The gameplay sets

- **Move.** The `Move` stick's raw pair is stored, and with neither step
  mode set it is turned into the world by the camera's right and forward
  axes (the vertical component dropped). The squared magnitude over 0.8
  (double precision) is the original's run flag (`gamepad::MOVE_FAST`).
  While the power bar is open the stick's horizontal axis steps instead
  (`0x10` left under −0.1, `0x20` right over 0.1), and while a dialogue is
  open (the power bar checked first) its vertical axis does (`0x40` up,
  `0x80` down), each repeated every 300 ms while held through the command
  word's edge filter; the move is zeroed and the run flag kept. Exarust's
  power bar is opened by the keyboard handler, not the pad, and there is no
  dialogue, so both step modes stay off.
- **Camera.** `Camera`'s vertical axis has a 0.5 dead zone; its horizontal
  axis is zero under 0.1 (double precision, a NaN passes) and otherwise
  `|x|·x`. When the squared pair is over 0.01 the navigation unit's look
  routine turns the camera by the orbit's rate (4.8 rad over the screen
  width, so 0.0025 rad a pixel at 1920) times the camera's sensitivity (1
  for the game's camera) times `(x·30, y·10)`: the yaw loses the `x` part,
  as a rightward mouse drag does, and the asked pitch becomes the held
  pitch plus the `y` part, held between the zoom's floor and 1.5. So up
  tilts the camera steeper, the opposite of an upward mouse drag, and a
  full stick turns 0.075 rad of yaw a frame at 1920 pixels
  (`gamepad::look_turn`). AimSpeed plays no part.
- **Aim.** `Aim`'s pair is turned into the camera's frame (in the
  original's raw-aim mode it is stored as it is, and the player behaviour
  then turns the camera with it by the same look rule). The player
  behaviour takes the aim once its `z·z + x·x` is over 0.01: it puts the
  cursor point `250·|aim| + 50` cm out along it and eases the screen
  cursor there by `(0.4·AimSpeed + 0.1)²` a tick, the one place AimSpeed
  acts. Exarust eases the world point by the same factor each behaviour
  tick, starting from the mouse's point (`player::AimCursor`,
  `player_behaviour::aim_factor`), and turns the combat aim to it; the
  original eases the screen point and picks it on the aim plane, which
  follows a slightly different path to the same point.
- **Zoom.** While `Zoom_In` is held the stepped camera takes ten zoom
  steps a frame, `Zoom_Out` minus ten, applied in that order and each held
  in the zoom range: a step is `(steps × 0.008333334) × stop`, the stop
  being a fifth of the range, subtracted, so zooming in lowers the zoom by
  20 a frame on 800–2000 (`gamepad::zoom_step`). The camera's minimum
  pitch follows the zoom already.
- **Rumble.** With a controller present, a hit adds its amount to both
  motors, clamped at 1. Every
  frame each motor keeps 0.6 of itself (multiplied in double precision) and
  is turned into a 16-bit speed (`×65535`, rounded to even). The original
  resolves `TriggerVibration` but its wrapper returns without calling it,
  so nothing is sent; Exarust computes and keeps the same values and sends
  nothing.
- **Dropped input.** After a gameplay set is activated the reader ignores
  input for three milliseconds, then primes the held keys: `Combat`,
  `WepSwp`, `Talk`, `Powers`, `UsePower`, `PowerTree`, `Select`,
  `pause_menu`, and in the interact set `Inventory` and `Skills`. Nothing
  else is primed.
- **The command word.** The digital actions become the player controls'
  command word, which the game consumes and clears. The bits: `1` the
  combat action, `2` weapon swap, `0x10`/`0x20` the power bar's left/right
  steps, `0x40`/`0x80` the dialogue's up/down steps, `0x100` inventory,
  `0x200` skills, `0x1000` use power, `0x2000` powers, `0x4000` the power
  tree (`PowerTree` only primes it; `Powers` held past 500 ms sets it),
  `0x10000` use/select, `0x20000` talk, `0x100000` Look, `0x200000` camera
  mode, `0x10000000` pause. All are set on their first pressed frame only;
  Look and camera mode are held (re-asserted every frame) and cleared on
  release. Bits 20–23 are the gamepad reader's; its other bits are other
  input's. `gamepad::PlayPad::commands` keeps the word and `consume` clears
  what a consumer took. `AltAction` and `CrouchC` instead add the Alt and
  crouch bits to the key word the keyboard fills, and `Sprint` is a held
  flag of its own.

### What the running game does with it

With controller mode on: the stick walks (its off-axis part turns with the
keys' ±0.025 rad a tick; pulled back beyond 135° it backpedals;
`gamepad::pad_keys`, Exarust's own reading of the original's wanted
direction for its key-driven steering), holds the
sprint latch when sprint is on, crouches on `CrouchC`, and in combat mode
steps along and across the aim. The attack trigger holds the attack request
(so attacks repeat after their cooldown), and `AltAction` asks for a stab;
the trigger's first frame is also latched for a click that falls between
ticks. Guard has no action of its own: in combat mode the stance rule holds
the guard (or Cmbt - Idle) while no attack plays, as with the keyboard.
`WepSwp` swaps the weapon sets. R3 turns combat mode on and off: the original has no
Steam action for that, so Exarust gives the right stick's click the job, as
`Look` (Cursor Look) has no meaning on a pad. The aim stick sets the combat
aim, `Zoom_In`/`Zoom_Out` change the camera's zoom, and `Camera` turns it.

The command bits for inventory, skills, powers, the power tree, use power,
select/use, talk, pause and camera mode reach the word, but Exarust has no
inventory, power, pause or camera-mode feature to consume them yet; the
world cursor's interact shape still reads the keyboard only. `MenuLMB`,
`MenuRMB` and `MenuCur` are unused as in the original.

There are no on-screen button prompts or glyphs in this component: the
original's prompts come from Steam's overlay and its binding panel, and its
text boxes ask Steam for the floating gamepad keyboard. Exarust does not
link Steam, so none of those is reproduced; the menu cursor is the only
gamepad-driven interface element.

## Using what the cursor is over

A click names the object. A free hinge is a door or a lever, and a
blocked one is left alone. A dead character is a corpse and a living one a
person. A loose prop whose mobility is under 0.08 is too heavy to move
unless it is tagged bulky, and one tagged as needing strength is refused
under strength 0.9 (both limits compared in double precision, so a
mobility or strength of exactly 0.08 or 0.9 in single precision is under
them). A usable item on a prop stays usable; with no body it is
nothing. A point farther than 70 cm is turned toward before it is used. A
drag pulls along the cursor direction by the nearest accepted hit, and that
reach starts at 1000 cm. The grabbed box is refused when any placed corner
is higher than 280 cm or lower than 1000 cm.

The click's outcome is [`player_controls::click`]: on a placed object with
no usable section it shows the item; a usable section of kind 2 is a lock:
open, it opens its container; closed, with the key and its reward still
due, its first opening pays 100, or 1000 when the lock's subtype is 3.
Kinds 0 and 1 open the container and every other kind shows the item. A
dead character runs its roles' interact scripts, the player's own body
shows the health and focus bars for 5000 ms (then the click callback runs,
as for a living character). Exarust wires the bars: a click on the own
body sets `exa_ui::hud::HudShowUntil` 5000 ms ahead. The double-click
interaction (in combat mode only with Ctrl held) is
[`player_controls::{interaction_allowed, interact_object, interact_character}`]:
while the object's used mark is clear, kind 0 subtype 1 is used after 2100
ms and kind 1 is taken in hand (which ends the interaction); kind 5 is
pushed while its flag allows; then a container section opens. The player's
inventory opens, a corpse is looted, and a living character talks when its
roles' condition passes or a role links it to the player. The double click
is strict: the same button within the system's double-click time
(`GetDoubleClickTime`, 500 ms by Windows' default, which Exarust uses) and
under 2 pixels from the press before. The original takes it on the second
press; Exarust reads it on that press's release. Exarust wires the player's
own inventory, the item cursor's lift and release (below) and a dead
character's role scripts: a double click runs the scripts
`exa_ai::roles::corpse_scripts` names with the corpse as actor 0 and the
player as actor 1, and their effects go out as
`exa_script::ScriptEffect` (`hand::run_corpse_scripts`). The registry is
`exa_script::Scripts` (`charroles.rdb`/`narrator.rcd`). The other windows
and the living character's talk condition are not in.

### Using an item

An item used on a character acts by its operative section's kind byte.
Kind 8 is a salve: when the body's rule bits take remedies it adds 0.2 to
the heal pool, or 0.1 when the section's value word is 50, in double
precision. Kind 14 is an elixir: when the body takes remedies it sets the
potion flag, stamps the world clock (the save's seconds plus game time)
plus 1800 s, and plays `LqdA_Pour_01` at volume 0.25 with pitch
0.9 + 0.2 times one unit draw. Both kinds OR `0xC80000`, whether or not
the body takes remedies, into the item's flags when it is used directly,
or into its container's inventory entry (the entry slot less one) when
that slot is above zero; a zero slot marks nothing (`exa_combat::remedy`,
`player_controls::mark_used`). The elixir's sound plays only with the
potion. Kind 9 is a cleaning kit that takes an item's dirt
byte down to 0x20, and 10 a repair kit that lowers its wear byte by 255
over the material's rating step `(r + 5) / 10` (at least 1), never under
zero, and marks the item (or its entry) with `0xFF0000`; its sound plays at
the player. A kit's change runs over two seconds: the byte moves 0.0005 of its
gap a millisecond (rounded to the nearest, ties to even), and the timed
list drops an entry once it arrives by moving the last into its place
(`player_controls::{use_on, salve, elixir, clean_timed, repair_timed, Timed,
TimedList}`). A timed value is written only while it lowers a section's byte
or arrives, and the same entry applies to the usable and weapon sections;
when the item has the usable section and its byte is not written, the
weapon section waits for the next frame.
The sounds are `player_controls::use_sound`: the pour at 0.25, the cleaning
kit at 0.3 and the repair kit at 0.15, each pitch 0.9 + 0.2 times one draw.
Wired in Exarust: the right press's use start (a usable kind of zero opens
its container, any other kind begins the use), the target by the pick's
class (a hinge, a living character, a usable item), the GUI use (a cleaning
kit, a repair kit, a kind 0x10 moving the same item between slots) and the
used item's removal from the scene or a container entry
(`player_controls::{use_press, use_target, gui_use, after_use}`), and a
sound played at a point at 0.25 and pitch 1.0 (`point_sound`). Not in
Exarust: the right press itself is the movement request's walk and the
combat attack, so the use cursor has no caller, and the GUI windows the use
writes to do not take items yet.

The start of play makes the AI's activation centre the player with a 40 m
radius; the end of play puts it back at the origin with 200 m
(`player_controls::PlaySession`). While play runs the player controller
moves the centre to the player every tick, and the AI's presence
(`exa_ai::presence`) reads the area: a character out of range leaves the
scene, and one that comes back into range enters it again, reset.

### The reach

A point is within reach when the point the pick hit is at most 245 cm high
in the world (the level's floors are near height 0), and its offset from the
player's hip is at most 300 cm up or down and 300 cm across the ground. A
door is measured from the door's own place with 300 cm of height and 350 cm
across; a wheel from its own place with the plain limits. A comparison with
a NaN passes, as the original's unordered compares do. The reach is measured
each frame: outside combat mode while the right button is not walking to the
cursor, in combat mode only with Ctrl held or something held, whatever the
right button does; a drag in hand is always within reach. The original
keeps its walk latched after the right button's release until the player
stops moving, but the release also sets the request's keys-walking bit,
which measures the reach again, so the reach waits only while the button
walks to the cursor, as in Exarust. Outside combat mode the cursor shows
the pointer within reach and the cross beyond it. The player down,
collapsing or dead lets go of everything and is out of reach. Not in
Exarust: the original's two further tests for everything but a door, that
the point's grid-environment room is the player's or joined to it by an
open portal (6 m cells), and that no flagged mesh of the environment lies
along the pick ray. The region and the walk-to point's own test are
[`pick::reach`] and [`pick::walk_within_reach`]; the runtime hand still
passes the plain point test, since Exarust has no navigation region at a
picked object yet.

### The hand

The hand acts on the left button's **drag start**, not on its press: the
interface sends it once the pointer, the button held, has moved from where
it was pressed by more than 2 pixels on either axis, or within the first
190 ms by more than 2 × ((200 − t)/20) pixels, t the milliseconds since the
press (20 pixels at once, nothing between 181 and 190 ms). A release before
that is a **click**: outside combat mode it turns the player toward the
cursor's point once that is farther than 70 cm (the heading takes the
cursor's direction); the original then names what was clicked (an object's
description card, the narrator's line for a lock it opened, the HUD for the
player's own body, a corpse's roles' scripts), which Exarust cannot show
yet, except the bars for the own body. A double click (a second press
within the 500 ms double-click time and less than 2 pixels
from the one before) opens containers, the player's own inventory, loots
corpses and talks in the original, within reach only; Exarust has no window
for them yet.

A drag start does nothing while combat mode keeps the button for attacks
(unless Ctrl is held or something is already held), while the point under
the cursor is out of reach, or while the player is down. Each one starts
from nothing held. The pick at the pressed point then names what is
there:

- **A prop or item the hand can move**, while the player does not
  locomote: the object's hold begins (the standing quarter turns wait), and
  with **Ctrl** held it is dragged: a grab on the part the pick hit, at the
  hit point, strength 1, in the first two of the tick's 20 passes. Without
  Ctrl it is lifted onto the **item cursor** (`item_cursor`): the part's
  frames saved and the item riding the cursor until the release (below);
  Exarust holds it with a plain grab of strength 1 where the original moves
  the node itself. While the player's kinesis waits for its
  pick, the lift hands the prop to the kinesis instead
  (`docs/THAUMATURGY.md`, Force), and the kinesis's pick callback answers
  the reach in place of the measure: a point the kinesis can take is in
  reach wherever it is, any other out of it (Exarust takes a prop the hand
  can move as the kinesis's pick).
- **A corpse** is dragged by the limb the pick hit, strength 0.5, in every
  pass, with or without Ctrl.
- A door, a wheel or a sim object (cloth) start their own drags in the
  original. Exarust's doors, wheels and cloth are not bodies under the hand,
  so a press only records the drag until the release; meanwhile, as in the
  original, the keys and the right button keep the movement request and the
  idle camera waits.
- **Anything else** (a person, the level, a heavy prop) is refused until the
  release: the cursor shows the cross and the frame's movement request is
  left as it was.

The drag's release does not end the object's hold: in the original only the
lift's release clears it, so after a drag the standing quarter turns wait
until the next lift is released (kept as the original has it).

### The item cursor

A lift without Ctrl holds the item on the cursor. The movement inputs,
which the lift leaves alone, turn it, not the mouse: the pad's raw move
stick when its squared length is over 0.1 (double; a NaN reads the keys),
its forward part about the camera's axis and its sideways part about the
world's up axis; otherwise Forward (else Back) gives ± the speed about the
camera's axis and Left (else Right) ± the speed about the up axis. The
pieces turn the held frame every frame. The speed is `0.25·ms·0.0025` rad a
frame at first (ms: the last frame's time), ramps to `1.0·ms·0.0025` once
the cursor has been turning for more than 300 ms
`(0.25 + (now − ramp)·0.003333…·0.75)` in between, and a frame 500 ms after
the last starts from `0.25` again; the product is taken in double. A piece
under 1.19209e−7 is no turn.

Each frame the cursor's ray is picked against the level, the held item left
out. A hit whose height is not under the limit (−100000 cm) places the item
there; otherwise it goes where the ray crosses that height, unless the ray
climbs (its height part over −1.19209e−7) and the placement is refused. The
item goes there whether or not its box (its rest points' centre and half
extents in its saved frame, turned as above) stays within 280 cm up and
−1000 cm down: the box only decides the routine's answer. The original
then backs the ray's point off 600 cm along the ray and sweeps every
collision sphere of every part of the item against the level's meshes from
there, starting at a 1000 cm reach and keeping a contact only when it is
strictly nearer. Exarust carries each sphere with the grab point the
cursor places, sweeps it at its own radius against rapier's level
colliders from 600 cm before the pick (never from behind the eye), and
stops at the nearer of that contact and the pick; a sweep that starts
inside the level is no contact, and a cursor over nothing keeps the last
point (`item_cursor::{per_sphere_contact, placement_distance,
placement_point}`).

The lift's hover word comes from its frame: an item that snaps onto a
level placement marker gives 0x10, or 0x30 when the marker's link byte
reads `L`; otherwise a free placement that passed its checks gives 1
(`item_cursor::hover_word`). The marker is an entry of a level node whose
eight name characters, read as hexadecimal up to the first other
character, give the item's object id or its model's id; the cursor's
segment (to the pick, or 10 000 cm with none) must meet the entry's
sphere, the item must have an object id and fit the marker's frame, and
the ray must not climb; the first marker in order wins
(`item_cursor::{snap_to_marker, marker_id, segment_meets_sphere}`).
Exarust's level loader does not register the marker nodes and its props
carry no object id yet, so the live hand leaves the word at zero.

A plain release leaves the item where the cursor put it: the release hands
the saved rest points to the deferred restore only to take the body out of
its old sector. A release over a GUI item slot is the GUI's. A lift the
movement request cancels (the point out of reach) goes to the slot it is
over, stays with the world hover bit, and is otherwise put back at the
saved rest points. The turn, placement, hover, release and drop rules are
`item_cursor::{TurnRamp, place_point, place_frame, hover_word, release,
drop, DropTask, spawn_record}`.
An item put down runs the scripts of the scene objects within 5 cm² of the
point (`player_controls::nearby_scripts`), which have no placed nodes in
Exarust yet. The GUI drop takes an inventory entry whose object has a body,
puts it into the scene and lifts it; an entry with no body is the
"Game object has no physics" content error and spawns nothing. Both are
rules with no inventory window to originate them: Exarust's inventory form
has no item panel. A GUI drag (an item or object payload `0xFB01`/`0xFB02`)
is the controls' while no dialogue is open, the movement request is free
and no drop is queued; an item already held makes it busy
(`item_cursor::claim_drag`).

Each frame of a drag the grabbed point is let go once it is farther than
330 cm from the player's hip (the squared distance compared in double
precision), or when the body is a character that lives again. Otherwise,
while the point under the cursor is within reach, the anchor moves to where
the cursor's ray meets the level plane at the grab's height, kept within
200 cm of the hip across the ground (and then at the grab's height), and the
body is woken to 256; out of reach the anchor stays on the grabbed point.
Releasing the button lets go.

A door or a wheel is recorded as a drag until the release: its leaf's pull
point 100 cm from the hinge-side point toward the cursor, and a sliding
leaf's projected onto its axis (`hand::{door_pull_anchor,
door_drag_frame}`); Exarust's doors and wheels are static level geometry,
so nothing moves yet. (The original's door drag also stamps the dragged
body with the player as its last pusher for 2000 ms, which impacts read;
that is not the hold's end.) A scene object's mesh that is a cloth takes
the vertex within 10 cm of the press point in the cloth's own frame (a
stand-in: the original takes the mesh element the pick hit and its own
radius) and sets the drag plane through the press point, its normal the
camera's view axis. Each frame the vertex's target is the pick's point on
the cloth while it stays strictly within the radius of the vertex (the
ray's nearest point to the vertex stands in for the pick), else the
cursor's ray on that plane, both in the cloth's frame
(`hand::cloth_drag_frame`); the cloth's step puts the grabbed vertex on
its target after its links (`exa_physics::cloth::Cloth::grab`, an Exarust
stand-in for the cloth's own grab record).

The grab is a world link of the body solver (`exa_physics::body::Grab`):
in each pass it runs in, after the part's own links, the gap from the point
to its anchor is taken across the ground only; under 1.19209e-7 cm² it does
nothing; otherwise it pulls twice the strength times the gap, the gap
counted at most 3·√mobility cm and never more than 5 cm, less the point's
motion since the tick began that already heads toward the anchor, spread
over the part's particles by the point's weights and taken by the part
alone. A limb's
mobility is its per-bone mass value, a prop's its registered mobility. In
the solver's test a 40 cm crate of mobility 0.5, held by its middle, crosses
1 m in about 30 ticks (0.36 s) and tips onto its side as the floor holds its
bottom back.

The pick looks 40 m along the cursor's ray: at the level and the props'
meshes (their stand-ins), and at the characters' collision spheres (the
original tests every object's meshes, through spheres padded by 5 cm for
interactive objects); the nearest hit wins. A static mesh under a named
scene node comes back as that node (`Hit::SceneNode`), whose stored name a
click hook compares as the original's scene pick does; generated geometry
and every unnamed collider stay `Hit::Level`. The hub's shop counter is a
`_null` box (never drawn, but picked), built as a hidden pick-only collider
that only this pick meets. The
hip is the player's pelvis (the original's reference point the swing whoosh
also plays at).

The point under the cursor is the navigation unit's pick: a screen ray, then the scene's nodes in order, each tested by its class (`0x89`, and `0x4000000` skips it), its layer against the view's, the segment against its bounding sphere and then its box, and each mesh asked for its nearest face within the best distance so far, which then shortens the segment. With no hit the pick records where the ray meets the ground plane. Exarust's `exa_game::nav_pick` does that walk; the physics world answers as the mesh callback because Exarust keeps no CPU triangles. The kinesis variant also needs the node's `0x200000` option bit, looks 40 m, grows each sphere by the caller's height and asks the mesh from where the ray crosses that sphere's top.

## Combat mode

**Tab** toggles it, and every practice or arena fight starts with it on, as
the original turns it on at each fight start (with `--opponent`, once the
level shows; not in the arena hub).

**Footwork** is the original's step machine (`exa_game::footwork`):

- **The movement word.** W, S, A and D form a word along the cursor's
  direction (A is 2, D 8, W 1, S 4; A with D and W with S cancel). A word
  pressed for less than 200 ms is kept until 200 ms after the press, so a
  tap still steps; a word held longer clears on release. A word that stays
  the same strictly over 200 ms while the crouch, widened to double, is
  under 0.1 is flagged a **tap** (the float 0.1 is above that double, so a
  crouch of 0.1 is no tap). The pad's stick vector (camera-relative, in
  the world) takes the keys' place once its squared length, summed x, y,
  then z, is over the double 0.01: it is the **analog word** `0x8000`,
  whose goal and queued step run along that vector (kept as it was read);
  an analog word is a tap only when the stick's squared length is also
  over 0.9. In combat mode only the pad's sprint still reaches the keys.
- **The gate.** No step is laid while the condition word has any of
  `0x82C1` (walking, the shocked bits, down, the kick), the energy is
  under 0.7, a swing is under way, or the word is zero.
- **Starting a step.** With no step under way the goal is laid from the
  word: the chest plus the key sum (summed W, A, D, S; a side key weighs
  0.8 of the facing, its height dropped) scaled by
  `(stride·extension·√scale)/|dir|` cm, where the stride is the step reach
  or, for a tap, `(1.5 − 0.3·encumbrance)` reaches (one double product
  narrowed once: 120 cm unencumbered, 97.2 cm at encumbrance 0.5 on a
  72 cm reach). The line walk from the chest's x/z on the ground clamps
  it; it is taken only when over 400 cm² from the chest in three
  dimensions, and is then moved onto the grid's nearest walkable point
  across the ground. The deep crouch's 0.7 stride is not the goal's: the
  goal routine lays its own stride.
- **Under way.** A step is under way while it has not landed, or its
  target is still more than 20·scale cm from the chest across the ground.
  Within 300 ms of its start, a word not yet a tap **queues** the raw word
  when it differs from the step's own word (the pad's analog word queues
  even when the same), and a tap **re-lays** the step at the long stride
  once. From 300 ms on any held raw word queues. A queued step stores the
  raw word and its full step vector (the side at full weight, scaled by
  `reach·√scale/|dir|`).
- **Landing.** Exarust's interim feet move the capsule toward the target
  at the walking speed and land the step within 20·scale cm of it (or
  after a second, Exarust's own, so a wall does not hold it). A landing
  with a queue takes it by the body's queue rule
  (`exa_physics::body::queued_step`): the target becomes the chest plus
  the queued vector, clamped by the line walk; the next tick the word
  takes the queued word back, its tap and clocks restart. The queue
  clears once the word has been zero for over 200 ms. A step already
  started finishes after the keys are let go, so a held key walks on as a
  chain of steps.
- **The mode word.** Each combat tick the behaviour's ladder sets its mode:
  down or dazed (energy under 0.85; a NaN reads as enough) is idle;
  switching held items or casting is idle; the right button walks to the
  cursor (`player_behaviour::combat_mode`); the left button's press is the
  attack mode. The right button's walk only runs in the walk mode, so a
  dazed player no longer walks to the cursor.

Exarust's footwork plays "Combat Walk" while a step is under way and gives
back the clip it replaced (the guard stays the guard) once it lands.
A walk or run under way when Tab turns combat mode on
stops softly, as the original's combat branch stops it, and ends in the
stance once the body stands. While its condition
word is helpless (down, getting up, collapsing, dead) the player is not
steered, its body does not turn and it engages no attack, as the
original's player behaviour and body tick skip them for mask `0xAC0`.

- **Facing.** The heading turns to the cursor's direction once it is more
  than 30° off it, and follows it exactly while an attack is requested
  (the left button held). The cursor's direction is kept only once its
  point is 50 cm or more from the player across the ground; nearer points
  keep the last aim. The body turns toward the heading by the
  table: standing 300°/s (combat mode has no slow turn), eased over the
  last 0.6 rad; while it attacks, at (1 − 0.6·enc)·(1 − 0.25·progress)·B·W16·P / scale a tick
  (B = 240°/s, W16 the primary striker's handling, P the swing strength,
  the step ÷ √scale): 192°/s at the start of a punch, 168°/s half way,
  slower with a heavier weapon and none once a stall has spent the swing.
  A cursor moved less than 30° leaves the character facing where it
  faced.
- **The attack request**, kept each behaviour tick (83 Hz) as the
  original keeps it: a left press requests an attack, holding the button
  keeps the request standing, releasing it drops the request and
  **releases** the swing in progress (below): a click checks its swing at
  once, a held button swings through. A press asks for:
  - **the main side**: Attack Mid, Down or Up, picked at random and never
    the one picked last;
  - **the other side**, when the cursor lies more than 20° to the left of
    the body's facing at the press (15° for weapon class 0: a body holding
    nothing at all, which a human never is since the fists count as held,
    or a shield or an off-hand light beside the fists): Attack Combo 1, 2
    or 3, the same way;
  - **an overhead** (Overhead Swing) when it comes within 300 ms of the
    press before (that press then no longer counts for the next double
    click), for as long as the button stays down;
  - **a stab** with Alt held, only with a held weapon (never the fists).

  The picks use the original's 32-bit generator (× 0x08088405 + 1, clip =
  state × 3 >> 32), redrawn while equal to the side's last pick; a side's
  last pick starts at 0, so its first attack is never its first clip
  (crouched it would always be Attack Mid; Exarust has no crouch yet).
  Unarmed, the three clips of each side are one clip: the main side is
  the right straight (the right arm out, the body twisted into it, the
  left fist pulled back), the other side the left straight, and the
  overhead a right overhand.
- **When an attack starts.** While the request stands, the original's
  attack executor, which the AI uses too, starts what it asks:
  - only with at least 80 % of the fighter's most energy (1 for a human):
    a blow's daze holds the request until the energy has recovered
    (0.005 a tick);
  - only once the **cooldown** has passed since the last tick of the
    fighter's last attack: 400 ms while it has no primary striker (after
    its hands were emptied, until its first attack), else 300 ms × its
    primary striker's recoil, rounded, + 300 ms: 330 ms for the fists,
    426 ms for the arena sword. Times are on the game clock, 12 ms a tick
    (so 330 ms is the 28th tick), which stops while the simulation is
    paused;
  - while a swing stalls (released or struck) nothing starts, except
    after a **released** swing: a request within 300 ms of the side
    attack's start throws the overhead at once with no blend (whatever
    it asks, before any cooldown, also once the stall is over), and with
    **Feint** (a perk), from (2f + 1) × (round(100·W17) + 100) ms after
    the release (f = smoothstep(progress/0.7), 1 past 0.7: 110–330 ms for
    the fists), the next attack starts despite the stall or the cooldown
    (a stab or an overhead with a blend of 10, a side attack with its
    own);
  - an overhead unless a stab or an overhead plays; a stab unless a stab
    plays, and only with the weapon's long axis forward of −0.367 (its
    forward component in the body's frame); a side attack only when no
    side attack plays.

  So a held button repeats the attack after each cooldown, on the main
  side (every attack clears the side of the press). The cooldown never
  passes while an attack plays (every attack tick restarts it), but a
  double click releases the first attack (its button came up) and its
  second press throws the overhead at once. The duellist's strikes
  (Exarust's own choice, every 2.2 s within 1.15 m, until the original's
  AI) pass the same energy test and cooldown, and it holds every swing
  through.
- **What strikes.** Empty hands are the fists (or claws), each a striker
  built from the race's fist or claw record across its hand's box
  ([formats/OTHER.md](formats/OTHER.md), Weapon record). A striker strikes
  only while its swing's **strike window** is open: from 0.3 of the clip
  (stab 0.4, overhead 0.5) until the clip ends, or for the overhead past
  0.9–0.95 (or at once past 0.85 when it no longer falls 3 cm a tick),
  for the stab past 0.8 (0.95 while it still drives on). With the fists
  only the motion's hand strikes: the right fist in the main side and the
  overhead, the left in the other side (both claws in a clawed overhead).
  A held weapon strikes in every attack and the fists then never do. Each
  attack makes the striking hand its primary striker; equipping a weapon
  makes the weapon the primary at once.
- **How an attack starts.** Its clip starts from where the body is and
  blends to its first key, eased smoothly: the main side with no blend
  past the first key, the other side with round((20 − W19) × 0.6) frames,
  W19 the stance blend of the striker it made primary (12 frames with the
  fists, 8 with the arena sword), the stab and the overhead with the
  executor's blend (10 for Feint's, else none). It ends into the stance
  with twice the primary's stance blend.
- **The swing** (the original's swing routine, every attack tick;
  [formats/RSQ.md](formats/RSQ.md), Motion playback). Its clock runs at
  the weapon's rate, slower at the ends (1.62 frames a tick for the
  fists: a 110-frame punch plays 67 ticks), (E/Emax)² when tired, none
  under 80 % of the energy. A swing strength P scales it while the swing
  stalls:
  - **released** (the button up mid-swing): P drains by max(1 −
    0.05·|v|, 0.1)/W17 × 0.03 a tick (|v| the fist's or blade's tracked
    speed): a still punch in 4 ticks (0.3 a tick), a fast late swing
    slowly (0.01 a tick); the stance returns after 1000/(80 × that) ms,
    at most the swing's expected rest — a clicked punch is back in its
    stance after 5 ticks;
  - **struck** (a blow dazes the attacker by D): r = 4·D·f − (1 − D)·
    min(0.005·s, 1)·W17 a tick, f = 0.25 over its most health; a hard
    blow stops the swing in a tick and the stance returns 80·P/stall ms
    later (47 ms for a forearm cut on a punch), a fast swing shrugs off a
    light daze.
  - **Remise** (a perk): holding a side attack through 0.8 of its clip
    chains one of the other side's, with round(6·W18) + 2 frames of blend
    (2 for the fists): a held punch chains the left straight after 55
    ticks; the chained swing does not chain again.
- Perks come from the character's record (placed characters); the player
  and the duellist are new records, with none (`--perks` gives the player
  some, [DEBUGGING.md](DEBUGGING.md)).
- The swinging arm's stiffness (W20·L(p)) runs in the muscle tune while
  an attack plays: each side's shoulder, upper arm, forearm and hand
  take that arm's stiffness, and the turn floors it. A power sets both
  arms to 0.3 and a weapon switch to 0.4; a kick leaves them, and a daze
  comes before a parry. Once the attack ends, a standing tick eases each
  stored stiffness (0.8x + 0.08·(0.5 + 0.5·race arm relaxation), or
  0.8x + 0.08 while stepping). The two arm drive slots are zeroed at the
  start of every body tick and only an attack writes them, so every other
  tick saturates the arms at 0.1. Not yet:
  swing sounds, the AI's own release and attack choice (L11), recoil and the
  follow-through after a wound (C21, M26), the aim nudges (M23), parries
  (L10).
- **Bodies striking limbs** (`exa_combat::impact`). A body pair whose
  fastest relative travel at its hits passes 1 cm² a tick² measures its
  impact as that times the negated cosine against its summed push; over
  0.5 it strikes each side with the impact over 2·m + 1 of the other
  side's contact mobility. A struck limb of a fighter, not met by a
  striking limb, past its root's sound clock, takes the part's impact
  damage from its volume, the root's damage multiplier, its sinking speed
  and the damage it has already taken from impacts; a hurt takes half the
  amount as yellow health (vital over 0.6 of total) and dazes by half of
  it. The clock then waits 220 + rand(100) − round(170·volume) ms. The
  taken damage falls 0.01 a tick, and the sinking speed follows the
  root's fall at once and its rise by 0.5 % a tick. Each struck part's
  recent-impact counter rises by 2 (under 40) and falls by 1 a tick to 1;
  with the root's over 2 the impact sounds through the character's foley
  (volume times the foley's, pitch 0.5·v + 0.6), else as the bone's hit
  sound (pitch 0.6·v + 0.5), at the character. The impact's voice and the
  level's own impacts are not wired.

### The behaviour's aim and contact rules

The player's behaviour keeps these rules (`player_behaviour`); the gamepad
aim's point and ease, the combat auto-target and the aim point's bone lock
are wired, the rest are tested functions with no caller yet:

- **Aim point.** The cursor's point on the plane 80 cm over the player's
  ground; with a target, the nearest of its bones on screen (each bone's
  fraction `t = (b − a)·(c − a)/|b − a|²` from the target's projected
  reference point `a`, clamped into [0, 1] by the ordered tests, so a NaN
  stays NaN; the first bone taken outright, a later one only when strictly
  nearer) sets a lock on the reference point moved toward that bone by
  `t`: within the pixel radius 80 cm make at the reference point's depth
  (`width·scale·0.5·80 / depth`) the aim height is that point's, within
  twice it the two blend by `(d − r)/r`. Without a target
  the point the cursor's pick meets replaces it when it is above the
  plane's point and under the ground plus 200 cm
  (`aim_lock_bones`, `bone_screen`, `bone_lock_radius`, `aim_raised`).
  `player::cursor_ground` runs it each frame: the auto-target's opponent
  gives the reference point (its chest) and three of its part origins —
  the head, the belly and the pelvis, a stand-in for the original's own
  three aim points — and the pixel radius uses the window's width and the
  projection's x scale. The raised point comes from the level's fixed
  colliders along the cursor's ray (the original's navigation pick also
  admits marked non-solid nodes).
- **Contacts.** Every character within 12 m, rebuilt each tick: closeness
  `1 − d·0.00083333335` capped at 0.99, facing weight `0.75·(0.5 −
  0.5·facing·dir) + 0.25·(0.5 − 0.5·cursor·dir)²`, score half of each;
  twelve kept, a new one replacing the lowest it outscores.
- **Engaged enemy.** The largest threat among the living characters within
  the player's reach plus 70 cm, in front of the heading by more than 0.3
  of their distance (in double precision), whose own contact score for the
  player is over 2.
- **Attack timing.** Computed on an attack's first tick: the wind-up step
  `(1 − 0.05·√E)` (1 at no energy), floored at 0.1, over the primary
  weapon's recoil (W17), times 0.03,
  floored at 0.01; the expected end `1000/(80·step)` ms on, at most the
  swing's length; the recovery `(2f + 1)·(round(100·recoil) + 100)` ms, f
  being `(3 − 2b)·b²` for `b = balance/0.7` while the attack motion (5 to
  12) is on and the balance is at most 0.7, else 1. The running game
  records it when an attack starts and holds the next attack until the
  recovery clock (unless the swing has been released, which keeps its own
  stall and renewal). The fists' 0.1 recoil at
  full energy give a 0.285 step, a 44 ms expected end and 330 ms of
  recovery at full balance.
- **Movement goal.** A word's direction is the facing (W), the side
  `(facing.z, 0, −facing.x)` at 0.8 (A, D) and back (S), summed and
  flattened; the step vector sums the same keys with the full side vector
  (no 0.8 share), flattens it (the pad's own stored direction instead) and
  scales it by `step·factor / |dir|`; a tapped word lays the goal at
  `(1.5 − 0.3·encumbrance)` of the reach (in double precision), then the
  legs' extension and the clock factor; and a goal more than 400 cm² from
  the body lets go of the movement target. Exarust's exploration request walks by the body's
  look-ahead and does not build this goal, so the direction, the step
  vector and the tap stride are tested rules with no caller
  (`steering::key_goal_direction`, `key_step`, `goal_stride`); the 400 cm²
  test is the footwork's step test (`goal_released`).
- **Auto-target.** The visible opponent within 500 cm whose
  `(500 − d)·(δ·cursor/d + 0.5)` is the largest, strictly over the best
  from zero (so a zero, negative or NaN score never wins), the first on a
  tie; the target is dropped when it dies, and the stance picks 3
  (Defence) or 4 (Cmbt - Idle) from it (350 cm × the target's scale, or
  3 s since the player's attack, and the energy within 0.05 of the most)
  (`auto_target`; the stance rule is `exa_combat::stance::player_stance`).
- **Look.** Strictly over 30° (the single 30 times the single degree)
  between the look and the cursor's direction, the gap wrapped into ±π in
  double precision, the look snaps to the cursor's direction
  (`look_snaps`). When the camera follows the body, a camera yaw more than
  0.1π from the body's is put back to the body's yaw 0.1π on the camera's
  side, wrapped into ±π; within it nothing changes (`camera_follow_yaw`). The cursor
  follower itself is the same second-order one as the steering's
  (`steering::CursorFollower`: error clamped to ±π/2, speed
  `(speed + 0.015708·error)·0.7`, a speed under 1e-5 still).
- **On screen.** The living character whose point shows nearest the cursor.

## The combat stance

Every fighter keeps a **stance**: the looping clip it returns to. Every
clip it plays once (an attack, a get-up) ends in the stance it holds when
the clip ends, and a balanced fighter (balance 0.8 or more, muscle tone
0.85 or more) standing in another looping clip switches to it. The
original's body tick leaves three clips alone: **Cmbt - Idle**, the walk
and the run each have their own branch, which skips the switch (Exarust
also leaves its interim backpedal and combat walk alone).

| who | stance |
|---|---|
| the player out of combat mode | Idle (the clip follows at once when it was the old stance's) |
| the player in combat mode, dying (yellow health under 0.01, compared in double precision; alive there only after non-vital damage, [formats/OTHER.md](formats/OTHER.md), Applying damage) | Defence stance |
| the player in combat mode, not dazed, its target nearer than 3.5 m × the target's body scale (chest to chest), or within 3 s of its last attack's last tick | Cmbt - Idle |
| the player in combat mode otherwise | Defence stance |
| a duellist entering combat (at the fight start; a level character once its rating of the player engages, from the roles both hold) | Defence stance |
| a duellist in combat | Defence stance at 4 reaches or more from its target, Cmbt - Idle within |
| any fighter, dazed (its energy more than 0.05 under its most, compared in double precision), every body tick | Defence stance |

- **The player's target** is picked every tick among the other living
  fighters within 5 m: the best (5 − d)·(cos + 0.5), the angle measured
  from the cursor's direction, so never one more than 120° off it.
- **The guard.** Unarmed, the Defence stance of the "Default" moveset is
  the fists-up guard: hunched, the fists before the chest and chin, knees
  bent (reference shots 1 and 2). Armed with "One Handed" it holds the
  sword hand at mid-chest; Cmbt - Idle holds the hands, or the weapon, low
  before the hips (reference shot 6; the blade's angle is the one-handed
  grip's, [Held items and their grips](#held-items-and-their-grips)).
- **The original's quirk.** The switch never runs while Cmbt - Idle plays,
  so a player who dropped to Cmbt - Idle stays there when the target walks
  away or dies, or 3 s after its attack: its stance turns back to the
  Defence stance but its clip does not. The guard returns through the end
  of a clip played once (an attack, a get-up), a fall, or a new fight. (An
  attack ends within 3 s of its own last tick, so it ends in Cmbt - Idle
  unless the player is dying.) The live retail trace showed exactly this:
  Cmbt - Idle with the Defence stance for seconds, also after the opponent
  died, and the user's own game for over half an hour at the exanima01
  start, the torch carried before the chest.
- **Leaving combat mode.** Tab off writes the stance Idle through the
  set-stance helper, which starts Idle only when the playing clip is the
  old stance's: from the guard, or from Cmbt - Idle within 3 s of an attack
  or with a target near, the player idles at once. From Cmbt - Idle with
  the Defence stance (the quirk above) nothing starts it: the stance turns
  to Idle, the clip stays Cmbt - Idle until the player walks, and the walk
  ends in Idle.
- **A duellist** starts its stance whenever it is free (not attacking), so
  it guards again when its target leaves. Its **reach** is 60 cm plus its
  weapon's sweet spot ([formats/OTHER.md](formats/OTHER.md), Weapon
  record), 50 cm unarmed: the arena sword's 98 cm makes it guard beyond
  3.92 m.
- **The daze.** While a fighter's energy is more than 0.05 under its most
  its body tick makes its stance the Defence stance, and, unless it is
  down or attacking (its condition word helpless or attacking,
  [formats/RSQ.md](formats/RSQ.md), Condition), starts the Defence stance
  when another clip plays (so a dazed player still falling restarts its
  fall every tick until it is down, as the original does),
  blending round((1.05 − E) × 300) frames (129 at the 0.62 a duellist's
  blow leaves), eased smoothly. Its body meanwhile holds the pose it is
  physically in (standing on its feet, in full), so the guard does not
  rise; when its energy is back the player's stance turns to Cmbt - Idle
  again (with its target near) and the posture switch blends back.
- **The blends.** Every stance change starts the stance's clip from
  where the body is (the root and the legs' last targets, the upper
  body's physical turns) and blends it to the clip's first key over
  (first key + blend) frames: the set-stance helper (Tab, a duellist
  entering combat) 30 frames, smooth; the posture switch 30, linear; a
  duellist's stance 40 frames without a primary striker, else twice its
  primary's stance blend (14 with the arena sword, none after a punch),
  smooth, and on every pass it keeps the blend's ease smooth; the end of
  a clip played once the original's table (twice the primary's stance
  blend after an attack, none after a get-up), smooth
  ([formats/RSQ.md](formats/RSQ.md), Motion playback).
- **Readiness.** With the stance in Cmbt - Idle or while parrying, a
  fighter's readiness rises 0.004 a tick (the add in double, 0 to 1 in
  3 s) while the clip is Cmbt - Idle, it parries, or a released attack
  passes its ready bit. Anything else resets it.
  A blow's wound (penetration, impact, stab energy) is scaled by 1 − 0.5 ×
  readiness, so a fighter ready for 3 s takes half the wound; the shove is
  not lessened ([formats/OTHER.md](formats/OTHER.md), Damage).
- **Healing.** Yellow health regenerates only while Cmbt - Idle does not
  play and the fighter does not attack: the regeneration rate ramps up
  over 40 s and every blow sets it back to 0, so a fighter that stays in
  its fighting stance (the quirk above included) never heals, and the
  player heals once it leaves combat mode and its clip is no longer Cmbt -
  Idle, or stands in the guard; in the daze's Defence stance too
  ([formats/OTHER.md](formats/OTHER.md), Regeneration).

Not yet, with the change that brings each:
- the walk's end in the stance (Exarust's interim footwork and gaits
  start their clips with no blend; the original's walk start is in);
- the Defence stance and Rebalance a fighter takes from its balance
  (below 0.8) or its tone (below 0.85), C11;
- parries, whose clips are stances while they last, and readiness while
  parrying (L10, C21);
- the original's line-of-sight test on the player's target; the AI's own
  footwork in its stance instead of the combat walk, and its parries (C16,
  L11). The live retail opponent took its guard for 0.4 s at the fight
  start before Cmbt - Idle; Exarust's duellist holds it one frame when it
  starts within four reaches (that part of the original's AI is not
  traced).

A combatant's head tracking (`exa_combat::aim`) is the original's per-tick
head-tracking routine, with a far variant (a 150 cm look-ahead and no pitch
floor). Its yaw is the engine's planar angle (the arc cosine of z over the
horizontal length, the routine the free camera's yaw uses) and its pitch
the negated two-branch arc sine of the height.

## Moveset from held items

What a character holds picks the moveset its clips come from, after every
equip change and in or out of combat mode, as the original picks it. The
choice is a config word: the weapon moveset to look up under the race's
own moveset (`Default` for a human), plus the "Torch" overlay. The
character's clip table is the race's moveset chain with the weapon
moveset and the overlay laid over it ([formats/RSQ.md](formats/RSQ.md),
A character's clip table): a moveset lacking a clip takes its parent's
("One Handed" from "Default"), and an ogre or a golem with a sword plays
its own "One Handed" over the human one. The same choice gives a stance
class (what the other arm does while one swings) and a weapon class (the
held weapon's hands bits, `0x80` for the fists).

| held | config: moveset | stance class | weapon class |
|---|---|---|---|
| empty hands (the fists) | `0`: the race's own alone | 5 | `0x80` (`0xC0` for a clawed race) |
| one one-handed weapon | `0x1` "One Handed" | 1 | `0x10` |
| two one-handed weapons | `0x4` "2W Light" | 6 | `0x90` |
| a two-hander with flags `0x521` | `0x30` "Greatsword" | 2 | `0x20` |
| another two-hander, kind bit 0 set (bladed, stick) | `0x20` "2H Light" | 2 | `0x20` |
| another two-hander | `0x10` "Two Handed" | 2 | `0x20` |
| a polearm | `0x40` "Polearm" | 2 | `0x30` |
| a shield (beside anything) | `0x100` "1H & Shield", or `0x200` "Shield C1" / `0x300` "Shield C2" by two character flags; plus `0x10010000` (the "Torch" overlay) when the main item is a light | 3 | the main item's |
| a light in the off hand | `0x3` "1H & Torch" | 4 | the main item's (`0` beside the fists) |
| a light in the main hand | `0x10010001`: "One Handed" with the "Torch" overlay | 5 | the light's (`0x10` for the torch) |

The rows are tried top to bottom from the shield down; the weapon flags
are the low 16 bits of the item's weapon section's first word
([formats/OTHER.md](formats/OTHER.md), Weapon stats). The torch item is a
one-handed stick (flags `0x13`). A human holding the torch in the main
hand plays the same clips as Exarust's "Torch" moveset on its own
(`exarust character Torch/Idle`).

In Exarust the player and the duellist follow this rule: the player
unarmed fights with the "Default" guard and punches, armed with "One
Handed" (also idling with it out of combat), and with a torch as below.
Their animators keep the race's moveset as the base and take the config.
The character view plays the moveset its command line names, and level
characters keep their race's moveset with no config until they have
equipment. `exa-tool clips <base> [config]` prints any table. Not yet:
held items on creatures, and the flaming weapon (its config adds
`0x10020000`, whose shared bit also lays the Torch overlay on a one-handed
weapon). A shield or a second weapon in the off hand is attached as its
own body (`exa_character::offhand`).

The cursor is the game's own world cursor ([UI.md](UI.md), World cursor):
a red ring in combat mode, and outside it an amber arrowhead over ground
within reach, a cross beyond it and chevrons while walking to it. In
combat mode the right button walks to the cursor too, as the original's
combat branch does while the button is held: the request keeps only its
sprint latch, the heading and the look take the cursor's direction, a
cursor beyond 3 m runs (the Run key's latch also runs nearer), and the
look-ahead `6·v² + 45` cm along the facing stops the free walk and, within
100 cm, the run; the footwork yields while it walks and the cursor shows
the chevrons however near the point is (`steering::Steering::combat_walk`).
The release ends it at once.

## Held items and their grips

Every held item sits under its hand as the original's grip table places
it, at the equip (by the item's own weapon class) and again whenever the
held items change (by the character's), both giving the same grip
(`exa_combat::grip`):

- **Hand class.** The second held item (a light or a second weapon in the
  off hand) takes the off-hand grip, 5. Otherwise the weapon class (the
  low 16 bits of the weapon flags): two weapons (its high nibble 9) the
  one-handed grip, 1, so the first of two weapons takes the one-handed
  grip and the second the off-hand grip; else its bits 4–6: 0 fists, 1
  one-handed, 2 two-handed, 3 polearm, 4 claws. Of the 208 weapon items
  of the shipped object database, 136 take the one-handed grip (the torch
  among them), 40 the two-handed and 32 the polearm's.
- **The grips** (offsets in cm in the hand's space, Euler angles in
  degrees about the hand's X, Y and Z):

  | grip | hand | offset | the ogre's offset | angles | links to the hand (point, slack) |
  |---|---|---|---|---|---|
  | 0 fists, 4 claws | — | — | — | — | none: the hands strike |
  | 1 one-handed, the first of two weapons | right | (−12, −1.5, 6) | (−27.5, −7, 9) | (70, −15, 20) | (0, 20, 0) 0; (15, 0, 0) 0 |
  | 2 two-handed | right | (−13, −2, 6) | (−30, −2, 14) | (70, −15, 20) | (15, 0, 0) 5; (0, −20, 0) 0 |
  | 3 polearm | **left** | (11, −2, 2) | the same | (110, 15, −170) | (0, 0, 15) 0; (0, 20, 0) 0 |
  | 5 off-hand item, the second of two weapons | left | (11, −0.5, 6) | (30, −4, 14) | (110, 15, 160) | (0, −20, 0) 0; (15, 0, 0) 0 |

  The two-hander also ties the item's (0, −30, 0) to the left hand's
  (11, −3, 0) and its (0, 20, 0) to the right arm's (−12, 3, 6), and a
  stab its (0, 25, 0) to the left hand's (8, −5, 0); the polearm ties its
  (0, −40, 0) to the right hand's (−10, −2, 0) and its (0, 20, 0) to the
  left arm's (11, 3, 6). The points along the haft (−30, −40) scale with
  the weapon's second-hand factor. Weapon models hold their grip at the
  origin with the blade along +Y, so the second hand holds the haft below
  the first.
- **Placement.** The offset times the character's size (its race's size
  × (1 − 0.05 × height)); an ogre (race byte 3) takes the grip's own ogre
  offset, not scaled. Then the race's held offset (`races.rdb`, three
  signed bytes × 0.1 cm; 9 cm along x for the demon alone): its x taken
  off in the right hand and added in the left, its y and z added. The
  turn: the item about the hand's X first, then Z, then Y (the rotation
  builder puts the item's X axis on (cos z·cos y, sin z, −cos z·sin y));
  a flipped item has its X and Z axes negated, half a turn about its
  blade. The one-handed grip points the blade along (−0.36, 0.32, 0.88)
  of the hand: mostly along the axis the fist closes around, tilted up
  and toward the thumb.
- **What shows.** The one-handed sword in "One Handed" Cmbt - Idle is held
  low in the right hand, pointing forward about 25° up and across the
  front of the hips; the two-hander in "Greatsword" Cmbt - Idle stands
  upright, tip up, before the chest, as in the live retail capture of the
  player's two-hander; a polearm is held in the left hand at the chest,
  its haft running down to the right hand at the hip.
- **In Exarust.** Every rigid node of a held item is a solver part joined
  to its hand, the top one moving the item's root. An assembled factory
  weapon's fitted part is built by a tested function but not attached yet:
  those weapons stay scene children of the hand. The grip modes switch the
  two-hander's second hand, arm and stab links and slide the polearm's
  grip by the motion, and a flip request (sent by F) runs the flip
  animation and rebuilds the weapon record from its flipped side; a
  weapon whose section has no flipped-side kind byte is refused, and a
  dead holder's soft weld stiffens while a collapse drops the weapon and
  keeps the shield. A held
  shield item takes the shield builder's face pivots, and the off hand's
  second weapon takes the grip table's off-hand entry; both are attached
  by `exa_character::offhand` after the main hand's weapon, and the
  record's flipped hand flag starts the main weapon turned over
  (`HeldFlipped`).

## Weapon sets and what a character carries

`exa_items::possession` keeps a character's possessions as its record
does (`Possessions`, on every placed character from its record and, empty,
on the player): two weapon sets of a main and an off hand, each hand an
item id and a flags word (`0x0100_0000` flipped, `0x0200_0000` the light
burns), the worn items as the record's id-and-flags pairs, and the carried
list, in which an item in a hand stays listed with its held bit `0x100`
set. A dead body's loot window shows the same list (the pack at its saved
places) and the worn items plus the shields in the hands on its equipment
panel; a drag between that window and the player's inventory moves the
possession entries (`exa_game::body_inventory`, `exa_game::panel_live`). Established by REA on the
character body's possession routines; addresses are in the research notes.

- **Swapping sets** (R, Switch Equipped). Refused while the body is busy or
  a drag is in progress; with the pack open the sets swap at once,
  otherwise the body enters its swap state and its animation swaps them.
  The swap toggles the record's second-set flag `0x0100_0000`, then for
  each hand of the set going away unmarks as many held entries of its item
  as hands it fills (a two-handed item, the same id in both hands, counts
  twice and is handled once from the main hand), less the hands of the
  incoming set that hold it too; the incoming set's items are marked held
  the same way. The two sets trade places only when a hand's item changes
  (the original exchanges them in the step that re-dresses the hands and
  skips that step otherwise, so sets holding the same items keep their own
  flags). The hands then take what changed: a weapon as a weapon
  (in the off hand a weapon that is a light as a light), a shield as a
  shield, any other item with a held-light section as a light, anything
  else as a weapon; only a weapon takes the stored lit and flipped bits.
  Exarust carries the request out at once (no swap animation, no pack
  window, no drag) and leaves the plan on the body (`PendingHands`);
  nothing re-attaches held models at run time yet.
- **Taking an item into the hands.** A one-handed item goes to the main
  hand when it is free. With the main hand full: a light beside one
  weapon or the fists (weapon class `0x10` or `0x80`) goes to the off hand
  as a light, freeing it first; a light already in the main hand moves to
  the off hand when that is free, else it is taken out; a second weapon
  goes to the off hand only when asked for and never beside a two-hander
  (class bit `0x20`), which is taken out instead; otherwise the main
  hand's item is taken out. A two-handed or polearm item (grip `0x20` or
  `0x30`) frees the off hand, then the main hand. The value of what would
  be displaced follows the same branches, by desirability.
- **Taking an item out.** The record's entry for that hand (else the other
  hand's) gives its flags and is cleared; a dropped first weapon hands the
  main hand to an off-hand weapon or light. The item goes back to the pack:
  when no pack is open it is listed at (40, 4) sixteenths of a unit; when
  it was already listed as held, the mark is cleared instead.
- **A held item coming loose** clears each out hand that is not a shield
  and its first held entry; a stowed hand whose item is no longer listed
  is cleared (a two-handed stowed item keeps its main hand while one is
  still listed).
- **Giving** appends `{id, flags, 0, 0}` to the list when no pack is open.
  **Having an item** searches the list and every container in it, at any
  depth, through the containers' children (`exa_items::catalogue`).
- **Golem cores.** A race flagged `0x2000` takes only the AL power
  crystal, one flagged `0x1000` only crystal A, and only while it wears
  none of the five crystals.
- **The AI's planned arms.** Planning a one-handed weapon keeps a planned
  light beside it; a two-handed one drops what was beside it.

## Camera

- The camera follows the player's **pelvis** only, at a fixed **20 m**: the
  pelvis as it is drawn in the frame, between its last two body ticks, so
  the view rises and falls with the walking body (the feet before the
  player has a body). It never turns by itself in exploration, except
  when the player is left idle (below).
- The camera **ticks every 17 ms** of real time: a tick runs once the frame
  passes its start, at most nine a frame (a longer stall drops the rest).
  Each frame shows the direction to the eye, the zoom and any shake offset
  between the last two ticks, by how far the frame is into the last one, so
  the view moves smoothly on any refresh rate. A shake offset moves the eye
  only, and only while it is over 1e-5 cm² squared. A **kick** moves the
  offset one way and the last tick's the other; each tick then keeps 0.96
  of its change and 0.99 of the whole, a wobble of about a second that
  dies away, and drops it once both are tiny (change within 1e-7 cm²,
  offset within 1e-4 cm², squared). The kick's offset is the kick's own
  direction, normalised (or, when it is shorter than √0.1, the camera's
  right axis by the sign of a random draw, lifted by up to ±2), plus a
  small random vector (±0.5 across, 0–0.15 up), scaled to the kick's
  size. Exarust builds it in `nav_camera::kick_offset`; nothing fires it
  yet, since the directions and sizes its impact callers pass are not
  traced (`ChaseCamera::kick_random`).
- **Zoom is field of view**: fov = zoom / 2000 × 45° × √(1.6 / aspect),
  clamped to 10–100°. The wheel steps through six stops from 800 to 2000
  (18° to 45°); the game starts at 1300. The unit's wheel handler reads the
  present zoom as a stop level (`(zoom − min) / (range/5)`, rounded to the
  nearest, ties to even), subtracts the wheel in 120-unit notches of
  0.008333334 levels, holds the level between 0 and 5 (the processor's
  max-then-min, so a NaN holds at the least) and writes `level · range/5 +
  min`; one notch of Bevy's scroll line is 120 units, and a touchpad's
  pixels count 24 to the notch (Exarust's guess) (`exa_game::nav_drag::zoom_detents`).
- **The zoom's pitch offset.** When the preset gives the controller an
  amplitude (a controller field set outside this unit), each zoom change
  computes `t = 1 − (zoom − min)/(max − min)` and sets the pitch offset to
  `exp(3·ln t) · amplitude`, the engine's fast logarithm and exponential,
  and lowers the pitch to `−offset` when that is steeper
  (`nav_drag::zoom_pitch_offset`, `pitch_with_offset`). The chase camera's
  preset leaves the amplitude zero, so the live camera is unchanged.
- **The absolute drag** (the free and editor cameras): a press snapshots
  the yaw and the pitch `−asin(back.y)`, and builds the pointer's vertical
  range from the two pitch bounds, `y + round((bound + pitch)/(rate ·
  sensitivity))`; a move sets `yaw = base − rate · sensitivity · dx` and
  the pitch likewise, its negation held between the bounds; releasing after
  more than an eighth of a turn snaps the yaw a quarter turn from the shown
  yaw in the direction moved (`nav_drag::FreeDrag`). The vertical zoom
  slide is `clamp(dy · 0.001 + base, 0, 1)² · range + min`
  (`nav_drag::zoom_slide`). Modelled with tests; the chase camera uses its
  own spring and the mouse's relative orbit.
- **The editor's grab drag**: a press on a scene node holds the node's
  place at the looked-at point's height, the camera `radius + zoom` from
  it and the yaw of the target less the grabbed point; a move turns the
  yaw by `dx · rate · 0.5` and raises the point by `dy · zoom /
  (width · aspect / 2)`, the looked-at point following the drag at the new
  yaw. A press on a node's face turns the view onto the face normal (the
  engine's arc cosine and a rotation about the cross product) and puts the
  target that many zoom units along it (`nav_drag::{grab_start, GrabDrag,
  grab_target}`). Modelled; Exarust has no scene editor to drive it.
- **Pitch**: 0.8 rad below the horizon under zoom 1000, then 0.000125 rad
  steeper per unit up to 0.925 rad at 2000 (the zoom less 1000 in single
  precision, the product and the sum with 0.8 in double). A vertical
  orbit drag can raise it to 1.5 rad. The camera holds the asked pitch or
  the floor when that is steeper, and the pitch moves 10 % of the way per
  17 ms camera tick (in double precision, only over a gap above 2e-6).
- **Cursor Look keeps the target yaw within half a turn of the camera's**:
  the align routine compares the target with the camera's present yaw in
  double precision and adds or subtracts a whole turn, so the spring
  turns the short way. The unit also places a camera from a looked-at
  point and an eye: the point's height is clamped to the preset's limits,
  the backward vector runs to the eye from the unclamped point and is
  scaled by its reciprocal length, the yaw is the engine's planar angle
  (its polynomial arc cosine of z over the horizontal length) and the
  pitch its arc sine. The stepped camera (the game's) starts its zoom
  between the preset's 10 and 10000 and its pitch between the held pitch
  and 1.5 rad. A free zoom moves 0.2 % of its value per wheel unit (120
  to a notch) but at least 0.5 % of its range. The preset record's
  numeric fields are applied as the original applies them (the 280 and
  16000 defaults over 59², the 59th roots of the retention, a double
  3.94e-13 by default whose root is the orbit's 0.616, and of the rate,
  the −1.5 and 1.5 pitch bounds, the 10 and 10000 distance range and its
  clamp of the distance field to that range, and the speeds: 5 and 5/59
  on the stepped camera, 15 × the largest distance / 59² and 6.6/59
  otherwise). Under the editor flag it scans the scene for a `0xA9` node
  of class word 1 and takes its place and its half extents, a `0xAE`
  marker raising its extra flag (`nav_camera::scan_target`). The callback
  set its flags ask for (the wheel, the editor's press and move, the free
  camera's press and move) is `nav_camera::preset_handlers`; installing
  them on a viewport has no Exarust counterpart. The
  chase camera uses the shared pieces; the placement, free-zoom, preset
  and drag paths are modelled (`exa_game::nav_camera`, `nav_drag`) with
  tests but have no Exarust caller yet except the wheel.
- **The ground pick** (the unit's point-to-ground routine, used by the
  player-control ground and reach probes): from the screen ray, the
  direction's reciprocal height is held to zero when it points up, the
  reach is 40 m, and each pickable node grows its bounding sphere by 5 cm
  when the caller's accept test takes it and by 1 mm when it does not.
  The mesh is asked from the point where the ray crosses the grown
  sphere's top for what is left of the reach, the crossing added back to
  the hit (an eye under the top asks from the eye with the whole reach); the last node that answers wins, its face is the callback's
  count less one, and a miss records the ground plane (`nav_pick::ground_hit`).
  Modelled with tests; nothing calls it yet because the cursor still uses
  the aim plane and the hand picks through the physics world.
- **Middle mouse** orbits: 4.8 rad per screen width (the division in
  double precision), through a near critically damped spring (stiffness
  0.0718 and retention 0.616 per tick), leading by at most 45° per tick.
- The camera's **cut-away detector** runs each frame on the stepped
  camera. Its segment starts twice the camera's largest distance out
  along the backward vector from the looked-at point (beyond the eye) and
  runs back towards the point for twice that distance less a metre. It
  probes the meshes of the sectors the first view draws, in each sector's
  space: a candidate mesh is marked when the segment meets its bounding
  sphere (and, by kind, the mesh's own segment test), and a mesh over the
  player is marked when a 10 m segment straight down onto the point meets
  it; a mark carries to the parent and the mesh children when their bits
  say so, and a miss clears a marked mesh
  (`exa_world::scene_nodes::Scene::detect_cut_away`). The model's fade
  then moves a marked mesh toward dithered or partial drawing. The
  detector and the fade are not on the draw path yet (`room_view` still
  draws from the room states), and with no CPU triangles the tests stop
  at the bounding sphere.
- **Wall tilt**: each camera tick ends with a probe from 50 cm under the
  followed point along the view, alternately testing the present step
  (30 cm ball, a hit climbs to at most 4) and the step below (20 cm, a miss
  comes down); the tilt eases toward 0.15 rad steeper per step and is added
  to the axis pitch, held to 1.54 rad. Modelled (`ChaseCamera::probe_wall`),
  but nothing answers the probe in the live game yet: the original sweeps
  only level meshes with certain flag bits, and Exarust's level colliders
  hold every static mesh, ceilings included.
- **The idle turn.** Each frame the player's controls test whether the
  player is idle: the pointer within 2 pixels of where it was last seen on
  both axes, the condition word plain (0, so not walking or attacking), no
  foot stepping, the left button holding nothing, and no mouse button
  held. A dead player counts as idle whatever the pointer does. When the
  player is busy the pointer's place and the game clock are noted. When it
  is idle and the game clock is past the noted time plus 10 000 ms, the
  camera's target yaw turns by 0.0001 rad for each millisecond since the
  last frame (0.1 rad a second, in double precision, then single), less a
  whole turn once it passes 2π, and the behaviour stops reading the cursor
  until the player is busy again. A screen that blocks the controls turns
  it with only the ten-second test; an open window (the active view not the
  game's) only restamps the clock, and a dialogue or the power bar keeps the
  player busy (`idle_orbit::Frame`). Exarust's in-game menu is such a
  window; no Exarust screen blocks the controls yet, and there is no
  dialogue or power bar.

## Start outfit

- The original's human body is nude. Underwear is an item, "Braies_A"
  (pale grey linen braies), and the engine keeps every human record it
  creates or re-dresses in it: when nothing worn covers the groin, the
  braies go on. Trousers, hose and braies cover it; shirts, mail legs and
  plate do not ([formats/OTHER.md](formats/OTHER.md), Underwear).
- A new character starts in braies; story background 0 keeps nothing else
  but a start letter, the proctor and villager start in their templates'
  clothes.
- Exarust's play mode starts the player as that new character without a
  kit: braies only. `--outfit` puts apparel on over them, and an item that
  covers the groin takes their place (`--outfit trouser_a,shirt_b` wears no
  braies). The character view's human body is kept in braies the same way.
- Not yet: taking clothes off and the braies coming back, and conflicts
  between garments given together; they wait for equipping.

## The carried torch

- The torch (`torcha02`, item "Torch A01") goes in one of two hand slots:
  - the main hand, the **right** (bone 17): the moveset becomes the
    **"Torch"** overlay on "One Handed", which raises the torch beside the
    head out of combat and carries it before the chest in combat (below);
  - the off hand, the **left** (bone 18): **"1H & Torch"**, the left hand
    at shoulder height, about 12 cm forward.

  There is no additive arm layer: the whole moveset changes, in and out of
  combat, as the held items pick it ([Moveset from held items](#moveset-from-held-items)). A new game puts the torch in the off hand for two of the four
  backgrounds; the player in the reference captures holds it raised in the
  right hand.
- **The two carries** of the right hand. "Torch" defines four clips, Walk,
  Cmbt - Idle, Attack Mid and Idle, each replacing One Handed's whole; the
  guard (Defence stance) and the other attacks are One Handed's. The two
  standing clips' arm, from their first key on the body model's bone
  offsets, in the chest's frame (the elbow is the angle between the upper
  arm and the forearm, 0° straight; the wrist is the hand's origin from
  the shoulder joint):

  | clip | played | elbow | upper arm | wrist from the shoulder |
  |---|---|---|---|---|
  | Idle | out of combat mode | 122.6° | 39.8° out to the side, 11.4° forward | 16.4 cm out, 1.4 cm up, 18.7 cm forward |
  | Cmbt - Idle | in combat mode, after an attack or with a target near (and kept, above) | 101.5° | 24.9° out, 31.8° forward | 8.6 cm out, 3.0 cm down, 31.6 cm forward |

  Out of combat the torch is up beside the head, the flame about 27 cm over
  the top of it, the elbow near its link limit (Exarust's body uses 99.6 %
  of the elbow's range there); in combat it is the compact carry, the upper
  arm tucked, the hand before the chest, the flame about 21 cm over the
  top of the head and 11 cm before it. The
  user's reference shots of the original carrying the torch before the
  chest show **combat mode** (the red cursor ring is the original's combat
  cursor), its Cmbt - Idle; exploring, the original raises it as Exarust
  does, with "Torch" Idle (REA: its steering sets the Idle stance; not yet
  watched live). In Exarust: Tab, a left click, and the attack ends in
  that carry, kept when Tab turns combat mode off, until the player walks.
  The walk's arms hang loosely (stiffness 0.2): "Torch" Walk asks for the
  elbow at 125–129° and the upper arm 37–49° out.
  Measured equal to the original's (REA and its running game, read only):
  the grip below, the overlay (whole clips), the arm's muscles in
  Cmbt - Idle (stiffness 0.4, part factor 0.5, relaxation 0.36, saturation
  0.1; no arm state for a held light) and how closely both bodies follow
  the clip (within 1°). The torch weighs next to nothing on the hand: the
  hand takes 5 % of each grip correction, the torch the rest.
- Exarust's play mode gives the player a lit torch in the story levels
  (and scenes no game mode plays): in the right hand, or in the left beside
  a weapon given with `--weapon`. Practice (`arena`, `arenae`) and the arena
  career (`arenahub`, the match arenas such as `arenao1`) start without it:
  before those sessions the original clears the torch from both hands, and
  the arena keeps only its starter clothes and weapons.
- Each hand holds the item by the original's grip table
  ([Held items and their grips](#held-items-and-their-grips)): in the
  right hand as the first held item, a one-handed stick, (−12, −1.5, 6) cm
  and (70°, −15°, 20°); in the left as the second, the off-hand grip,
  (11, −0.5, 6) cm and (110°, 15°, 160°). The offset scales with the
  holder's size and moves by its race's held offset (a human of average
  height holds it exactly there). The rotation builder puts the item's X
  axis on (cos z·cos y, sin z, −cos z·sin y) and its Y axis on
  (sin x·sin y − cos x·sin z·cos y, cos x·cos z, sin x·cos y +
  cos x·sin z·sin y): applied to the item, the turn about X first, then
  Z, then Y, all about the hand's axes. Not yet: the hand: the original's
  lies open under the torch, the handle in its palm, where Exarust's looks
  curled (a skinning or hand-mesh question, not looked into yet).
- Not yet: the original's torso twist and head look toward the cursor. In
  Cmbt - Idle it turns the belly about 8°, the chest 15° and the head 19°
  about a near-vertical axis, while the arms keep the clip's turn relative
  to the pelvis, so its arm ends about 7° off the clip; Exarust's trunk and
  head stay on the clip (M8).
- The light is the torch's own `_Fire` emitter. A moving torch dirties all
  its cube faces every frame it moves; its flicker never does. Nothing keeps
  it off its holder: the original has no owner test in its light selection,
  and only the torch's own mesh stays out of the torch's shadow map.
- The original adds no separate glow around the player, except with
  night-vision headgear or one Mind power (The player light, below).
- `exarust character Torch/Idle` (or any "Torch" or "1H & Torch" action)
  shows the body holding the lit torch in the hand that moveset raises: the
  right for "Torch", the left for "1H & Torch".

## The player light

The player's own light burns only while the player wears an item whose
body has special kind 1 (it sees in the dark: the three night-vision helms
"UW Helm NV", "UCA Helm A" and "UGA Helm FB"), and goes out while either
hand of the record's out set holds an item with a held light's section (a
torch; in Exarust also the torch the body carries). The switch is read
whenever what the player wears or holds changes (the original reads it at
the start of play, after a respawn and on every change of equipment).

The light is an omni light, colour (1, 1, 1), radius 1200 cm, multiplier
2, softness 4, falling off with distance. It is made at 245 cm with the
intensity 0.01 and rises toward 0.4: each frame while it is under 0.4 less
the single-precision epsilon it gains 0.0005 per millisecond of the frame
(in double precision, then single), clamped to 0.4, so it reaches 0.4 in
about 0.78 s. Each frame it hangs 80 cm above the player's head; when that
is more than 1 cm up, a 10 cm ball swept up from the head through the level
(not the props) puts it where the ball first meets the ceiling. Nothing
moves while the game is paused.

The second off rule (an item in either hand whose weapon section is of
kind 3 while the player's held part carries its lit flag) is modelled
(`player_light::weapon_puts_out`), but Exarust's bodies keep no such flag
yet, so the running switch passes it off. Not
in Exarust: the Mind power that forces it on, and the spectating view's
light. A normal start wears no such helm, so the light stays off, as in the
original.

## Unattended runs

`--autowalk` holds W and ignores every other key reaching the window, so a
scripted run is not steered by typing elsewhere.

`--trace <file.csv>` writes the player's body state after every 83 Hz tick,
in the columns of the retail body probe (`docs/formats/RSQ.md`, Physical
body), the stance factor, tone target, support, balance and its counter
included (standing still: 1, 80 and 1). Steering already runs once per body
tick, as the original runs it.

The body keeps its own balance, as the original measures it: its chest
against its ankles, and the ground under each foot. Standing still it never
falls; standing with no ground under its feet (dropped, or walked off a
ledge and stopped) it loses its balance and falls. Off its balance (under
0.6) it is not steered: upright it takes the Defence stance or Rebalance,
leaning it staggers (a get-up played partway) or falls forward or
backward; lying still under 0.3 it gets up once its feet give support or
1.2 s after they lost it, with Get up from its front or Get Up 2 from its
back, turned the way it rises; unsteady on its feet (0.6–0.8) it takes the
Defence stance or Rebalance ([formats/RSQ.md](formats/RSQ.md), Falls and
get-ups). A standing player whose capsule perches on a stone its feet hang
beside keeps its balance and drops onto its feet (Exarust's own, while the
capsule leads the body).

`--press key@frame,...` presses keys at frames after the load
(`key@a-b` holds one from frame a to b, and `LMB`, `RMB`, `MMB` and `Ctrl`
are keys too), `--click frame,...` clicks the left button,
`--cursor x,y` holds the pointer at a window position, and in
play mode `--camera x,y,z,yaw,pitch` puts a fixed camera there instead of
the chase camera, for unattended pictures. The body overlay (F3), the joint
readout and trace, and the pause (P), step (`.`) and speed (`[`, `]`) keys
are described in [DEBUGGING.md](DEBUGGING.md). The chase camera runs on
real time, so it keeps orbiting while the simulation is paused or slowed.
