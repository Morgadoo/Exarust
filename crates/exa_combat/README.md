# `exa_combat`

Combat resolution: contacts between weapons and bodies, impact energy, armour coverage and layers, damage types and injuries.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

It already decides when a fighter may attack and which of its strikers
strike, by the original's attack executor shared by the player and the AI
(`attack`: nothing without a held striker, the fists counting as two and a
weapon as one; 80 % of the most energy, the cooldown from the last attack tick
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
halving the wound after 3 s in Cmbt - Idle or while parrying; rules in
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
pitch and muffling and each layer's ringing (`hit_sounds`). An armed bite
above 0.1 sticks the weapon (`bite::stick`); its grab (`bite::grab_of`)
pulls by the original's shares of the weapon's and the struck part's link
mobilities, and once it breaks every grab to that part is dropped with the
grab layer (`bite::BiteHold`, not live yet). A blow on a
shield or a weapon (`Guard`) meets no layer and draws no roll: a weapon's
parry deals nothing and sags the parrier (`parried`), a shield takes it on
its arm's stamina and leaks yellow (`block`, the block path with its
Synchrony, Persist and Counter rules). A shield's own equip gives its
holder the guard's factors from the shield's record and its techniques,
with a full arm (`damage::shield_guard`), kept on the holder's
`block::ShieldHold` (the body tick refills its arm, `block::refill_hold`);
the fighter's guard is not raised, since only a blow on the shield itself
is blocked. The parry factor waits for the shield parry. The first block
of a swing rings the struck item's own material collision set at 1.5 times
the blow's volume and 1.8 times its pitch (`block::ShieldRung`,
`block::shield_ring`).

Held items are struck as bodies (`block`). A held weapon's chain keeps the
attach's permanent weapon bit and mark (0x20001,
`exa_physics::grip::weapon_words`); while its striker's window is open it
also carries the strike-chain bit (0x10000, `impact::sync_held_striking`,
written only when the window opens or shuts), so its pair contacts with
hittable bodies take the pair stage's hit event instead of the impact
path. A resting held weapon is therefore a clash for another's striker,
and its own impacts ring its weapon set (the plain collision sound is a
shield's or torch's, with the item's own material set). `block::strike_held`
classifies each side of each hit event as the original's striking part
does (`part::classify`, `part::faces`): a held shield or torch is a block
(a torch blocks with the bare arm, `block::guard_for`; refused once
the swing's 0x200 bit is set), a held weapon a clash (once a swing via
0x100, set at once, unless the struck holder parries or counters the
motion with a weapon other than its primary). The last accepted contact
of each attacker in a tick becomes its `Hit` with `Struck::Shield` or
`Struck::Weapon`, waiting as `Attacking::held_pending` until the damage
routine deals it; a block on a striker under the large size keeps the
sweep off the holder (its window shuts there). The damage routine then
blocks with the holder's `ShieldHold` (parrying with it while the holder
parries; `Holding::countering` when it counters this motion), writes the
spent arm back, recoils the striker and stamps the shield body's sound
clock with the first shield ring's draw (now + 300 − draw); a clash takes
`parry::clash_branch` with the struck weapon's primary flag. Limbs stay
the blade sweep's (`Struck::Limb`), which never wounds the hand holding a
shield (the classifier's guard). Blocks and clashes are logged at info
("a block", "blow on …'s shield"), each classified contact at debug. Rules and
numbers: [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md), Damage.

The hit reaction then runs the striker's side (`SwingState::after_wound`):
the wound bits on its swing, its striker's window shut, and a cleave or a
follow-through by its size, the blow's impact and where on the weapon it
struck; a guard (for a striker under 1.4) or a body that turns blows
(`StateFlags::TURNS_BLOWS`, checked before a parry) recoils it
(`SwingState::recoil_from`), and a clash or a block marks the swing's
weapon and shield sounds once a swing (`strike_back`). The swing's first
clash rings the striking weapon (`sounds::weapon_ring`): the haft
class's shared set (`WH_HaftWoodA`, `WH_HaftMA`, `WH_HaftLA`) when the haft
took it, else its own set, at volume × share and pitch × 1.2 (double), as an
`impact::WeaponSounded`; its body's sound clock waits 200 ms less 80 a unit
of the volume (`sounds::ring_clock`), and `pain::ClashFelt` rumbles the
player's pad by half the volume when the player is either side. The swing's
first block (when no clash rang before) rings only the weapon's own set
(`sounds::block_ring`) at ((volume × share) × 2) × the striker's body scale
and pitch × 1.125 (single), stamps the clock only when a set rang, and
rumbles nothing. The swing's first clash then rings the struck weapon
(`Struck::Weapon` carries its item body; `sounds::struck_item_sound` on its
blade's record and frame): its record's class set when the blow lands
strictly below the record's split along it from the grip, else its own
set (a NaN projection too), at the blow's volume (no share) and pitch × 1.2
(double), and stamps that weapon's sound clock whether or not it rang
(logged at info: "struck weapon … rings on the clash"). A wound on flesh (a
turned one too) rings the striking weapon on every contact
(`damage::hit_sounds`): its haft class's set when the haft took it, else its
own set, at volume × share × the striker's scale × the muffling and pitch ×
the muffling; it stamps the weapon's clock, and each armour layer above the
epsilon rings its class's set from the damage routine's own table
(`sounds::LAYER_SETS`: `FlsCol`, `HitClt`, `HitLtr`, `HitChn`, `HitPlt`,
`HitHbr`, `HitGlm`) as a `sounds::LayerSounded`. Each contact's
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
standing tick eases back up, each arm on its own, instead of the
stance's recovery blend. A wound causes no damage over time: the body
tick's only damage-over-time routine is burning (`burn`, fed by the
particle body-contact pass through `Burn::add_dose`, not by a wound), and
a wound's blood is cosmetic (`wounds`).
A Force effect's interrupt (`SwingState::force_interrupt`) breaks an
attack into a full-strength stall with no drain, cleaving and recoiled,
its window shut and the stance 600 ms on. The debug barricade calls it
where the original's Force dome, barrier and barricade do, when their wall
strikes a weapon part; Exarust's held item bodies are not classified by
the force wall yet, so the call stays unexercised. The original's
50·clock attack-reach penalty has no attack reach to take it from.

## Grips

`grip`: the original's grip table (`GRIPS`, six entries by hand class:
fists, one-handed and the first of two weapons, two-handed, polearm,
claws, the off-hand item and the second of two weapons), the hand class of
a held item from its held slot and weapon class (`hand_class`, `grip`),
and each grip's hand (the left for a polearm), offset (times the
character's size; the ogre's own, unscaled), Euler angles (about the
hand's X, then Z, then Y) and links (to the hand, the second hand and the
arm, and the stab's; `spec` hands them to `exa_physics::grip`). The held
reach is `held_reach`: the item's length times a byte of its weapon record as a
hundredth plus 60 cm, or the race's reach with no hands bits; the aim
point sits `held_aim_height` above the grip. The equip/swap refresh
(`refresh_hold`, `HoldRefresh`) reports the first held item's measures,
the held count and the parry gate, and the aim point (0, aim height, 0) as
weights on the item body's four rest particles (`held_aim_weights`, the
engine's exact barycentric routine); with an item that is not a weapon
first, every held item's own point is weighted the same way
(`held_point_weights`, `copies_item_points`). The refresh's reach is an
intermediate: the equip pass it calls next clears the reach word and keeps
the longest held record's reach (the race's unarmed), which is what the AI
publishes. `exa_character::grip_runtime` keeps the refresh on the
character. `Grip::placement` places an item for a `GripHolder`
(size, race byte, the race's held offset) and a flip. The off-hand item takes entry 5 (a light, or the second of two
weapons) and a shield keeps its own record through the spawn's equip and
attach plans (`spawn::Equipped::shield_record`,
`exa_character::offhand`); the flip command's section gate lives in
`exa_formats::weapon_record::Section::can_flip`. Rules:
[`docs/PLAYER.md`](../../docs/PLAYER.md) (Held
items and their grips).

A weapon switch swaps the two held slots once past 60% of its clip
(`condition::swap_due`); `exa_character::grip_runtime::swap_on_switch`
trades the two blades' slots, the attacker's primary and the holder's
main and second weapon in `Hands`, so the moveset is chosen again from
the new main weapon.

## Weapon body impacts

A held item that is its own body (`exa_physics::grip_solve`) is struck
through the pair stage's struck side. While its owner's strike window is
open the item is a striker; the weapon branch (`impact::strike_limbs`),
for a contact with anything but a limb (the blade sweep answers a weapon
striking a body) and once the item's own sound clock is past, rings the
item's own hit set at `0.005·I + 0.3·(0.01·I)²` (volume at most
1, silent under 0.005), pitch `0.3·v + 0.7`, cooldown `200 − round(80·v)`
stamped on the item, its first-child chain and its striking weapon
ancestors, and interrupts or stalls the owner's swing by the same
`part::hard_impact` rules as an overhead or stab, recording
`round(5√impact)` on the swing (`SwingState::strike_strength`) with a
zero stall that ends at the tick clock. The
contact classifier (`part::classify`) reads the striker's state for the
kind when the other side is itself striking; the message carries it. A
weapon whose window is open no longer takes this branch against a held
item: that pair is a hit event (`block`).
A hard overhead by a big attacker past 0.6 of its clip, over impact 60,
emits a `CameraHit`; the game's chase camera measures it against the eye
(under 18 m) and takes the original's `(2000 − distance)·0.001` vertical
kick with its random offset. A non-striking item's contact takes the base
collision sound (`part::base_impact_sound`) on the item's own clock and
last-strength threshold, with its weapon hit set standing in for the
material's sound.

## Level impacts

A character's part that strikes the level makes the body's level impact
record (`exa_physics::body::LevelImpact`); `impact::level_impacts`, after
the limb strikes, reads them: the part's recent-impact counter rises, and
once the root's sound clock is past, the limb curve (a third of the
strength unless the character is down, then half) gives the volume and
pitch for an `ImpactSounded` at the character (the foley when crowded),
and a volume over 0.25 deals the same `limb_impact_damage` a body's blow
does — the taken total, yellow health and its daze and the death or
collapse path — with the ticking clock's cooldown; a hard impact on a
striking part in an overhead swing is dropped as for a body blow. A level
contact has no other body, so no `LimbImpact` is sent (the original skips
the striker's combat-state step); the original's pain voice and BadFall
branch is not reproduced.

## Fighters and their condition

`fighter::CombatScore` is the counter a body's behaviour keeps: the damage
routine adds 3 to the striker's score for every wound and 1 to the struck
body's own score for every clash or block it takes while parrying; the arena
reads it as a fighter's share of a round's experience.
At the same three points the routine tells the struck body's behaviour of
the blow, and `react_to_hits` writes `BlowNoticed` (kind 1 a clash and 2 a
block, first in their branch; kind 0 a wound, after the damage call, its
sounds and the damage-taken notice; `down` when the struck body is down,
collapsing or dead, a killing wound included); `exa_ai::struck` runs the AI's
reaction from it. A turned blow tells nothing.

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
(flag 0x40) keeps its bits. The knock-out's writes (`condition::knock_out`:
down, the wake nibble replaced by 0x30, the wake clock at now plus the
delay) and a corpse's revive (`condition::revive`: only on a dead
character, knocked out at once, yellow health half the red, the unarmed
hands set up again when the record's weapon slots are empty) are tested
rules; nothing in the running game knocks out or revives yet. A
collapse's start also runs the lower-leg pass on its feet (strike chain
and thin-plate bit off, `Ragdoll::clear_death_strikes`). The status-effect pass
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
get-up clip's end (`exa_game`) and the killing blow write it too.
`ticked_lives` tells the later stages of a character's tick that
it did not die in its posture stage. Rules:
[`docs/formats/RSQ.md`](../../docs/formats/RSQ.md) (Condition).

A killing blow on a living body starts its collapse (`collapse`):
collapsing, Fall Backward with round((1.2 − E) × 300) frames of blend from
the energy the blow leaves (`collapse_blend`), the groan its routine asks
for from the two draws it makes (`collapse_voice`: voice line 2, volume
0.15 + 0.1·second, pitch 0.8 + 0.1·first, both sums in double; sent by
`collapse_groans` as a `PainSounded` voice line on the tick `Collapse` is
inserted, so the character's voice waits and gains apply), off its
controller, its
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

The character's voice lines (`voice`, the part's request): line 1
(`Voice::GRUNT_WAIT` 400 ms), line 2 (500 ms) and line 3 (300 ms after
the last line), each scaled by the character's volume and pitch gains; a
line with no sample plays nothing and leaves the wait alone, and a line
still on its wait is refused. Lines 1, 2 and 3 play the voice's short
groan, heavy groan and grunt sets (`_GroanS`, `_GroanH`, `_GruntS`: the
voice entry's slots 1, 2 and 0, from the assembly). Every fighter carries
its voice (`CharacterVoice`: the sets, the wound set, the gains and the
clock), built by `exa_game::character_voice`. Death takes the voice
volume (the first gain) to 0 on the tick the condition becomes dead
(`condition::silence_the_dead`), as the original's death routine does, so
a corpse voices nothing.

The damage routine's pain sounds (`pain`): after the damage call, the
victim's wound sound (its own, `WndFls`, `SkelHit` or `HitGlm`) and its
pain line, a groan above a wound of 0.3 (wound sound at 0.8 of the
voice), else, when the wound passes the epsilon or the yellow 0.2, the
wound sound past a wound of 0.1 and a groan over a yellow of 0.5 or a
grunt (`pain_sounds`; a voice level under 0.01 is mute). A struck player
feels `7Y + 7L` (a big striker's blow by its speed, an overhead's at 0.4):
the camera jolts by its root past 1 and the pad rumbles by it
(`struck_player_jolt`); a striking player's pad rumbles by half the yellow
(`striking_player_rumble`). The pain sounds are live: the damage pass
writes `PainSounded` after the damage call on flesh (wound `L`, the
unweighed yellow `Y`, the yellow left as the voice gate) and `exa_audio`
plays them; the jolt and the rumble are live too: the damage pass writes
`pain::BlowFelt` (the wound, the yellow, the striker's scale, speed and
overhead swing, the blade's velocity in cm a tick as the jolt's direction)
and `exa_game`'s `feel_blows` kicks the chase camera (`kick_random`) and
raises the pad's rumble.

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

## Head tracking

`aim` is the combatant's head tracking, ticked by the behaviour: two
relaxed angles, a yaw and a pitch, chase the look-at point's direction
(yaw from the engine's planar routine, pitch the negated engine arc sine).
Each moves at most π/100 a tick — the double π/100 compared in double
precision, the float it rounds to added — and the yaw is then held within
±45° of the body's facing; the near variant floors the pitch target at the
float nearest −0.6, the far one has no floor. A look-at within 50 cm
horizontally reuses the previous direction; between 50 and 100 cm the
height is replaced by `(10000 − d²)·1e-4` before normalising. The result
is the unit direction, the point 250 cm along it (150 cm for the far
variant) and its flat direction. The direction's sine and cosine are
Rust's, not the engine's fast trig.

## Scope

- Hit detection from physics contacts (edge, point, blunt surfaces of a weapon).
- Momentum/energy based damage; blunt, cut and pierce through armour layers.
- Stagger, knockdown, grabs, blocking with weapon or shield.

## Sounds

`sounds` chooses a struck item's blow sound (`struck_item_sound`, tested:
its record's class set below the record's split along the item, else its
own sound, the clock 200 ms less round-half-even(80 · strength) on, the
pitch × 1.2 in double; a clash's struck weapon plays it) and a body part's hit
sound and its clothing foley from the armour layers the crate already keeps: the first layer (outer, then inner)
covering at least half the part gives its class, or the flesh/skeleton
default, or the golem's `HitGlm`; the foley sums each layer's coverage times
its bone's weight per class, picks the largest class sum times its factor,
and above 1 gives the loop's volume (the class's volume factor). The
golem bodies' `FoleyPlH` (2) and `Golem_Foley` (6, or 1 for race 0x1200)
are the body loops. `exa_audio` plays them.

`swing_voice` holds the swing voice's lifecycle, wired: `run_swings` keeps
each fighter's `SwingVoice` (required by `SwingState`) and sends a
`SwingVoiceCue` (start, each attack tick's volume and pitch from
`swing_sound`), `fade_swing_voices` the fade of every living body tick that
is not an attack, and `exa_audio` plays them on the first held item's
voice (an unarmed fighter has none). The voice starts silent at 0.4 × W26 on
the first attack tick in the swing's region (`SwingState::in_region`) and
again after leaving it, the second hand's tracker sets it for a one-hand
striker's other-side attacks (`voice_speed2`), the start rustles the
clothing at the foley volume × (0.02r + 0.02), pitch 0.4r + 0.8, once the
foley clock has passed (then 400 ms plus a jitter under 100,
`swing_rustle`), and a body that is not attacking fades each held voice by
0.01 and 0.0005 a tick, stopping it under 0.005 (`fade_voice`).

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
full queue. Fists have none. The weapons' blood list
(`weapon_blood::WeaponBlood`, a resource, the original's per-scene list of
spots) holds it: the weapon's first bleeding wound registers it (a clean
spot with its tip's length; `BladeBlood`'s second field keeps the 1-based
spot), the wound adds to its spot by the list's rule
(`weapon_blood::add_blood`: 1 when 1 is below it, 0 when 0 is above it; NaN
and −0 pass), and `BladeBlood` mirrors the spot's amount. A despawned
weapon releases its spot (`release_gone_blades`). The original's world
reset (`exa_world::level_lifecycle::WorldReset`: a new session, a save
load, an arena round's start or end, a scripted jump, a quit) drops the
list, and the bodies it rebuilds are clean; Exarust keeps the player and
its weapon, so `clear_at_world_reset` also cleans every weapon. A door's
level change keeps them, as the original does. `exa_character::blood`
draws its stain ([`docs/RENDERING.md`](../../docs/RENDERING.md), Blood on
weapons).

A bleeding blow also sprays at the blow (`wounds::Spray`, a message): one
jet on the struck part at the contact point, against the striking point's
swing, of round(0.3 × strength) droplets (ties to even) with strength 120·R,
or 200 once R exceeds 1.67 (`spray_strength`, `jet_droplets`): 36·R
droplets, 60 at most, even when the wound is dropped from a full queue.
`exa_character::spray` flies them
([`docs/RENDERING.md`](../../docs/RENDERING.md), Blood sprays).

## Swing body rules, grips and burn

`swing` also reproduces the swing routine's body-side rules: the
weapon-in-hand gate (unarmed, or the squared distance under (50·scale)² and
the axes' dot over 0.5; a failed gate takes 0.1 off the rate in double), the
attack arms' stiffness by stance class and swing progress, the swing's sound
pitch and volume (W25/W26, in double as the original does), the fatigue ramp
(added only while positive, capped at 1), and the start curves (smoothstep
`(3 − 2x)x²` and the fall ease `0.11x + 1 − 1/(8x + 1)`).
`strike_body` has the aim nudges the routine applies at a striker's point:
the fist's cursor push (none from a clock of 0.9), the edge alignment (the
velocity across the weapon's axis, scaled to 20 cm and combined with the
guide and push points, shared out as two opposed pushes), the overhead's
plane pull and the side swing's vertical homing (both reduced by the tip's
own motion toward the target), the two-hand group's class bit (mode 1 leaves
it off the second item and its grandparent), Break Tangle's counter and the
clinch clear (grip links assigned the disabled flag, arm bit 8 cleared,
countered motion reset, parry ended into the 0x28 stance blend unless the
condition holds the clip still). Grip link bodies are another lane's
(`exa_physics::grip`); only the drop/clinch logic is here. The attack
arms' stiffness is applied while an attack tick plays: the swing stage
stores the pair and the muscle tune writes each side's arm bones. The
gate, the sound, the fatigue ramp and the aim nudges are not called by
the running game yet.

`parry` adds the trigger (`parry_trigger`): a helpless body, the
break-tangle motion or yellow under 0.01 end a parry; otherwise the attacker
must be attacking and under 1.3 size (or facing a shield), in its 0.3–0.9
window, not cleaving, not behind and within its reach + 150 cm. Its counter
rises and falls by the chosen clip, a start blends round(recovery · 20) with
ease 2 and moves an attack motion to 4, and a changed attack motion repicks
the clip. The running fight calls the rule from each ticked fighter's
attack stage, after its swing routine, as the original's body tick does;
the posture block's arm writes and clip rate, which run before it, read
the flag the previous tick's check left, so the posture stage takes it
from the fighter ahead of the attack stage:
a fighter in the combat idle that is not parrying, whose muscle tone is at
least 0.85, whose auto-parry deadline has passed and whose hands hold
something (a fist does) checks the blow coming at it; a fighter that
parries checks it again on every tick of its own, so a parry that starts
takes its counter twice that tick; one that does not parry lets its
counter drift back toward 0, and a parry clip left playing by a blow that
ended the parry returns to the stance. A failed check ends the parry
(a helpless condition, Break Tangle, yellow health under 0.01, or no blow
passing the trigger); it returns to the Defence stance when the fighter is
dying or dazed, else the combat idle, with a 40-frame smooth blend, unless
the condition's clip stands still. A parry the fighter is not allowed to
hold (its hands empty) only drops the flag. The attacker is the nearest
attacking foe whose size, cleaving bit, progress, reach, chests and facing
pass the trigger; a swing carrying the continuation flag skips the clock
window. The picked clip, the countered motion and the last-check time are
kept; the clip starts looped with its blend and the fast ease, and the hit
path reads `Guard::Parry`, raised only while the fighter parries or still
counters a motion and never over a shield's guard. Not yet: the armed
parry (a weapon-holder does not parry: its speed and grip links come from
the weapon's handling), the explicit target (the nearest attacker stands
in for it), the Fend entry, the player's request-an-attack, which clears
the permission, the race and claw forbids of the permission, and the
previous-motion record of a continued swing (nothing sets the continuation
flag yet).

The armed parry's geometry is in place as tested functions, with nothing
calling them yet: `parry_blade_height` finds how far up the attacker's
blade the parry meets it (the chest gap less 20 cm against the reach and
the blade length), `blade_point` carries that point by the blade's
tetrahedron now and a tick ago (the fast barycentrics), `parry_point`
sweeps it (its travel's height scaled by 16 − 16·clock outside the stab
and the overhead, less its part toward the attacker's chest, both
normalised with the processor's approximate reciprocal square root) and
holds it 60 cm (80 for an overhead) short of the defender's chest, or
marks the blow passed 10 cm on, and `parry_distance` gives the ground
distance the grip's falloff reads. `armed_grip` takes the strengths'
products in double, lets a NaN distance through the falloff and rounds
the blend half to even, as the original's conversion does.

`stance::stance_by_energy` is the original's return to a stance by energy:
the Defence stance while dying or dazed (energy more than 0.05 under its
most), both compared in double, else the combat idle. The AI's guard
choice uses it.

`weapon` is the held weapon's handling: the blade lift and bend curves and
their two guide pushes (the weapon's and the forearm's), with the weapon's
balance W7 term, using `exa_physics::body::push_fraction`'s point weighting.

`burn` is fire's damage over time, worked in double as the original does:
the timer gains 0.03 of the strength (cap 20) or loses 0.1; yellow loses
timer × 0.00004 and red (timer × 0.002)², yellow is scaled by red's new share
of the red the tick began with (all of it is lost when that was near 0); a
timer over 1 crackles once the clock is 800 ms past the last draw; yellow
under 0 after the scaling collapses. `effects` holds a body's power ramp (its
pulses, the callback's end) and the cast start (the callback's answer, the
motion, the clinch end, condition 0x20).
