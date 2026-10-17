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
- **Shift** runs: a run blend rises 0.05 per tick, at most to 1 − 0.3 ×
  the character's encumbrance (what it wears and the shield it holds,
  [formats/OTHER.md](formats/OTHER.md), Encumbrance), and falls 0.015. A
  new character in braies runs at the full blend; in a tourney plate
  harness, helm and leggings (encumbrance 0.625) the blend stops at 0.8125
  and the run at about 3.2 m/s instead of 3.6.
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
- Stairs: the capsule climbs a step before the feet do, and nothing lifts a
  walking foot onto it yet (the original's walking feet and leg IK come
  later). Until then the body is held no lower than 6 cm under the
  capsule's floor (Exarust's own), so it climbs with the capsule, its feet
  dragging behind, and stands on the stairs; on level ground nothing
  changes ([formats/RSQ.md](formats/RSQ.md), Physical body).
- Falling: a player off its feet that sinks faster than 5 cm a tick
  screams once; with no ground within reach under it (25 m) it is lost:
  1 s later the pit sounds and the screen fades to black over 3 s, and 4 s
  after the loss the player dies and leaves the world, wherever its body
  is by then. The clock runs only while the body still falls or tumbles
  off its balance, as the original's; slowing down does not save it
  ([formats/RSQ.md](formats/RSQ.md), Falling to death; [UI.md](UI.md),
  Death fade). `--at x,200,z` drops it from 200 m.

## Combat mode

**Tab** toggles it, and every practice or arena fight starts with it on, as
the original turns it on at each fight start (with `--opponent`, once the
level shows; not in the arena hub). W, S, A and D step along and across
the cursor's direction, and keys held longer than 200 ms walk on. The
original's footwork steps the feet and never changes the clip; Exarust's
interim one plays "Combat Walk" while a key is held and gives back the
clip it replaced (the guard stays the guard) when they are released. A
walk or run still playing when Tab turns combat mode on ends in the stance
once the body stands, as the original's walk does. While its condition
word is helpless (down, getting up, collapsing, dead) the player is not
steered, its body does not turn and it engages no attack, as the
original's player behaviour and body tick skip them for mask `0xAC0`.

- **Facing.** The heading turns to the cursor's direction once it is more
  than 30° off it, and follows it exactly while an attack is requested
  (the left button held). The body turns toward the heading as in
  exploration: up to 240°/s, eased over the last 0.6 rad; while it
  attacks, at (1 − 0.6·enc)·(1 − 0.25·progress)·B·W16·P / scale a tick
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
- Not yet: the swinging arm's stiffness (W20·L(p)), swing sounds, the
  AI's own release and attack choice (L11), recoil and the
  follow-through after a wound (C21, M26), the aim nudges (M23), parries
  (L10).

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
| a duellist entering combat (at the fight start) | Defence stance |
| a duellist in combat | Defence stance at 4 reaches or more from its target, Cmbt - Idle within |
| any fighter, dazed (its energy more than 0.05 under its most, compared in double precision), every body tick | Defence stance |

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
- **The daze.** While a fighter's energy is more than 0.05 under its most
  its body tick makes its stance the Defence stance, and, unless it is
  down or attacking (its condition word helpless or attacking,
  [formats/RSQ.md](formats/RSQ.md), Condition; or in Exarust's fall),
  starts the Defence stance when another clip plays,
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
- **Readiness.** Standing in Cmbt - Idle, with that stance, a fighter's
  readiness rises 0.004 a tick (0 to 1 in 3 s); anything else resets it.
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
  start their clips with no blend; the original's locomotion starts come
  with C18);
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
shields and a second weapon (no Exarust item holds them), held items on
creatures, and the flaming weapon (its config adds `0x10020000`, whose
shared bit also lays the Torch overlay on a one-handed weapon).

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
