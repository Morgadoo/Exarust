## MODIFIED Requirements

### Requirement: Velocity retention
A limp part SHALL keep its velocity as the original's default environment gives it: 0.426, 0.25 or 0.05 per second (0.98977, 0.98344 or 0.96455 per tick) when the farthest of its particles moved over 40/83 cm, over 20/83 cm, or less in the last tick. A living body's limbs SHALL keep the fast retention times the living factor, whatever their speed.

#### Scenario: Limp bands
- **WHEN** a limp part's farthest particle moved 0.49 cm, exactly 40/83 cm, or exactly 20/83 cm in the last tick
- **THEN** it keeps 0.98977, 0.98344 or 0.96455 of its velocity this tick

#### Scenario: Living limbs
- **WHEN** a living human body's part drifts at 6 cm/s
- **THEN** it keeps 0.98977 of its velocity per tick

### Requirement: Arm stiffness
Each tick, unless shocked (which leaves the arms as they are), the arms' stiffness SHALL be, first match: 0.2 collapsing; 0.5 off balance (balance under 0.6, unsupported, falling, lying or getting up); an attack's own value per arm; 0.2 at tone under 0.85; unchanged kicking; the parry blend; 0.3 casting; the walking arms (0.2 unless feral) in the walk and run, a shield's off arm twice that; 0.4 switching; else the relaxed blend.

#### Scenario: Idle human arms
- **WHEN** an alert human stands still
- **THEN** its arms settle at stiffness 0.4 and pull with σ = 0.25, their relaxation capped at 0.36

#### Scenario: Walking and off balance
- **WHEN** the human walks, and later falls
- **THEN** its arms pull with σ = 0.175 walking and 0.3125 off balance

#### Scenario: A shield doubles the off arm
- **WHEN** a shielded fighter walks at full tone
- **THEN** its off arm's stiffness is twice its walking arms and its weapon arm keeps them

#### Scenario: Zombie arms
- **WHEN** a fully feral human walks, and is then dazed to tone 0.8
- **THEN** its arms' stiffness is 0.05 walking and 0.2 dazed

#### Scenario: Collapsing arms
- **WHEN** a body collapses, off balance, at tone 0.6
- **THEN** its arms' stiffness is 0.2

#### Scenario: Casting and switching
- **WHEN** a standing body with arms at 0.8 casts, and another switches weapons
- **THEN** their arms' stiffness is 0.3 and 0.4, each arm saturating at 0.1

#### Scenario: A shock leaves the arms
- **WHEN** a shocked body's arms were 0.6 the tick before
- **THEN** their stiffness, cap and saturation stay as they were

### Requirement: Parry arm blend
While a fighter parries (by the flag the previous tick left), and the body is not shocked, collapsing, off balance, attacking, at tone under 0.85 or kicking, each stored arm stiffness SHALL ease by 0.8x + 0.04 in double precision and then take the arm floor. A power, a walk and a weapon switch SHALL not replace that ease.

#### Scenario: A parry eases toward a fifth
- **WHEN** a standing fighter parries with its arms at 0 and at 0.5
- **THEN** those stiffnesses are 0.04 and 0.44

#### Scenario: Off balance keeps its arms
- **WHEN** a parrying fighter is off balance
- **THEN** its arms' stiffness is 0.5

#### Scenario: A daze wins
- **WHEN** a parrying fighter at tone 0.8 also strides and casts
- **THEN** its arms' stiffness is 0.2

### Requirement: Support
Each tick each leg of a living body SHALL query the ground 5 cm ahead of its ankle from 70 cm above it (from it, down or getting up), walking with the gait probe, else with a sphere of 5 cm + 10 cm × an anchored foot's share. Ground over half the leg under the pelvis is out of reach, but getting up. S SHALL be grounded × balance; a standing body without a grounded foot loses its balance unless its controller stands on the ground.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** every row has the player's support and balance at 1 and its counter at 80

#### Scenario: No ground
- **WHEN** a standing body has no ground under its feet
- **THEN** its balance and support are 0 and it counts the ticks it is airborne; a moving body keeps its balance

#### Scenario: Perched
- **WHEN** a standing body's controller stands on a stone its feet hang beside
- **THEN** it keeps its balance, with support 0

#### Scenario: Out of reach
- **WHEN** a 65 cm block stands under one foot of a standing human
- **THEN** that foot has no ground, the other is grounded, and its support is 1

#### Scenario: Getting up near the ground
- **WHEN** a body getting up has no foot grounded and its pelvis 20 cm over the ground
- **THEN** its support is half its balance, which rises 0.05 a tick

#### Scenario: Down
- **WHEN** a body lying or getting up has a shelf 40 cm over its ankle, and a 5 cm step's edge 7 cm beside its probe
- **THEN** its leg finds the floor under both, where a standing body finds the shelf and the floor, and a walking body the shelf and the step's edge (2.14 cm)

#### Scenario: Walking gait probe
- **WHEN** a walking body's clip progress puts its first foot at the gait bump's peak, at scale 1 and a speed of 0
- **THEN** that foot's sphere is 20 cm and its probe is 36 cm further along the facing than the 5 cm offset

### Requirement: Lift
With support the lift SHALL be f = scale·0.075(h + L) standing, at most 1.5, L being 15, or 10 while a foot steps, else 20 dormant (a negative f lifts nothing, unless crouched past 0.9: down to −10), or f = scale·(0.4·run + 0.2)(h + 5) moving, at most 2 (none when negative), each product in double, narrowed before the clamp; h = target − pelvis height (cm). Then f −= (1 − |f|/(|f| + 2))·E²·v, v the pelvis's last vertical displacement, and f ×= 2(t − 0.5) over half tone.

#### Scenario: Balance point
- **WHEN** an alert body stands on its legs, its pelvis 1.7 cm over its target
- **THEN** f is 1 and it stays there; set down 1 cm into the floor it rises less than 2 cm over where it comes to rest

#### Scenario: Gains
- **WHEN** the pelvis is 5 cm under its target standing, at the target walking, 3.3 cm over it running
- **THEN** f is 1.5, 1 and 1; falling 1 cm a tick at f = 1 adds 2/3 × E²

#### Scenario: Tone
- **WHEN** a standing body's tone is 0.75, then 0.5
- **THEN** its lift is half, then nothing

#### Scenario: Body scale
- **WHEN** the body scale is a half, at h = 0 standing and walking, and 2 standing
- **THEN** f is 0.5625, 0.5, and 1.5

#### Scenario: Standing leads
- **WHEN** a standing body at h = 0 and scale 1 steps, is dormant, or both
- **THEN** f is 0.75, 1.5 and 0.75

#### Scenario: Get-up
- **WHEN** `exarust --trace t.csv --shove 60 play arena` gets the player up with `Get Up 2` into Idle
- **THEN** its pelvis rises to its standing height, overshoots it by less than 6 cm (10.7 cm under the old vertical hold) and is back within a centimetre in under 100 ticks

### Requirement: Target pelvis height
Each leg's height SHALL be ground + crouch·60·scale + (1 − crouch)·(clip-root over its ankle + offset: sole; moving, the ankle over the walk's first key, 0.7 to the run's on the run flag, or the run's on the sprint latch). The target SHALL be the lowest foot's, blended by half (moving: min(0.5, 0.2·speed), then × min(1, (0.3·speed)²)), by 1 if the lowest is lifted or 0 if the other is (other wins), under 40 cm of that spread (less up to 60); standing, min(it, ground + sole + length) + 5 cm.

#### Scenario: Idle human
- **WHEN** a human's clip root is 91.4 cm over its ankles on flat ground, its sole 9.68 cm and its legs 85.15 cm
- **THEN** its target is capped at 99.83 cm and it stands with its pelvis 101.5 cm over the ground

#### Scenario: Step
- **WHEN** both feet stand on a 30 cm step, or the platform under a standing body is taken away
- **THEN** its target rises by 30 cm, or drops by 30 cm and its pelvis settles 30 cm lower without losing its balance

#### Scenario: One foot up
- **WHEN** one foot stands on a 30 cm step
- **THEN** standing the lower foot caps the target, and moving it is the mean of the two feet's, uncapped

#### Scenario: Blend
- **WHEN** the other foot's spread is 35, 55 or 60 cm, or it has no ground
- **THEN** it is blended in by 0.5, 0.125 or not at all, or not at all

#### Scenario: No ground
- **WHEN** no foot has ground
- **THEN** the target is −1000 m and nothing lifts the body

#### Scenario: Scaled crouch
- **WHEN** a standing body of scale one half or 1.25 crouches fully, and that crouch term is under the leg's reach
- **THEN** the target is 35 cm or 80 cm over the ground

### Requirement: Leg bend
A leg SHALL bend onto its bend target. The knee SHALL come from the law of cosines on the two bones: 30° when the cosine is above 0.866, and 180° when it is below −1. The correction from the clip SHALL advance at most 0.02 rad a tick (0.2 while stepping) plus its stored rate, never past the target. The rate SHALL become half that sum, within ±0.1 rad. Muscles SHALL follow the correction on the next tick, not the tick that computed it. Lying down it SHALL release instead.

#### Scenario: Folded to 40 cm
- **WHEN** both bones are 40 cm and the hip-to-target distance is 40 cm
- **THEN** the unsmoothed knee angle is 60°

#### Scenario: Folded too sharp
- **WHEN** both bones are 40 cm and the hip-to-target distance is 20 cm
- **THEN** the unsmoothed knee angle is 30°

#### Scenario: Next tick
- **WHEN** the stored correction and its rate are 0 and the new correction is past 0.02 rad
- **THEN** the pose the muscles follow on that tick is unchanged, and the next tick's correction is 0.02 rad

#### Scenario: Lying down
- **WHEN** the body is down, dormant or dead, not getting up, and a stored knee of 0.2 rad and a hip of −0.2 rad have a zero rate
- **THEN** the next stored knee is 0.18 rad and the hip is −0.18 rad, each rate half of that 0.02 rad step, and the bend target, the step lift and the thigh's extra turn are unchanged

### Requirement: Support pin pull
In each iteration a supported living body SHALL pull its two thorax pins and its pelvis origin in order, each reading the pushes before it, then the pelvis origin again. One pull, in cm: 0.2·e·|e|/60 under 60, else 12 along e, less this tick's horizontal displacement, capped at 8, times S²·(G·(P·F)), G the pin-gain (from 0.08), F the pin factor. A gap under 1.19e-7 cm², walking, or off its feet (down, getting up, dormant or dead) pulls nothing.

#### Scenario: Pelvis origin twice
- **WHEN** a supported pelvis origin and a thorax pin are each 10 cm from their clamped targets and have not moved this tick
- **THEN** the thorax pin takes one application and the pelvis origin takes a second after the first push has moved it

#### Scenario: One application inside 60 cm
- **WHEN** a supported pin is 10 cm from its clamped target, its horizontal displacement this tick is 1 cm along that error, and the body is an alert human (S = 1, pin-gain 0.08, T = 0.5, E = 1, pin factor 1.25) that is not getting up
- **THEN** one application is (0.2·10·10/60 − 1) cm along the error, times 0.05, which is −0.0333 cm

#### Scenario: One application past 60 cm
- **WHEN** the same pin is 80 cm from its target and has not moved this tick
- **THEN** one application is 12 cm along the error, capped at 8 cm, times 0.05, which is 0.4 cm

#### Scenario: A tiny gap
- **WHEN** a supported pin's horizontal gap is 0, then under 1.19e-7 cm², and it moved 1 cm this tick
- **THEN** one application is zero both times

#### Scenario: Walking or off its feet
- **WHEN** a supported body with its chest 10 cm off the feet centre is walking, then off its feet while getting up, then walking in a shock stagger
- **THEN** the pins do not move it

#### Scenario: Collapsing
- **WHEN** that body is collapsing instead, and not walking
- **THEN** one application is the standing pull

#### Scenario: Chest returns
- **WHEN** a supported standing body's chest is held about 10 cm off the feet centre
- **THEN** later ticks of the real step bring the chest back toward the feet centre

### Requirement: Stand-still score
An anchored foot's score base SHALL be 11.5 while the step clock is under the later stand-still stamp plus 100 ms, else 3.5 under the earlier plus 300 ms, else 2 under the later plus 600 ms, else 1.5; each bound strict, and an unwritten or closed stamp opens no band.

#### Scenario: Just planted
- **WHEN** the stamps are T and T plus 100 ms
- **THEN** an anchored foot's base is 11.5 at T plus 100 ms, 3.5 at T plus 250 ms, 2 at T plus 300 ms and 1.5 at T plus 700 ms

#### Scenario: Closed stamps
- **WHEN** the later stamp is closed, and then both are
- **THEN** the base is 3.5 at T plus 100 ms, and then 1.5

### Requirement: Pair impact
A body pair SHALL keep the greatest squared relative travel of its hits, measured before each push; when it passes 1 between two different trees its impact SHALL be that times the negated cosine of the travel against the summed push, the cosine being 0 when the product of their squared lengths is at most 1e-8. Over 0.5 it SHALL strike each side with the impact over 2·m + 1 of the other side's contact mobility m.

#### Scenario: Two limbs meeting fast
- **WHEN** two bodies' limbs meet head-on at speed
- **THEN** each side is struck with the impact scaled by the other's mobility

#### Scenario: A degenerate push
- **WHEN** a 3 cm travel meets a push of 1e-7 m, or of 0.01 m, straight against it
- **THEN** the impact is 0, or 9

## ADDED Requirements

### Requirement: Living retention
With u = (1 − balance)(1 − r) + r, r the race's limb damping ratio (1 for a human), the living factor SHALL be 0.5·u + 0.5 while walking or off its feet (0x200); else u in combat, or 0.97·u² in double when not attacking and the body scale is over 1.3; else u². A dormant body's fast retention SHALL first take the double 0.95.

#### Scenario: A golem in combat
- **WHEN** a golem (r 0.95) at balance 0.5 stands in combat at scale 1, at scale 1.4, attacking at 1.4, or walking at 1.4
- **THEN** the factor is 0.975, 0.9221063, 0.975 and 0.9875

### Requirement: Arm relaxed blend, floor and cap
The relaxed blend SHALL be x ← 0.8x + 0.08 while stepping, else 0.8x + 0.08·(0.5 + 0.5·race arm relaxation). Every written stiffness SHALL take the floor min(0.4·|turn left in rad|, 0.4) in double. An arm part's relaxation cap SHALL be 0.6 − (x − 0.1)·0.8 in double, and its saturation 0.1 − 0.08·drive, the drive zeroed each tick and written only by an attack.

#### Scenario: Stepping and the floor
- **WHEN** a half-relaxing race's arms at 0.5 blend stepping, then not; and an arm at 0.1 has 2 rad of turn left
- **THEN** they are 0.48 and 0.46; the floored arm is 0.4

#### Scenario: The cap in double
- **WHEN** an arm's stiffness is 0.8
- **THEN** its relaxation cap is 0.03999999

### Requirement: Gait probe sphere and sway
Walking, a foot's ground probe SHALL be a sphere of 10 + 10·(2 − s)·s cm, s the gait bump of its clip progress, swayed s·(36·scale + 8·speed) cm along the facing (against it backing), speed being the body's cm a tick along the heading, not its drive.

#### Scenario: At the bump's peak
- **WHEN** a walking body's first foot is at the gait bump's peak, at scale 1 and 1.5 cm a tick, forward or backing
- **THEN** that foot's sphere is 20 cm and its sway 48 cm, or −48 cm

#### Scenario: Speed, not drive
- **WHEN** a walk at 7.5 cm a tick has a 5 cm drive vector
- **THEN** the probe reads the speed 7.5

### Requirement: Lying leg release
While lying down (down, dormant or dead) and not getting up, the stored knee, hip and foot pitch SHALL each ease toward 0 by at most 0.02 rad a tick plus its rate, and the bend target, the step lift and the thigh's extra turn SHALL stay as they were.

#### Scenario: The foot pitch is released too
- **WHEN** a lying body, not getting up, has a stored foot pitch of 0.2 rad with a zero rate
- **THEN** the next stored pitch is 0.18 rad, as the knee and hip ease

### Requirement: Support pin power
The pins' power P SHALL be (√E·T)·((E·E)·(E·E)), or 1 in a shock stagger (a NaN root stays).

#### Scenario: Grouped products
- **WHEN** E = 0.7 and T = 0.8; and S = 0.77, G = 0.093, P = 0.61, F = 0.83
- **THEN** P is 0.16070566, and the scale S²·(G·(P·F)) is 0.027917228

### Requirement: Stand-still stamps
Leaving the walk SHALL write the earlier stand-still stamp as the clock the next score reads and the later as that plus 100 ms. A footwork landing SHALL write the later alone as now plus 100 ms, a stalled foot as now; a foot starting to move SHALL write the earlier as now; a recovery step's start SHALL close the later.

#### Scenario: A landing keeps the earlier stamp
- **WHEN** a foot lands at 500 ms with the earlier stamp at 120 ms
- **THEN** the later stamp is 600 ms and the earlier stays 120 ms

### Requirement: Hit event pairs
A pair whose one side is an active striker (bit 0x10000) and whose other side is hittable (a living limb or a held part, bit 0x20000), neither body being placed, SHALL take the hit event and no impact. A body's own parts SHALL take no impact from each other.

#### Scenario: A striker on a limb
- **WHEN** a held striker meets a living limb, and a limb meets a held weapon without the strike bit
- **THEN** the first takes the hit event and no impact, and the second takes the impact path

### Requirement: Dormant sleep countdown
A dormant body SHALL count its sleep down by one a tick, floored at 1, whatever its motion; a limp body keeps its speed bands.

#### Scenario: A statue
- **WHEN** a dormant body's countdown is 10, and another's is 10 while it moves at 60 cm/s
- **THEN** both are 9 after a tick

### Requirement: Death clears the feet's strikes
A body's death SHALL give each lower leg (a part with a foot contact) collision layer 0 and its foot child 0x30, clear that foot child's level-contact word and its strike-chain bit, and change no other part's class word or strike bit.

#### Scenario: Only the feet
- **WHEN** a body whose parts all carry the strike-chain bit dies
- **THEN** only the lower legs' first children lose it
