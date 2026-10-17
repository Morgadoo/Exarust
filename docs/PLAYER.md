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
- **Right mouse** walks toward the cursor's point on the plane 80 cm above
  the player's ground (the original's aim plane, about hip height),
  running when it is more than 3 m away. A reversal of more than 120° turns
  before walking. The point is found through the scene camera's own
  viewport: with supersampling the scene is drawn into an image twice the
  window's size, and window positions are scaled to it.
- **Shift** runs: a run blend rises 0.05 per tick and falls 0.015.
- **The clips.** The steering starts the walk, run or backpedal when the
  character moves, from whatever looping clip plays, and ends it in Idle
  when the character stops. It starts no other clip: what a standing
  character plays is its stance's ([The combat stance](#the-combat-stance)),
  so a player that turned combat mode off in Cmbt - Idle keeps it, also
  while turning in place, until it walks. (REA: the original's exploration
  steering reaches the clip start only through the set-stance helper.)
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
  instead of × 0.5 and damp with k = 50, and its pose drive turns the
  pelvis, spine and feet with a gain of 0.0497 or 0.0836 instead of 0.0314.
  In the walk and run cycles the arms hang loosely (stiffness 0.2); while
  the body turns toward its heading they stiffen with the turn left (0.4
  per rad, at most 0.4) ([formats/RSQ.md](formats/RSQ.md), Physical body).
- The body stands on its feet, as the original's does: a lift holds its
  pelvis at a height over the ground under its feet (1.7 cm over the
  target, 101.5 cm for an idle human), its height comes from the clip
  root's over each ankle, and it falls by gravity once its feet have no
  ground. The controller only moves it across the ground and turns it,
  and holds its pelvis over where it carries it (Exarust's own). Walking,
  the pelvis bobs with the walk's root (about 2.5 cm). A duellist's hits
  daze the player without swaying its balance: the lift and the legs carry
  it ([formats/RSQ.md](formats/RSQ.md), Physical body).
- Stairs: the capsule climbs a step before the feet do, and nothing lifts a
  walking foot onto it yet (the original's walking feet and leg IK come
  later). Until then the body is held no lower than 6 cm under the
  capsule's floor (Exarust's own), so it climbs with the capsule, its feet
  dragging behind, and stands on the stairs; on level ground nothing
  changes ([formats/RSQ.md](formats/RSQ.md), Physical body).

## Combat mode

**Tab** toggles it, and every practice or arena fight starts with it on, as
the original turns it on at each fight start (with `--opponent`, once the
level shows; not in the arena hub). W, S, A and D step along and across
the cursor's direction, and keys held longer than 200 ms walk on. The
original's footwork steps the feet and never changes the clip; Exarust's
interim one plays "Combat Walk" while a key is held and gives back the
clip it replaced (the guard stays the guard) when they are released. A
walk or run still playing when Tab turns combat mode on ends in the stance
once the body stands, as the original's walk does.

- **Facing.** The heading turns to the cursor's direction once it is more
  than 30° off it, and follows it exactly while an attack is requested
  (the left button held). The body turns toward the heading as in
  exploration: up to 240°/s, eased over the last 0.6 rad. A cursor moved
  less than 30° leaves the character facing where it faced.
- **The attack request**, kept each behaviour tick (83 Hz) as the
  original keeps it: a left press requests an attack, holding the button
  keeps the request standing, releasing it drops the request. A press
  asks for:
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
  - an overhead unless a stab or an overhead plays; a stab unless a stab
    plays, and only with the weapon's long axis forward of −0.367 (its
    forward component in the body's frame); a side attack only when no
    side attack plays.

  So a held button repeats the attack after each cooldown, on the main
  side (every attack clears the side of the press). The cooldown never
  passes while an attack plays (every attack tick restarts it), so a
  double click's overhead follows the first attack once it has ended and
  its cooldown has passed, if the second press is still held. The
  duellist's strikes (Exarust's own choice, every 2.2 s within 1.2 m,
  until the original's AI) pass the same energy test and cooldown.
- **What strikes.** Empty hands are the fists (or claws), each a striker
  built from the race's fist or claw record across its hand's box
  ([formats/OTHER.md](formats/OTHER.md), Weapon record). With the fists
  only the motion's hand strikes: the right fist in the main side and the
  overhead, the left in the other side (both claws in a clawed overhead).
  A held weapon strikes in every attack and the fists then never do. Each
  attack makes the striking hand its primary striker; equipping a weapon
  makes the weapon the primary at once.
- Not yet: the swing clock (attacks run at 60 frames a second), the
  released swing's stall, the overhead or feint a released swing allows
  and Remise (a perk) (M22); parries (L10).

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
| the player in combat mode, dying (yellow health under 0.01) | Defence stance |
| the player in combat mode, its target nearer than 3.5 m × the target's body scale (chest to chest), or within 3 s of its last attack's last tick | Cmbt - Idle |
| the player in combat mode otherwise | Defence stance |
| a duellist entering combat (at the fight start) | Defence stance |
| a duellist in combat | Defence stance at 4 reaches or more from its target, Cmbt - Idle within |

- **The player's target** is picked every tick among the other living
  fighters within 5 m: the best (5 − d)·(cos + 0.5), the angle measured
  from the cursor's direction, so never one more than 120° off it.
- **The guard.** Unarmed, the Defence stance of the "Default" moveset is
  the fists-up guard: hunched, the fists before the chest and chin, knees
  bent (reference shots 1 and 2). Armed with "One Handed" it holds the
  sword hand at mid-chest; Cmbt - Idle holds the hands, or the weapon, low
  before the hips (reference shot 6; the blade's angle is the grip's,
  L12).
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
- **Readiness.** Standing in Cmbt - Idle, with that stance, a fighter's
  readiness rises 0.004 a tick (0 to 1 in 3 s); anything else resets it.
  A blow's wound (penetration, impact, stab energy) is scaled by 1 − 0.5 ×
  readiness, so a fighter ready for 3 s takes half the wound; the shove is
  not lessened ([formats/OTHER.md](formats/OTHER.md), Damage).

Not yet, with the change that brings each:
- the daze: a dazed fighter (energy more than 0.05 under its cap) takes
  the Defence stance, and its body holds the pose it is physically in
  (C10, which lands both: without the pose hold the guard would rise at
  every dazing blow, which the original never shows);
- the blends of every stance change (30 frames for the switch, 2 × the
  weapon's recovery time at an attack's end, …), C10: Exarust's stance
  changes snap and the muscles smooth them;
- the Defence stance and Rebalance a fighter takes from its balance
  (below 0.8) or its tone (below 0.85), C11;
- parries, whose clips are stances while they last, and readiness while
  parrying or with an attack ready to chain (L10, C21);
- the original's line-of-sight test on the player's target; the AI's own
  footwork in its stance instead of the combat walk, and its parries (C16,
  L11). The live retail opponent took its guard for 0.4 s at the fight
  start before Cmbt - Idle; Exarust's duellist holds it one frame when it
  starts within four reaches (that part of the original's AI is not
  traced).

## Moveset from held items

What a character holds picks the moveset its clips come from, after every
equip change and in or out of combat mode, as the original picks it. The
same choice gives a stance class (what the other arm does while one
swings) and a weapon class (the held weapon's hands bits, `0x80` for the
fists). A moveset lacking a clip falls back to its parent ("One Handed" to
"Default").

| held | moveset | stance class | weapon class |
|---|---|---|---|
| empty hands (the fists) | the race's own ("Default") | 5 | `0x80` (`0xC0` for a clawed race) |
| one one-handed weapon | "One Handed" | 1 | `0x10` |
| two one-handed weapons | "2W Light" | 6 | `0x90` |
| a two-hander with flags `0x521` | "Greatsword" | 2 | `0x20` |
| another two-hander, kind bit 0 set (bladed, stick) | "2H Light" | 2 | `0x20` |
| another two-hander | "Two Handed" | 2 | `0x20` |
| a polearm | "Polearm" | 2 | `0x30` |
| a shield (beside anything) | "1H & Shield", or "Shield C1" / "Shield C2" by two character flags | 3 | the main item's |
| a light in the off hand | "1H & Torch" | 4 | the main item's (`0` beside the fists) |
| a light in the main hand | "One Handed" with the "Torch" overlay (Exarust's "Torch" moveset) | 5 | the light's (`0x10` for the torch) |

The rows are tried top to bottom from the shield down; the weapon flags
are the low 16 bits of the item's weapon section's first word
([formats/OTHER.md](formats/OTHER.md), Weapon stats). The torch item is a
one-handed stick (flags `0x13`).

In Exarust the player and the duellist follow this rule: the player
unarmed fights with the "Default" guard and punches, armed with "One
Handed" (also idling with it out of combat), and with a torch as below. The character
view keeps the moveset its command line names, and level characters keep
their race's moveset until they have equipment. Not yet: shields and a
second weapon (no Exarust item holds them), the "Torch" overlay over a
shield moveset, and the armed movesets of other races, which the original
looks up under the race's own moveset.

The cursor is the game's own world cursor ([UI.md](UI.md), World cursor):
a red ring in combat mode, and outside it an amber arrowhead over ground
within reach, a cross beyond it and chevrons while walking to it. Not yet
in Exarust: the original's right button also walks to the cursor in
combat mode.

## Camera

- The camera follows the player's **pelvis** only, at a fixed **20 m**: the
  pelvis as it is drawn in the frame, between its last two body ticks, so
  the view rises and falls with the walking body (the feet before the
  player has a body). It never turns by itself in exploration.
- The camera **ticks every 17 ms** of real time: a tick runs once the frame
  passes its start, at most nine a frame (a longer stall drops the rest).
  Each frame shows the direction to the eye, the zoom and any shake offset
  between the last two ticks, by how far the frame is into the last one, so
  the view moves smoothly on any refresh rate. A shake offset moves the eye
  only, and only while it is over 1e-5 cm² squared. A **kick** moves the
  offset one way and the last tick's the other; each tick then keeps 0.96
  of its change and 0.99 of the whole, a wobble of about a second that
  dies away, and drops it once both are tiny (change within 1e-7 cm²,
  offset within 1e-4 cm², squared). Nothing kicks the camera yet.
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
- Each hand holds the item by the original's grip table, offset (cm) and
  Euler angles (degrees) in the hand's space:
  - right hand: (−12, −1.5, 6), (70°, −15°, 20°);
  - left hand: (11, −0.5, 6), (110°, 15°, 160°).

  The rotation builder puts the item's X axis on (cos z·cos y, sin z,
  −cos z·sin y) and its Y axis on (sin x·sin y − cos x·sin z·cos y,
  cos x·cos z, sin x·cos y + cos x·sin z·sin y): applied to the item, the
  turn about X first, then Z, then Y, all about the hand's axes. Not yet:
  the offset's scaling by character size and its per-race correction, and
  the hand: the original's lies open under the torch, the handle in its
  palm, where Exarust's looks curled (a skinning or hand-mesh question,
  not looked into yet).
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
