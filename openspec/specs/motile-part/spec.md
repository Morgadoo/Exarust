# motile-part Specification

## Purpose
The character body part's state, impact, voice and wake rules as the original computes them: the strike window and mark, the part hit sound, the voice request, the wake and the attack reach.

## Requirements

### Requirement: Strike window and mark
A part's contact SHALL count as a strike only when both the strike window and the chain bit are set. Marking a chain SHALL set the window, the chain bit and the walk's separate mark bit; clearing SHALL drop all three, and an attack interrupt SHALL drop the chain bit.

#### Scenario: Mark and clear
- **WHEN** a part is marked as a striker and then cleared
- **THEN** it strikes after the mark with the mark bit set, and after the clear the window, chain bit and mark bit are all clear

### Requirement: Part hit sound
A part's hit sound SHALL skip an armour layer only when its coverage is below 0.5, so a NaN coverage counts as covering.

#### Scenario: NaN coverage
- **WHEN** a layer of NaN coverage is the first to cover
- **THEN** the layer's material class sound is chosen

### Requirement: Voice request
The voice request SHALL apply its two volume gains in the original's order and SHALL compute the scream wait as an unwrapped 64-bit sum.

#### Scenario: Late clock
- **WHEN** the clock plus the scream wait exceeds 32 bits
- **THEN** the wait does not wrap

### Requirement: Wake
The wake tone and readiness SHALL be summed in double precision, and the grace sums SHALL be unwrapped 64-bit sums. Waking from a knock-out SHALL notify the caller and SHALL set stance 0 in the knocked branch.

#### Scenario: Grace past 32 bits
- **WHEN** the knock-out clock plus the grace exceeds 32 bits
- **THEN** the character stays down until the unwrapped sum is reached

### Requirement: Attack reach
A character's attack reach SHALL be size × ((1 − 0.3·age) × base) in double precision with base = (1 − 0.2·encumbrance) × 80 and size the square root of the scale; a negative age SHALL use size × base, and a NaN age SHALL give a NaN reach.

#### Scenario: Human at age 0.5
- **WHEN** a human of scale 1 and no encumbrance has age 0.5
- **THEN** the reach is 1 × ((1 − 0.15) × 80) = 68 cm

### Requirement: Live limb impact
A sounded limb impact that is not dropped SHALL take damage (volume − 0.25)·m, times 1 − s while the root sinks, less the impact damage taken; a positive excess SHALL add to the taken damage, take half as yellow health (vital over 0.6 of total) and daze by half. Then, hurt or not, the root's sound clock SHALL move to 220 + rand(100) − round(170·volume) ms on, the draw made after the damage step. The taken damage SHALL fall 0.01 a tick while over 0.01, except on a dead body.

#### Scenario: A thrown ball hits an arm
- **WHEN** a fast ball strikes a fighter's limb
- **THEN** the limb takes impact damage and the root's clock waits before the next

### Requirement: Limb impact sound
A struck part's recent-impact counter SHALL rise by 2 while under 40, before any gate, and fall by 1 a tick down to 1. A limb impact past its gates SHALL sound at the character: through the character's foley, at the volume times the foley's own and pitch 0.5·volume + 0.6, when the root's counter is over 2, and nothing without a foley; else as the struck part's hit sound at the volume and pitch 0.6·volume + 0.5.

#### Scenario: A crowded root
- **WHEN** a character's root part has been struck twice within a tick and a limb is struck again
- **THEN** the impact plays the character's foley rather than the bone's hit sound

### Requirement: Limb impact gates
A struck limb of a fighter SHALL sound only when the other side is not a striking limb, the root's sound clock has passed and the impact, a third (double precision) unless the fighter is down and then halved, reaches 0.5; the volume is that times 0.05, at most 1. After the sound, a hard impact (volume over 0.25) on a part striking in an overhead swing SHALL be dropped: no damage, no clock, no draw.

#### Scenario: An overhead striker's pelvis
- **WHEN** a fast ball strikes the pelvis of a fighter in an overhead swing whose pelvis is striking
- **THEN** it sounds once at a volume over 0.25, the taken damage stays 0, the clock stays 0 and no impact message is sent

### Requirement: Held weapon part-tree mass
When a weapon is equipped, its model's rigid nodes SHALL form a part tree that is redistributed in two passes, parent before children, with the weapon's table mass as the scale; the root part's live mass word SHALL then be divided by the holder's size while its inverse word keeps the distributed value.

#### Scenario: Two parts
- **WHEN** parts of mass 3 and 1 are redistributed with scale 0.75 for a holder of size 1.25
- **THEN** their inverse words are 1 / (0.375 × 0.75) and 1 / (0.625 × 0.75), and the root's live mass is its inverse divided by 1.25

### Requirement: Weapon length walk
The armed weapon length SHALL add the tops of the weapon's node and of each first-child node down the chain, stopping at the first node that is not rigid.

#### Scenario: Plain node between rigid ones
- **WHEN** the chain is rigid 50, plain 5, rigid 20
- **THEN** the length is 50

### Requirement: Joining the world
A part without foot contacts SHALL gain level contacts when it joins the world; a part with foot contacts SHALL lose level contacts and its dynamic bit, and each of its children SHALL lose its dynamic bit only.

#### Scenario: Lower leg and foot
- **WHEN** a fresh limb with foot contacts and its child join the world
- **THEN** the leg has neither level contacts nor the dynamic bit, and the child has only its family bits

### Requirement: Recent-impact counter order
Each part's recent-impact counter SHALL fall by one (down to 1) before the tick's contacts are handled.

#### Scenario: Struck once
- **WHEN** a limb is struck once in a tick
- **THEN** its counter reads 2 after that tick

### Requirement: Leaving part's blood spot
A part that is not a limb and holds a 1-based blood spot SHALL release it and take 0 when it leaves the world; a limb SHALL keep its spot.

#### Scenario: Weapon and limb
- **WHEN** a limb and then a weapon leave holding spot 1
- **THEN** the limb keeps 1 and the weapon releases the spot and takes 0
