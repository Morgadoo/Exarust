## ADDED Requirements

### Requirement: Contacts between bodies
The system SHALL keep body parts out of each other, for every body: after the iterations each awake part meets the parts near it, awake or asleep, that it has not met this tick. The part with the smaller key round(100/mobility) brings its spheres against the other's, each where it is, or swept from the last tick when it moved over 0.1 cm² relative to the other part. Each hit pushes both at once; a pair keeps up to four points, spread widest over the sphere side.

#### Scenario: Two limbs meet
- **WHEN** two limbs with 10 cm spheres are thrown at each other at 2 m/s each
- **THEN** they stop at their spheres in the tick they meet, 0.6 cm in, and part again; without the contacts they pass through each other

#### Scenario: A cup on a table
- **WHEN** a light body (mobility 10, an 8 cm sphere) drops onto a heavy sleeping one (mobility 0.05, 6 cm spheres) lying on the floor
- **THEN** it wakes the heavy one, and both sleep again with the light one resting on the heavy one's spheres, 14 cm from the nearest centre

#### Scenario: Two sleepers
- **WHEN** two sleeping bodies lie against each other
- **THEN** no contact is looked for and both stay asleep

#### Scenario: Which side brings its spheres
- **WHEN** a human's pelvis (mobility 0.3) and hand (mobility 1.67) meet
- **THEN** their keys are 333 and 60, and the hand brings its spheres

### Requirement: Groups within a body
Two parts of one body SHALL skip each other when their group masks share a bit. From each root, with a bit counter from 0 wrapping at 32, a part with children SHALL take the next bit, its parent's bit and the two bits its parent was given, and a leaf its parent's bit; each child then SHALL also take its parent's two given bits. A character's two feet SHALL share the union of their masks.

#### Scenario: A human's own parts
- **WHEN** a human body of 18 parts is built
- **THEN** 13 bits are given; legs, pelvis, spine and head never meet each other; the lower body meets the humeri, forearms and hands but not the shoulders; the lumbar meets the forearms and hands, the thorax and head the hands; a shoulder, humerus or forearm meets only the other hand; the two humeri and the two forearms skip each other; the hands meet

### Requirement: Layers between bodies
Parts of two bodies SHALL skip each other when, in some channel k from 2 to 15 (membership bit 2k, exclusive bit 2k + 1), both are members and either is exclusive. A character's lower legs SHALL be in channel 4, exclusive (0x300), while it lives and in none once it collapses or goes limp; its feet in channel 2, exclusive (0x30). A character's feet SHALL never meet other parts.

#### Scenario: Channels
- **WHEN** layer masks are compared
- **THEN** 0x300 and 0x300 skip each other, 0x300 and 0 meet, 0x30 and 0x10 skip, 0x10 and 0x10 meet, 0x40 and 0xC0 skip, and bits 0 to 3 make no channel

#### Scenario: Living lower legs
- **WHEN** two living characters' lower legs and feet overlap
- **THEN** no contact is found; once one collapses or goes limp its lower legs meet the other's, and the feet still never do

### Requirement: Immediate push of a body contact
Each hit SHALL push its two parts apart at once. Each side's contact weights, negatives dropped and normalised to sum 1, give φ = 1/(largest weight). With f = 2φ_A/(φ_A + φ_B), the sphere side SHALL move by its mobility share m_A/(m_A + m_B) × (2 − f) of the push and the other side back by its share × f, each spread so its largest weight takes it whole. Each push SHALL count toward the part's 0.1 cm edge restore.

#### Scenario: A point against a centroid
- **WHEN** a hit on one particle of a pelvis meets a hand's centroid
- **THEN** f is 0.4: the pelvis's particle takes its share × 1.6 of the push, each of the hand's particles its share × 0.4 back, and the two parts' momentum is unchanged

### Requirement: Pair pass
In each of the next tick's 20 iterations, before any body's links, each kept point deeper than 0.01 cm along its normal SHALL be taken back to 0.01 cm, less the sphere side's travel relative to the other this tick across the normal: with L = (0.01 + 0.99·n_y²)·μ_A·μ_B in cm² a tick², a travel whose square is at most L SHALL lose 0.01·L of itself and a larger one all of it. A pair needing nothing SHALL be dropped for the rest of the tick.

#### Scenario: Reversed friction
- **WHEN** two limbs side by side (μ 2.5 each) keep a point 0.51 cm deep
- **THEN** it is taken back 0.5 cm by their shares; a travel of 0.2 cm across loses 0.000625 of itself and one of 0.3 cm is cancelled; with one limb on the other (n_y 1) 2.4 cm is kept and 2.6 cm cancelled

#### Scenario: Within the slop
- **WHEN** a kept point is 0.009 cm deep
- **THEN** nothing moves

#### Scenario: Dropped once satisfied
- **WHEN** a pair's parts have moved apart before a pass
- **THEN** the pair is dropped, and pushed back into each other later in the tick its parts are not corrected

### Requirement: Pair friction
In each of the 8 friction passes, before the level friction, each point of the pairs found this tick SHALL have its slip across its normal, relative to the other side this tick, cancelled under L = (0.01 + 0.99·n_y²)·μ_A·μ_B and kept less 0.01·L of itself over it, each side moving toward the other's slip by its mobility share over its weights. Pairs whose μ_A·μ_B is under 1.19·10⁻⁷ SHALL NOT rub.

#### Scenario: Stick and slide
- **WHEN** a limb slipped 0.2 cm across a pair's point with another limb side by side
- **THEN** the slip is cancelled; a slip of 0.3 cm loses only 0.000625 of itself

#### Scenario: Almost no friction
- **WHEN** a limb lying on another with a friction product of 1.1·10⁻⁷ slipped 3·10⁻⁴ cm across their point
- **THEN** the slip is kept; with a product of 1.2·10⁻⁷ it is cancelled

### Requirement: Pair trace
`exarust --pair-trace <file.csv>` SHALL write, after every tick, one row per pair of body parts its body–body detection found in contact: the tick; each side's body, bone and whether it lives, the side that brought its spheres first; whether the two are one body; the hits; the pushes summed and the longest, and the mean touched point, in cm.

#### Scenario: A row
- **WHEN** a contact between a living hand and a corpse's pelvis is written
- **THEN** its row names both bodies and bones, marks them as two bodies, and gives the pushes and the point in cm

## MODIFIED Requirements

### Requirement: World tick order
The system SHALL run each 83 Hz tick of the bodies as one world, in the original's order: a pre-step; every living body's character tick, the player's last, each body's whole tick before the next; the queued blows; the integration of every awake body; 20 iterations, each the pair pass then every body's links; body–body, then body–level detection; 8 friction passes, each the pairs' then the level's; the post-step of every body; then the per-character post-step.

#### Scenario: Character ticks first
- **WHEN** a tick runs with the player and other living bodies
- **THEN** every living body's targets are set before any body is integrated, the player's after every other body's

#### Scenario: One body alone
- **WHEN** a single body that touches no body part, its own or another's, ticks in the world
- **THEN** it ends the tick exactly where ticking it alone leaves it

#### Scenario: The tick's sets
- **WHEN** a tick runs
- **THEN** the pre-step runs first, then the character ticks, the solver and the per-character post-step

### Requirement: Sleeping limp bodies
The system SHALL sleep a limp body by the original's countdown: 256 when woken, then each tick, by its fastest part's last motion, back to 256 over 40 cm/s, held at 30–32 between 20 and 40 cm/s, two off under 20 cm/s; at 1 it sleeps. Its first tick asleep SHALL take its velocity away; later ticks do nothing. A living body SHALL never sleep. Blows and body contacts (the lower countdown takes the higher) SHALL wake a body without its contacts; going limp SHALL wake it; level contacts SHALL NOT.

#### Scenario: Corpse at rest
- **WHEN** a limp body's parts have moved under 20 cm/s in every tick for 128 ticks since it was woken
- **THEN** it falls asleep in that tick and is not simulated until a blow lands on it

#### Scenario: Woken corpse
- **WHEN** a sleeping corpse on the floor is shoved
- **THEN** it wakes with none of its earlier contacts and is tested afresh where it lies

#### Scenario: Creeping body
- **WHEN** a limp body's fastest part keeps moving between 20 and 40 cm/s
- **THEN** it never falls asleep

#### Scenario: Deep sleep
- **WHEN** a body has slept through one tick
- **THEN** it has no velocity and its particles stay exactly where they are in every later tick

#### Scenario: Living body
- **WHEN** a living body stands, lies fallen or gets up for any time
- **THEN** it stays awake, and it sleeps by its speed only once it goes limp

#### Scenario: Touched by a falling body
- **WHEN** a body falls onto a sleeping one
- **THEN** the sleeper wakes with the falling body's countdown and none of its level contacts
