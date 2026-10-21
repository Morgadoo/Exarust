# `exa_combat`

Combat resolution: contacts between weapons and bodies, impact energy, armour coverage and layers, damage types and injuries.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

It already decides when a fighter may attack and which of its strikers
strike, by the original's attack executor shared by the player and the AI
(`attack`: 80 % of the most energy, the cooldown from the last attack tick
on the 12 ms game clock, 400 ms or round(300 × the primary's recoil) +
300 ms, nothing while a swing stalls but a released swing's overhead
within 300 ms or Feint's renewal, the choice of attack and the side's
pick without repeat, the motion's hand with the fists; rules in
[`docs/PLAYER.md`](../../docs/PLAYER.md), Combat mode), and resolves the
contacts of blades and fists by the original's damage model
(`damage`, with the weapon record of `exa_formats::weapon_record`
re-exported as `damage::WeaponRecord`; two armour layers per part, filled
from the protection each worn item stores, coverage widened by Deflection;
rules in [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md), Armour)
and keeps each fighter's combat stance and readiness (`stance`:
the looping clip it returns to, the body tick's switch to it that skips
Cmbt - Idle, the player's and a duellist's rules, the daze's Defence
stance, the original's blend and ease for every stance start, readiness
halving the wound after 3 s in Cmbt - Idle; rules in
[`docs/PLAYER.md`](../../docs/PLAYER.md), The combat stance).

A fighter's swing (`swing`) is run every attack tick by the original's
swing routine (`SwingState`, `run_swings`: in the character's attack
stage, after its posture stage and before its health bookkeeping and its
clip's advance): the clip clock runs at its primary striker's swing speed (the
record's W15, slower at the ends of the swing by its shaping W18, ramped
by the feral level), tired (E/Emax)², and the attack ends into the stance
with twice its stance blend W19; a swing strength P scales it while the
swing stalls — released (its request dropped: by max(1 − 0.05·|v|,
0.1)/W17 × 0.03 a tick) or struck (the daze's 4·D·f − (1 − D)·
min(0.005·s, 1)·W17) — until the stance returns within the swing's
expected rest; the strike window opens at 0.3 of the clip (stab 0.4,
overhead 0.5) and the overhead's and stab's close late; Remise chains a
held side attack past 0.8; the body's attack turn rate comes from the
handling W16. An attack starts eased smoothly, the other side with
round((20 − W19) × 0.6) frames of blend, the stab and the overhead with
the executor's (`attack::start_blend`)
([`docs/formats/RSQ.md`](../../docs/formats/RSQ.md), Motion playback;
[`docs/PLAYER.md`](../../docs/PLAYER.md), Combat mode).

A blocked striker recoils (`recoil`): strength and the stall rate become
0.5 and the released bit clears. A class word of 0x90 that is already
continuing, or whose clock widens past 0.4 while a counter starts, stops
there. Any other swing sets stalling, cleaving and recoiled, blend 40 and
both arm stiffnesses to 0.2. The stance returns after the widened drive
times 0.1 times 400, rounded half to even (the conversion rounds, it does
not truncate), plus 200 ms. A wound's follow-through
(`follow_through`) uses that same skip and otherwise writes nothing until
it runs. The scale is the widened gain times recoil times speed times 0.15,
floored at 0. A body zone at or under the epsilon sets strength 0, and a
larger zone uses its square root times the scale. An edge uses one minus
the zone times the scale plus the zone. Strength caps at 1. The stall is
the widened one minus strength times 0.05. The end is the scale times 400,
rounded half to even.

## Damage

`damage::resolve` is the original's damage routine in its order and
precision (constants multiplied in double where the original does): the
speed capped at 30 and softened under 10 (then the overhead's 1.5), the
swing's and the stab's impact, the hit point along the weapon, the zone
(a stab's tip and lunge, a hanging striker's and the fists' shares in
full, the haft, past the striking part, each kind's tip zone, the haft's
crush bonus by its material), unarmed 0.6·√(3I), one coverage roll over
the outer then the inner layer, the channels through padding and
resistance raised by the engine's fast logarithm and exponential
(`fast_math`), the shove, the readiness, the daze with its stall factor
(`Victim`: Security, a potion, a steadfast body) and the bars. It
returns an `Outcome` with what the rest of the hit reads: the impact at
the hit, the blow sag (min(2·I, 1)), the zone and whether the edge or the
weapon's body struck, the bite and its smoothing, the hit sound's volume,
pitch and muffling and each layer's ringing (`hit_sounds`). A blow on a
shield or a weapon (`Guard`) meets no layer and draws no roll: a weapon's
parry deals nothing and sags the parrier (`parried`), a shield takes it on
its arm's stamina and leaks yellow (`block`, the block path with its
Synchrony, Persist and Counter rules). Rules and
numbers: [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md), Damage.

The hit reaction then runs the striker's side (`SwingState::after_wound`):
the wound bits on its swing, its striker's window shut, and a cleave or a
follow-through by its size, the blow's impact and where on the weapon it
struck; a guard (for a striker under 1.4) or a body that turns blows
(`StateFlags::TURNS_BLOWS`, checked before a parry) recoils it
(`SwingState::recoil_from`), and a clash or a block marks the swing's
weapon and shield sounds once a swing (`strike_back`). Each contact's
striker is dealt right after its struck side, as the original's routine
runs per contact. The blow sag rides the `Shove` to the struck body.

A clash on a weapon parry takes one of the original's three branches
(`parry::clash_branch`): a raised parry clears the victim's attack state,
locks it 600 ms unless it has Riposte and sags its stance by half the
impact, as before; a parry that counters this very motion with a weapon
other than its primary ends into its stance with no lock and no sag
(`ParryState::end`); a spent parry changes nothing. A recoil keeps its
rest on the swing (`SwingState::recoil`) until the stall ends, and the
stance that ends it starts with the rest's blend (40 frames) and both
arms' stiffness (0.2, `Ragdoll::set_arm_stiffness`), which the next
character tick eases back up, instead of the stance's recovery blend. A
wound on a boned limb adds its size to the struck fighter's bleeding tally
(`Fighter::bleeding`, the part's slot 39), which the body tick turns into
damage over time.
A Force effect's interrupt (`SwingState::force_interrupt`) breaks an
attack into a full-strength stall with no drain, cleaving and recoiled,
its window shut and the stance 600 ms on; nothing calls it yet, and the
original's 50·clock attack-reach penalty has no attack reach to take it
from.

## Grips

`grip`: the original's grip table (`GRIPS`, six entries by hand class:
fists, one-handed and the first of two weapons, two-handed, polearm,
claws, the off-hand item and the second of two weapons), the hand class of
a held item from its held slot and weapon class (`hand_class`, `grip`),
and each grip's hand (the left for a polearm), offset (times the
character's size; the ogre's own, unscaled), Euler angles (about the
hand's X, then Z, then Y) and links (to the hand, the second hand and the
arm, and the stab's; used once weapons are bodies). `Grip::placement`
places an item for a `GripHolder` (size, race byte, the race's held
offset) and a flip. Rules: [`docs/PLAYER.md`](../../docs/PLAYER.md) (Held
items and their grips).

## Fighters and their condition

`fighter`: a `Fighter` (health bars, natural armour and armour layers,
readiness, whether it bleeds, its spawn stats, its record's perks, of
which Remise and Feint change its swings) and `Health` (yellow, red,
maximum, the regeneration rate and the heal pool; also re-exported from
`damage`). `Health::take` is the original's damage call: a `Damage`
(yellow, red, `vital`; `Damage::blow` for a weapon blow, always vital),
the victim's rule bits and state flags and whether the game plays the
story (`StoryMode`, a resource) give the rate 0, yellow −0.1·Y (−0.05·Y
with a potion), red −0.2·R with yellow scaled by the red left, or, with
no lasting wounds (rule bit 0x8), yellow −0.2·R and red untouched, in
double precision; it returns `Taken`: `Hurt`, `Weakened` (yellow under
0.05 and alive: the hit reaction writes the `Weakened` message, the
low-health check the HUD's heartbeat reads) or `Killed` (vital, yellow
under 0.01 in double precision, `failing`; in the story red also loses
0.1 × the maximum). Rules: [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md)
(Applying damage). Every fighter keeps the original's condition word and state
flags (`condition`): `Condition` with the original's values (0 normal, 1
walking, 2 attacking, 8 combat idle, 0x20 casting, 0xC0 shocked, 0x100
weak, 0x200 down, 0x300 getting up, 0x600 dormant, 0x800 collapsing, 0xA00
dead, 0x8000 kicking, 0x10000/0x10004 switching weapons) and the masks its
readers test (`helpless` 0xAC0, `fallen_or_dying` 0xA00, `off_feet` 0x200,
`dying` 0x800, `busy` 0x10203). A knock-out (`condition::KnockOut`, `condition::wake`) holds a body down
until its wake clock: down, tone and stance 0, readiness 0.4; after 1.5 s
the tone rises 0.001 a tick (×(1 + (1 − clock)²) while getting up); after
8 s the readiness rises 0.01 a tick to 1, when the tone and stance return,
the wake bits clear and a dormant body announces "Awaken". A held body
(flag 0x40) keeps its bits. The status-effect pass
(`condition::update_statuses`) updates only the owner's effects on phase
0, notices those whose update returns bit 4 and removes those returning
bit 2 by swapping the last entry in. The reset facing
(`exa_physics::body::balance::reset_heading`) is the chest over the pelvis,
flattened and normalised. The attack reach
(`CharacterStats::attack_reach`) is √scale × (1 − 0.3 age, omitted when
negative) × (1 − 0.2 encumbrance) × 80 cm.

Each living character's posture stage
(`track_condition` in `exa_physics::body::CharacterStage::Posture`, after
the daze's stance) writes it once a tick: the daze's writer, then
`posture`, the original's decision from the
clip playing, last tick's balance (0.6, 0.3 in double), summed limb
motion (25 cm) and tone (0.85), which also names the branch the original
takes next (`PostureBranch`). The get-up's start, the stagger and the
get-up clip's end (`exa_game`), the killing blow and placed corpses write
it too. `ticked_lives` tells the later stages of a character's tick that
it did not die in its posture stage. Rules:
[`docs/formats/RSQ.md`](../../docs/formats/RSQ.md) (Condition).

A killing blow on a living body starts its collapse (`collapse`):
collapsing, Fall Backward with round((1.2 − E) × 300) frames of blend from
the energy the blow leaves (`collapse_blend`), off its controller, its
lower legs out of the living lower legs' collision layer
(`exa_physics::body::pairs::set_leg_layers`); its
dying tick, the posture stage's collapsing branch (in the character's
tick, so its heading's draws follow the characters' tick order), drifts
its heading
(`jitter_heading`), starts Rebalance (20 frames, linear) once under
balance 0.8 below size 1.2 (`rebalances`, `throw_arms_wide`), takes 0.003
off its tone and 0.0003 off its pins' gain (`lose_tone`, `fade_pins`), and
at a tone of 0.25 (250 ticks) it dies there and then, limp with the motion
it has. While it is not already stepping it starts one stagger
(`stagger_step`): a balance that widens above 0.4 scales the reach by the
balance, and a lower balance scales the widened reach by 0.4. The span
squares the tone plus one half and multiplies by that length and the
stored square root of the scale. The target adds the velocity's
reciprocal-square-root unit times the span to the chest, a zero velocity
leaves the chest, and the landings and the step kind are 0. The footwork
walks that target (`Ragdoll::walk_stagger`): the stance origin is the
target pulled back to 1.1 step lengths of the frame, and one foot is
anchored on its shifted spot. The dying tick calls that walk from the
measured chest and its fast velocity when no step is running. The same
tick drops every held weapon (`release_hands`): friction 0.75, its own
mobility, a drag of that mobility times 0.175, no striker flags, and the
velocity it had. The weapon becomes its own owner, and each child copies
that owner. It leaves its parent's list. The weapon and each descendant
take the weapon as their root. Iterations are 2 plus the parts visited,
minus 1. Links are 0 and the awake byte is 0. A grip at or above 0
becomes -1. A part with a child, an object type whose low twelve bits are
0x89, and object bit 0x400000, sets its mark to -2. A solid dropped
weapon enters the solver (`free_body`), awake at 256, with its own
mobility, friction 0.75, its spheres and its velocity. One solver tick
moves it. A flat shape spawns nothing. A dropped weapon whose
sphere starts on another body's sphere meets it in the shipped body
step: one hit, and the weapon leaves the place one step alone would
leave it. A lasting wound is noticed when it times 0.75, widened,
times 0.2, is greater than 0.25 widened times 0.05. Bits 0x3daaaaab
are not noticed. The next float is. The hit path records that notice.
A body without muscles dies at
once. Rules: [`docs/formats/RSQ.md`](../../docs/formats/RSQ.md) (The
collapse).

Health regenerates (`regen`): a living character's tick keeps it
(`regenerate_red` in its upkeep stage, `keep_health` in its attack stage
after its posture and its swing routine); the health tick (`HealthTick`,
before the character ticks) runs `regenerate_health` once a fixed tick on
every other fighter (the dead, any without a living body): red
regeneration for rule bits holding 0x20
without 0x100 (`BodyRules::regenerates_red`; `Health::regenerate_red`,
1.25e-5 a tick, dead or alive), then, unless dead, `Health::bookkeep`:
the heal pool's 1.25e-5 a tick to red, an active second wind's double
0.0003 added to yellow (`heal_second_wind`; it ends once the game clock
passes the ready clock minus 180000 ms plus 3000 ms, and that tick still
adds), the yellow regeneration (the rate 0
while held, `holds_regeneration`: attacking or Cmbt - Idle playing; else
+0.0003 × `RATE_CAP` a tick to `RATE_CAP` = 0.01/83, yellow += √red ×
rate above `RED_FLOOR`) and the clamps. Every hit on a fighter sets its
rate to 0 in the hit reaction, and the collapse's death sets yellow to 0.
Rules: [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md)
(Regeneration).

A shield arm's stamina (`regen::ShieldStamina`, with its rate) refills on the body tick: while it is under 1 its rate rises
2e-6 a tick to 0.001 and the stamina rises by the rate; at 1 the rate
stops. A block restarts the rate at 0, about 6 s to full rate and 15 s
from empty to full. The body tick that calls it is the kernel lane's
(`TMotileDynamics`); the rule and its test live here.

The character's voice lines (`voice`, the part's request): a grunt
(`Voice::GRUNT_WAIT` 400 ms), a groan (500 ms) and a scream (300 ms after
the last line), each scaled by the character's volume and pitch gains; a
line with no sample plays nothing and leaves the wait alone, and a line
still on its wait is refused. The collapse asks for the groan and the
damage routine for the pain lines; nothing plays them in the game yet.

A fall to death (`fall`) is kept on every fighter (`FallDeath`: its stage,
the game-clock time it was lost, whether it has left the world) and
checked by `FallDeath::check` from a `FallReading` (the chest's slow
vertical velocity, the pelvis, the ground under the body without support,
each foot's ground; metres, compared in cm): steady at −5 cm a tick or
slower (`FALLING_BELOW`), a scream the first tick faster (not when dead;
`scream_voice` gives its gain and pitch from two draws of `Roll`, the
original's generator), lost at `LETHAL_DROP` (2500 cm) over every ground;
then the player's pit after `PIT_AFTER` (1000 ms) and death after
`PLAYER_DIES_AFTER` (4000 ms), anyone else's death after `DIES_AFTER`
(3000 ms). `FallDeath::fall_out` is the kill plane's once-only test.
`exa_game::fall` runs it in the falls branch (`condition::PostureBranch`,
which `track_condition` writes each tick). Rules:
[`docs/formats/RSQ.md`](../../docs/formats/RSQ.md) (Falling to death).

## Scope

- Hit detection from physics contacts (edge, point, blunt surfaces of a weapon).
- Momentum/energy based damage; blunt, cut and pierce through armour layers.
- Stagger, knockdown, grabs, blocking with weapon or shield.

## Sounds

`sounds` chooses a body part's hit sound and its clothing foley from the
armour layers the crate already keeps: the first layer (outer, then inner)
covering at least half the part gives its class, or the flesh/skeleton
default, or the golem's `HitGlm`; the foley sums each layer's coverage times
its bone's weight per class, picks the largest class sum times its factor,
and above 1 gives the loop's volume (the class's volume factor). The
golem bodies' `FoleyPlH` (2) and `Golem_Foley` (6, or 1 for race 0x1200)
are the body loops. `exa_audio` plays them.

## Wounds

`wounds`: a blow bleeds when the struck fighter's race bleeds
(`Fighter::bleeds`, from `races.rdb` flag bit 0 through
`Fighter::of_stats`, which also takes the fighter's health from the stats
its record spawned with, kept as `Fighter::stats`, with its encumbrance and
whether Deflection widens its armour's coverage) and its wound's size R (`Outcome::wound`, the lasting
damage on the part before the red bar's 0.75) exceeds 0.05. The wound waits
on the victim (`WoundQueue`, seven at most, a new one dropped when full)
with the striking body (`Hit::striker`), the contact point as weights in
its tetrahedron, the world point and R. Each body tick counts the waiting
wounds down; seven ticks after the blow the first is sent as a `Bleed`
(one a tick, corpses too) with how the striker's struck point moved on
since and the radius 30·R + 10 cm; `exa_character::blood` paints it. See
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Blood).

The weapon that deals a bleeding blow draws blood at the blow
(`wounds::BladeBlood` on the entity holding its model): half the wound's
R, up to 1, never washed off, even when the wound itself is dropped from a
full queue. Fists have none. `exa_character::blood` draws its stain
([`docs/RENDERING.md`](../../docs/RENDERING.md), Blood on weapons).

A bleeding blow also sprays at the blow (`wounds::Spray`, a message): one
jet on the struck part at the contact point, against the striking point's
swing, of round(0.3 × strength) droplets (ties to even) with strength 120·R,
or 200 once R exceeds 1.67 (`spray_strength`, `jet_droplets`): 36·R
droplets, 60 at most, even when the wound is dropped from a full queue.
`exa_character::spray` flies them
([`docs/RENDERING.md`](../../docs/RENDERING.md), Blood sprays).
