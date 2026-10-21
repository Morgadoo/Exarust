## MODIFIED Requirements

### Requirement: Blow damage
The system SHALL resolve each weapon contact by the original's damage routine, in its order and precision: the struck point's speed, the swing's or the stab's impact, the zone along the weapon (a haft hit adds its material's crush bonus at any on-target share), one coverage roll over the part's two armour layers, outer first, and impact, crush, slash and pierce through padding and resistances, crush and pierce raised by the engine's fast power. Part multipliers apply.

#### Scenario: Armour stops the edge
- **WHEN** a sword cut lands on a chest fully covered by plate
- **THEN** its penetration is less than a third of the same cut on bare flesh

#### Scenario: A cut on a bare chest
- **WHEN** the tests' sword (weight 50, balance 80, slash 72, 90 cm) cuts a bare chest at 20 cm a tick, 70 cm along, at 0.8 of its swing
- **THEN** its impact is 0.6746667, its penetration 0.6148849, its wound 0.4569130, its yellow 1.0286233 and its daze 0.6852875

#### Scenario: A haft's metal crushes
- **WHEN** a mace with a metal haft (crush 90, its head from 60 % of 100 cm) strikes 5 cm below its head at 20 cm a tick
- **THEN** its crush gains 0.2 of the 0.25 off target, and its penetration is 1.3150014

### Requirement: Struck swing
A fighter dazed by D (at most 0.9) while attacking SHALL have its swing stall by r = 4·D·f − (1 − D)·min(0.005·s, 1)·W17 a tick when r ≥ 0.001 (f the blow's stall factor: 0.25 over its most health, times (D′/τ′)² with the Security perk, halved under a potion, 0 for a steadfast body; s its strikers' larger tracked speed squared), added to a stall in progress up to 1; its stance SHALL return 80·P/stall ms later, at most the swing's expected rest. A spent swing (P = 0) takes no stall.

#### Scenario: Cut mid-punch
- **WHEN** a human punching (12 of 100 frames into its clip, its fist still) takes a forearm cut of daze 0.428
- **THEN** its punch plays on, stalled by 1.713 a tick, and its stance is due 47 ms later

#### Scenario: Fast swing shrugs off a light daze
- **WHEN** a swing of recoil 0.42 tracked at 30 cm a tick takes a daze of 0.05
- **THEN** it does not stall

#### Scenario: Security
- **WHEN** a fighter with Security, at readiness 0.5 and under a potion, takes the tests' cut
- **THEN** the blow's stall factor is 0.5000001

### Requirement: Guarded blow
A parry SHALL stop the blow: no wound and no health. A shield that is not parrying SHALL deal no lasting wound and yellow 0.05·(1 − stamina)·(0.4 + 0.4·I)·leak, I the impact at the hit before any armour, with leak = 1 − block/2. A guarded blow SHALL meet no armour layer and draw no coverage roll, and SHALL make a striker smaller than 1.4 recoil. An open guard SHALL keep the flesh wound.

#### Scenario: Parry
- **WHEN** the same cut that wounds bare flesh is parried
- **THEN** the wound and the yellow damage are zero, and the fighter's health is unchanged

#### Scenario: Tired shield
- **WHEN** that cut meets a shield of block 0.58 with an empty shield arm
- **THEN** the lasting wound is zero and the yellow damage is the leak, less than the flesh wound

## ADDED Requirements

### Requirement: Striker after a wound
The system SHALL, after a wound, clear the striker's released bit, set its wound bits (cleaving, recoiled, clash, block, wound) and shut its striker's window; then a striker smaller than 1.4 SHALL follow a blow by the weapon's body through with gain 1 − I/2, cleaving on from an impact at the hit of 2, and a blow by the edge with gain 0.5 when its swing is held back, else cleave on.

#### Scenario: A body blow bogs down
- **WHEN** a swing of recoil 0.5 wounds by the weapon's body at 20 cm a tick, zone 0.8, impact 1
- **THEN** its strength becomes √0.8·0.75, it stalls, and its stance returns 300 ms later

#### Scenario: A hard blow cleaves on
- **WHEN** the same swing wounds at an impact of 2
- **THEN** its strength stays 1 and it does not stall

### Requirement: Large striker after a wound
A striker of size 1.4 or more SHALL follow a blow by the weapon's body through at gain 0.5, whatever its impact, and cleave on from the edge.

#### Scenario: A large striker
- **WHEN** a striker of size 1.5 wounds by the weapon's body at an impact of 5
- **THEN** it follows through at gain 0.5 instead of cleaving on

### Requirement: Quadruped stab
The stab's rules SHALL apply to the stab motion unless the attacker's race walks on four limbs (its second flag word's bit 8): a quadruped's stab is resolved as a swing.

#### Scenario: Harvester
- **WHEN** a race's second flag word holds 0x8
- **THEN** its characters' stabs take the swing's rules, and the ogre's 0x7 does not

### Requirement: Turned blow
A blow on a body whose state flag 0x400 is set SHALL deal nothing (no health, no shove, no daze) and SHALL make the striker recoil, whatever its size.

#### Scenario: Turned
- **WHEN** the tests' cut lands on a body that turns blows
- **THEN** its health is whole, no shove is sent, and the striker's swing recoils at strength 0.5

### Requirement: Blow sag
A blow on a limb SHALL set the struck living body's blow sag to min(2·I, 1), I the impact at the hit; it fades 0.01 a tick from the next step. A blow on a shield or a weapon SHALL set none.

#### Scenario: A slow overhead
- **WHEN** the tests' sword lands an overhead at 5 cm a tick, 50 cm along
- **THEN** its sag is 0.5753399

### Requirement: Hit sounds
The system SHALL compute a blow's hit sound as the original does: volume (I + T)·0.6 + 0.025 and pitch 0.5·I + 0.5, the weapon's at volume × (½ unarmed) × the attacker's body scale × (0.6·Rc + 0.7) and pitch × the same (its haft's class for a haft hit), and each armour layer met at 1.5·volume × its ringing weight × its class's loudness, pitch 0.4·pitch + 0.7.

#### Scenario: Plate rings
- **WHEN** the tests' cut meets a plated chest
- **THEN** the plate rings at 1.5 times the volume and the flesh under it at 0.2 of that, at half its class's loudness

### Requirement: Holding pass precision
The weapon record's holding pass SHALL compute the half-sword balance, h, the arm stiffness, the swing speed, the AI swing turn and the unarmed swing sound factors in double precision where the original does, and take the long weapon's factors only over 124 cm and the weight-only h only over a balance of 1.

#### Scenario: A two-hander held in armour
- **WHEN** a two-hander of weight 37, balance 63, 110 cm (sweet spot 80) is held at encumbrance 0.3 by a holder of size 1.1
- **THEN** its arm stiffness is 0.29503599 (bits 0x3e970ef5), its swing speed 1.1445, its swing turn 0.6548 and its reach 146 cm
