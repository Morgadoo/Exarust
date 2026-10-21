## ADDED Requirements

### Requirement: Mind cast gates
A Mind cast SHALL first ask for focus of 0.3 times its cost, in double precision, a NaN passing, and then a picked target; record kinds 1 and 2 SHALL be living, 4 and 5 undead, and any other SHALL refuse after what the cast already spent. An undead target SHALL take the cost (half for Control Mind) and answer 0. Instant casts SHALL add their cost to the spent focus; persistent casts SHALL hold it through their power class's reserve.

#### Scenario: Calm on the three kinds
- **WHEN** Calm costing 0.14 is cast on a living, an undead and a kind-3 target by a caster with focus 1
- **THEN** it answers 1, 0 and −1, and the spent focus is 0.42
- **WHEN** the caster has focus 0.04
- **THEN** it refuses and spends nothing

### Requirement: Calm
Calm SHALL settle every contact's concern and hostility into its regard (over 10 losing 10 and giving 30, otherwise, or a NaN, all of it three times over), lower alert by 10, stress by 50 and fear by 1, each floored at 0 with a NaN kept, and set the calm factor to 1.

#### Scenario: A worried contact
- **WHEN** a contact with regard 1, concern 12 and hostility 4 is calmed
- **THEN** its concern is 2, its hostility 0 and its regard 43

### Requirement: Timed Mind effects
Enrage SHALL add 50 alert and the enraged bit with its timed class stamped on the scene clock; Confusion SHALL add 50 alert, clear every contact and the AI's level and focus, and set the confused bit; Trust SHALL place its marker toward the caster, refused on a body already loyal to it, and halve the contact toward the caster with its regard set to 30; Suspicion SHALL need the whole cost and place its marker; Fear SHALL add 20 fear up to 40 and Embolden 50 courage up to 80.

#### Scenario: Trust toward a loyal body
- **WHEN** Trust is cast on a body already bound to the caster by Loyalty
- **THEN** it refuses and spends nothing

### Requirement: Loyalty cast
Loyalty on a body bound to the caster SHALL end the binding and answer 2, and on a body bound to another caster SHALL refuse; otherwise, with a free slot and the whole cost, the target SHALL take the marker with strength `((relation + 100)·0.005)²·100` (the sum in single precision), its contact toward the caster SHALL settle with regard 30 and its AI re-arm, and the caster SHALL run Loyalty holding the cost.

#### Scenario: A relation of 20
- **WHEN** Loyalty is cast on a target whose relation to the caster is 20
- **THEN** the marker's strength is 36 and the caster's reserve holds 0.29

### Requirement: Control Mind cast
Control Mind cast while it runs SHALL end it unless it is still starting; it SHALL need a free slot (none while Spirit Form runs) and the whole cost; a target with technique bit 0x10000 SHALL resist, taking half the cost and answering −1; otherwise the binding SHALL start without spending.

#### Scenario: A resisting target
- **WHEN** Control Mind costing 0.35 is cast on a resisting target
- **THEN** it answers −1 and the spent focus is 0.175

### Requirement: Dead powers
Raise Dead and Command Dead SHALL need a collapsed body not yet raised whose character allows it; a body with no decay SHALL be revived with decay 0.2, gait scale `1.25 − decay²/4`, pace `round((1 − decay/4)·1660/83)`, most energy `1 − decay/10` and stamina `(1 − 0.3·decay)·(1 − 0.2·load)·80·build`, and its yellow and red health SHALL become 0.5 and 1. Command Dead SHALL place its marker and run the caster's power, and cast on a standing body the caster commands SHALL end the command.

#### Scenario: A revival
- **WHEN** a corpse with load 0.5 and build 1 is revived
- **THEN** its gait scale is 1.24, its pace 19 ticks, its most energy 0.98 and its stamina 67.68

### Requirement: Sense and mask casts
The instant Sense Minds SHALL last 4000 ms or the focus's share of it, rounded to even, on the real clock, and show the overlay that long, refused while the persistent one runs; the instant Mask Mind SHALL stamp its class with the scene clock and set the masked bit, refused while the persistent one is slotted or runs; a persistent cast SHALL end its running self or end the instant one beside it and start its class holding the cost.

#### Scenario: Half the focus
- **WHEN** a caster with focus 0.065 casts the instant Sense Minds costing 0.13
- **THEN** the overlay shows for 2000 ms

### Requirement: Spirit Form cast
Spirit Form SHALL need 0.05 of focus in double precision; a running form SHALL end unless its screen is still coming up; otherwise the form SHALL start with the cost as its drain, set status bit 0x100000 and ask for the spirit screen, its music at 0.1 (0.15 with the music option) and the spirit body.

#### Scenario: A second cast
- **WHEN** Spirit Form is cast twice
- **THEN** the first starts the form and the second ends it

### Requirement: Mind front end and restore
While the body runs Control Mind only Control Mind SHALL be cast, on the held body; rows SHALL be priced as the shared front end prices with the tick over 1000. A saved persistent Sense Minds, Mask Mind, Loyalty or Command Dead SHALL be rebuilt holding its whole cost, its cost reserved (three quarters with the reserve technique), and the load SHALL take three quarters of what it holds with the technique.

#### Scenario: A restored Loyalty with the technique
- **WHEN** a saved Loyalty is loaded on a body with the reserve technique
- **THEN** it holds 0.2175 against a reserve of 0.2175

### Requirement: Mind-sight tracker
At most every 4 ms the tracker SHALL walk the scene's characters but the player and the body it controls; a living or undead kind SHALL get an entry, reusing a freed slot, its values eased toward the mood's by `exp(−ln 2·dt/1000)` (a half past a second); a collapsed body SHALL fade over 5 s and an out one over 3 s from when first seen so and then be dropped; a collapsed body without an entry SHALL not be added.

#### Scenario: A body collapses
- **WHEN** a tracked body collapses at 1000 ms and the tracker runs at 3500 ms and 6001 ms
- **THEN** its fade is 0.5 and then its entry is dropped

### Requirement: Blast charge and release
A blast SHALL hold sixteen chunks on the icosahedral directions 16 cm out in the hand's frame, aimed within 45° of the facing; charging, the chunks SHALL follow the hand and grow `0.0015·swell`, their trails easing a tenth toward 9 cm back, the caster SHALL pay the drain (running out fizzles) and the charge rise five sixths of the tick; past 1 the characters within 1800 cm SHALL be alerted by `0.25·(1 − (d/1800)⁴)·(aim·dir + 3)` and the chunks released along their forward-pushed directions.

#### Scenario: A charge passing 1
- **WHEN** a blast started on a 1300 ms tick charges once
- **THEN** it is thrown, its release sound pitch lies in 0.9..1.1 and "PowerUsed" is raised

### Requirement: Blast thrown chunks
A thrown chunk SHALL grow its reach by `growth`, its radius by `swell·(0.7·|move| + 3)·0.0006` and its trail by `(0.7·|move| + 3)·0.07 + 1`, move by its last move plus `weight·(1 − charge)²·2.4` of its direction, times `retention`, spend `round(|move|²·wear)` life when worn, and push apart from chunks closer than twice its reach.

#### Scenario: The first thrown tick
- **WHEN** a released blast at charge 0.5 ticks with its chunks still
- **THEN** each chunk moves `weight·0.6·0.96` along its direction, its reach is 4.4, its radius 0.0018 and its trail head has moved on

### Requirement: Torrent feed
A torrent SHALL hold up to 24 streams of sixteen points, sixteen born 16 cm out at the blast's directions, a point living 66 ticks; charging SHALL feed each head 15 cm and the point before it 30 cm along the aim, and past a charge of 1 the torrent SHALL run.

#### Scenario: A charge passing 1
- **WHEN** a torrent started on a 1300 ms tick charges once
- **THEN** it runs, each head 9 cm on from where it was, and "PowerUsed" is raised

### Requirement: Torrent stream ticks
A running torrent SHALL spawn a head after `8 − roll(3)` ticks along a direction drawn round the aim, age every live point, pull neighbours by `0.1·(1/2 − 2500/(d² + 2500))`, keep 0.999 of each point's move, split a stream where two points lie more than 1000 m apart, and remove a dead stream other than the first by moving the record before the last into its place.

#### Scenario: A dead stream
- **WHEN** stream 5 of sixteen dies while running
- **THEN** fifteen streams remain and record 5 holds what record 14 held

### Requirement: Force limb push
A limb struck by a blast chunk moving into it SHALL take `move·f·(−k)·0.2` with `k = (−inverse/(inverse + share))·push` and `f` clamped to 0..1, and the chunk SHALL move by a quarter of its share of the contact less a twentieth of its move; a torrent point's limb impulse SHALL be `k·f·straightness` of its move plus `(contact − point)·(1 − straightness)·(−k)`.

#### Scenario: A chunk into a limb
- **WHEN** a chunk moving 10 cm hits a limb of inverse mass 1 with share 0.2 and push 0.5
- **THEN** the limb takes an impulse of 10·(0.5/1.2)·0.2 along the move

### Requirement: Force character knock
A struck character SHALL be knocked by its gathered impulse, normalised, capped at 30 and doubled, when the impulse is longer than 1, with a stun of `(1 − k)·0.08` that is 0 below 0 and 0.008 above 0.08.

#### Scenario: A heavy impulse
- **WHEN** a character of mass 80 gathers an impulse of 100 along x
- **THEN** it is knocked 60 along x with a stun of 0.078

## MODIFIED Requirements

### Requirement: Mind-sight fade
The overlay SHALL be off at state 0; shown, it SHALL hold alpha 1 and start fading out once its end has passed; both fades SHALL move alpha by the milliseconds since the tracker's last stamp over 2 s (the span signed, worked in double precision); the fade-out SHALL end below 0.001, dropping every tracked mind, and the fade-in SHALL hold at 1. A start SHALL show it for the duration (to the end of time for 0), extend a showing one, and restart the alpha unless it is fading.

#### Scenario: Shown, fading and finishing
- **WHEN** the overlay starts for 4000 ms and is drawn 1000 ms after the stamp
- **THEN** alpha is 0.5
- **WHEN** it is drawn past its end
- **THEN** it switches to fading out
- **WHEN** it fades below 0.001
- **THEN** it is off and the tracked minds are dropped

### Requirement: Mind-sight pick
The pick SHALL take a character on screen, not the player, not hidden, not collapsed unless the dead powers ask, seen by the player's view unless Sense Minds runs, within `height·zoom·0.5·40/depth` pixels of the screen centre with an unordered compare passing; the nearest share below 1 SHALL win.

#### Scenario: Inside and outside
- **WHEN** two characters lie 5 and 10 px from the centre with a 12 px radius
- **THEN** the nearer one is picked
- **WHEN** a character lies 20 px out
- **THEN** nothing is picked

### Requirement: Mind node weights
The mood SHALL give anger `a = 0.2·anger` and fear `f = 0.2·((dread + fear) − courage)`, each product in double precision; a collapsed body or `f > 1` SHALL give 1 for all three values.

#### Scenario: Marked and ordinary
- **WHEN** the body is collapsed
- **THEN** the three values are 1
- **WHEN** anger is 2 and dread and fear are 1
- **THEN** the larger value is 0.4 and the share one half

### Requirement: Mind threshold and blend
Otherwise `f` SHALL be floored at 0 and `a` clamped to 0..1, the second value their larger and the third `f/(f + a)` when the larger is above 0; the first SHALL be the larger plus 0.2, raised to 0.6 of arousal and set to 0.015 of stress when below 0.02 of it, capped at 1. An entry SHALL keep `exp(−ln 2·dt/1000)` of its old values, a half past a second.

#### Scenario: Threshold and a half
- **WHEN** the larger value is 0.4 and stress is 60
- **THEN** the first value is 0.9
- **WHEN** the step is a second
- **THEN** a half of the old values is kept

### Requirement: Mind-sight quad
The sphere SHALL sit 15 cm above the head node's height offset and the record's forward offset along the node's axes; its texture axes SHALL be the node's third and first axes made square to the view direction, blended by `1 − |dir·axis₀|` and 0.025 long, and expressed with the eye direction in the node's frame; the node's rows SHALL be scaled by the record's three scales.

#### Scenario: A level camera
- **WHEN** the camera looks along the node's third axis
- **THEN** the texture axes are 0.025 long and perpendicular and the eye direction is the negated third axis

### Requirement: Sight entry colours
A living entry SHALL take colour0 (0.1, 0.4, 0.6), colour1 (0.8, 0.6, 0) with alpha `(1 − first)·0.6 + 0.1` and colour2 from (0.4, 0, 0) toward (0.31, 0, 0.78) by its third value with alpha `(1 − second)·0.6 + 0.1`, in double precision; the overlay SHALL give colour0 its alpha less a positive entry fade and the glow `1 − fade`.

#### Scenario: Zero and full values
- **WHEN** an entry's third value is one half
- **THEN** colour2 is (0.355, 0, 0.39)
- **WHEN** its first value is 1
- **THEN** colour1's alpha is 0.1

### Requirement: Pale entry
An undead entry SHALL use (0.3, 0.3, 0.3) for colour0, (0.3, 0.3, 0.3, 1) for colour1 and (0.5, 0.5, 0.6, 0.5) for colour2.

#### Scenario: Pale palette
- **WHEN** an undead entry is coloured with alpha 0.5
- **THEN** colour0 is (0.3, 0.3, 0.3, 0.5)

### Requirement: Sight dwell
A collapsed body SHALL fade over 5000 ms and a body whose out word is 2 or more over 3000 ms, from when it was first seen so; the fade SHALL be the elapsed time over the window in double precision, and the entry SHALL be dropped once the clock passes seen plus window.

#### Scenario: Window and fade
- **WHEN** 2500 ms of a collapsed body's window have passed
- **THEN** its fade is 0.5
- **WHEN** the clock passes seen plus 5000 ms
- **THEN** its entry is dropped

## REMOVED Requirements

### Requirement: Mind mask
**Reason**: The continuous Mask Mind is a power class (`thaum::power::mind`); this duplicate step is removed.
**Migration**: See the power classes' Mask Mind requirements.

### Requirement: Mind sense
**Reason**: The continuous Sense Minds is a power class; this duplicate step is removed.
**Migration**: See the power classes' Sense Minds requirements.

### Requirement: Loyalty control
**Reason**: Loyalty's control is the power class's; this duplicate is removed.
**Migration**: See Loyalty cast and the power classes.

### Requirement: Loyalty apply and remove
**Reason**: Applying Loyalty is the cast's (Loyalty cast) and ending it the power class's.
**Migration**: See Loyalty cast.

### Requirement: Command-dead control
**Reason**: Command Dead's control is the power class's; this duplicate is removed.
**Migration**: See Dead powers and the power classes.

### Requirement: Command-dead apply and remove
**Reason**: Applying Command Dead is the cast's (Dead powers) and ending it the power class's.
**Migration**: See Dead powers.
