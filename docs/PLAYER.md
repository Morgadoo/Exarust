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

- The torch (`torcha02`, item "Torch A01") sits in the off-hand slot, the
  **left hand** (bone 18). A new game puts it there for two of the four
  backgrounds. Exarust's play mode always gives the player one.
- Carrying it switches the whole moveset to **"1H & Torch"**, which holds
  the left hand at shoulder height, about 12 cm forward. There is no
  additive arm layer.
- The original's grip table gives the off-hand item an offset of
  (11, −0.5, 6) cm and Euler angles (110°, 15°, 160°) in an order not yet
  pinned. Exarust holds the torch upright in the fist until that order is
  confirmed.
- The light is the torch's own `_Fire` emitter. A moving torch dirties all
  its cube faces every frame it moves; its flicker never does.
- The original adds no separate glow around the player, except with
  night-vision headgear or one Mind power.

## Unattended runs

`--autowalk` holds W and ignores every other key reaching the window, so a
scripted run is not steered by typing elsewhere.
