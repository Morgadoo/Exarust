# Player control, camera and the carried torch

How the original steers the player and frames the dungeon, and how Exarust
(`exa_game::player`, `exa_game::torch`) does it. Established by REA on
`TPlayerBehaviour`, `TNavCameraControl` and the body tick; the notes, with
addresses, are in the git-ignored `context/re/player-control.md`.

## Movement (exploration)

The original has no camera-relative keyboard movement. Outside combat:
- **W** walks the way the character faces (its heading); neither the camera
  nor the cursor changes the heading.
- **A / D** steer the heading by ±0.025 rad per body tick (83 Hz, about
  119°/s) while moving. With W held this walks a circle of about 0.67 m by
  design. Standing still, A or D turns a quarter (90°), at most once a
  second.
- **S** backpedals. Faster than 1 cm per tick, it first brakes; it never
  turns the character about.
- **Right mouse** walks toward the cursor's point on the hip-height plane,
  running when it is more than 3 m away. A reversal of more than 120° turns
  before walking.
- **Shift** runs: a run blend rises 0.05 per tick and falls 0.015.
- The body's yaw follows the heading at up to 240°/s, ×0.9 with the keys,
  slower when moving fast, and eased over the last 0.6 rad. There is no
  turn-in-place clip.
- A look-ahead of 75 cm + 6·v² cm (v in cm per tick) stops the character in
  front of a wall instead of sliding along it.
- Walking settles near 1.4 m/s; backpedalling is the walk scaled by the gait
  table's thrusts (77.8 against 88.9). Exarust's run speed is a measured
  guess until the jog row is simulated.
- Moving firms the body up: its stance factor rises 0.01 per tick from the
  race's 0.5 to 0.7 walking and 1 running (it keeps what it reached until
  the character stops), so its link muscles pull with tone × 0.7 or × 1
  instead of × 0.5 and damp with k = 50. In the walk and run cycles the arms
  hang loosely (stiffness 0.2); while the body turns toward its heading
  they stiffen with the turn left (0.4 per rad, at most 0.4)
  ([formats/RSQ.md](formats/RSQ.md), Physical body).

## Combat mode

**Tab** toggles it. The character faces the cursor; W, S, A and D step
along and across that direction, and keys held longer than 200 ms walk on.
The left button attacks; the mouse's vertical motion picks the strike.

## Camera

- The camera follows the player's position only, at a fixed **20 m**. It
  never turns by itself in exploration.
- **Zoom is field of view**: fov = zoom / 2000 × 45° × √(1.6 / aspect),
  clamped to 10–100°. The wheel steps through six stops from 800 to 2000
  (18° to 45°); the game starts at 1300.
- **Pitch**: 0.8 rad below the horizon under zoom 1000, then 0.000125 rad
  steeper per unit up to 0.925 rad at 2000. A vertical orbit drag can raise
  it to 1.5 rad. The pitch moves 10 % of the way per 17 ms camera tick.
- **Middle mouse** orbits: 4.8 rad per screen width, through a near
  critically damped spring (stiffness 0.0718 and retention 0.616 per tick),
  leading by at most 45° per tick.
- Not yet in Exarust: the original tilts 0.15 rad steeper per step (up to
  4) when a wall blocks the view, and fades geometry between the camera and
  the player.

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
    **"Torch"** overlay on "One Handed", which holds the right hand at
    shoulder height, about 25 cm forward;
  - the off hand, the **left** (bone 18): **"1H & Torch"**, the left hand
    at shoulder height, about 12 cm forward.

  There is no additive arm layer: the whole moveset changes, in and out of
  combat. A new game puts the torch in the off hand for two of the four
  backgrounds; the player in the reference captures holds it raised in the
  right hand.
- Exarust's play mode gives the player a lit torch in the story levels
  (and scenes no game mode plays): in the right hand, or in the left beside
  a weapon given with `--weapon`. Practice (`arena`, `arenae`) and the arena
  career (`arenahub`, the match arenas such as `arenao1`) start without it:
  before those sessions the original clears the torch from both hands, and
  the arena keeps only its starter clothes and weapons.
- Each hand holds the item by the original's grip table, offset (cm) and
  Euler angles (degrees) in the hand's space:
  - right hand: (−12, −1.5, 6), (70°, −15°, 20°);
  - left hand: (11, −0.5, 6), (110°, 15°, 160°).

  The rotation builder puts the item's X axis on (cos z·cos y, sin z,
  −cos z·sin y) and its Y axis on (sin x·sin y − cos x·sin z·cos y,
  cos x·cos z, sin x·cos y + cos x·sin z·sin y): applied to the item, the
  turn about X first, then Z, then Y, all about the hand's axes. Not yet:
  the offset's scaling by character size and its per-race correction, and
  the hand closing around the handle.
- The light is the torch's own `_Fire` emitter. A moving torch dirties all
  its cube faces every frame it moves; its flicker never does. Nothing keeps
  it off its holder: the original has no owner test in its light selection,
  and only the torch's own mesh stays out of the torch's shadow map.
- The original adds no separate glow around the player, except with
  night-vision headgear or one Mind power.
- `exarust character Torch/Idle` (or any "Torch" or "1H & Torch" action)
  shows the body holding the lit torch in the hand that moveset raises: the
  right for "Torch", the left for "1H & Torch".

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
ledge and stopped) it loses its balance and falls, then gets up once
settled. A standing player whose capsule perches on a stone its feet hang
beside keeps its balance (Exarust's own, until the body stands on its
feet).

`--press key@frame,...` presses keys at frames after the load, and in play
mode `--camera x,y,z,yaw,pitch` puts a fixed camera there instead of the
chase camera, for unattended pictures. The body overlay (F3), the joint
readout and trace, and the pause (P), step (`.`) and speed (`[`, `]`) keys
are described in [DEBUGGING.md](DEBUGGING.md). The chase camera runs on
real time, so it keeps orbiting while the simulation is paused or slowed.
