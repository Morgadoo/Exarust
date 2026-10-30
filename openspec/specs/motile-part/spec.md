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

### Requirement: Held item body
An equipped item whose model has a rigid root SHALL be its own solver body, tied to its holder by the grip links. These links use the limp link law (pivot softening 0.001 per cm squared), with the item and the hand each moved by their share of the mobility sum. The item SHALL never meet its holder's body or another item the same holder holds. It SHALL be freed, with friction 0.75 and its own group masks, when its holder loses its drive.

#### Scenario: Held at the grip
- **WHEN** a held item's body and its holder's body tick together for many solver ticks
- **THEN** the item stays at its grip point on the hand

#### Scenario: Holder falls
- **WHEN** the holder's body goes limp
- **THEN** the item's links stop acting and the item falls free

### Requirement: Grip link build
The grip builder SHALL make the main links from the table's points scaled by the body scale, mirrored in x and z when flipped, with the table's slack and strength 1, and 0.01 smoothing on the second link. A grip with a second hand SHALL add that hand's link, with points scaled by the character's size and the item point's y scaled by the record's second-hand factor. It SHALL also add an arm link whose hand-side point is the table's raw point. Every link left at zero smoothing SHALL take 0.001.

#### Scenario: Arm point
- **WHEN** a two-handed grip is built for a character of size 1.5
- **THEN** the arm link's hand-side point is the table's point unscaled

### Requirement: Grip mode store
The grip mode setter SHALL store the requested mode before running the grip's callback, and SHALL mark the links as relaxing only when the grip has a callback. The two-hander's half-sword turn of mode 2 into mode 1 SHALL be local to the callback.

#### Scenario: Half-sword thrust
- **WHEN** mode 2 is asked for a half-sword thrust grip
- **THEN** the links take mode 1's arrangement and the stored mode is 2

### Requirement: Weapon body impact
A striking held item's contact with a non-limb part SHALL count on the item and run the hard impact on the owner's swing. A stab or an overhead records round(5 times the square root of the impact) as its strength, with a zero stall. The item SHALL then sound only when its own sound clock is below the game clock, at volume 0.005 times the impact plus 0.3 times the square of a hundredth of the impact, at most 1. The cooldown SHALL be 200 minus round(80 times the volume) ms.

#### Scenario: Struck weapon rings
- **WHEN** a striking weapon body in an overhead is hit hard by a thrown body
- **THEN** it plays its set, records the overhead's strength and stamps its sound clock

### Requirement: Held reach
With hands bits in the held item's class, its reach SHALL be the record byte times 0.01 times the item's y extents, worked in double and rounded to single, plus 60 cm. Without them, the race's reach SHALL be used. The aim height SHALL be 0.9 times the extents, worked in double.

#### Scenario: Double product
- **WHEN** the byte is 5 and the extents are 97.1 cm
- **THEN** the reach is 64.855 cm (single arithmetic would give 64.854996)

### Requirement: Grip runtime
Each tick a held item's grip mode SHALL follow its holder's clip (a downed body 0, Idle or Walk 2, Stab 3, other clips 1), running the grip callback when it changes, and the post-change relax pass SHALL run while the links relax.

#### Scenario: Walking two-hander
- **WHEN** the holder of a two-hander plays Walk
- **THEN** the grip takes mode 2 and the arm link runs

### Requirement: Flip step
A flip SHALL start only on a request for the holder and step only while active, by (1 − (0.5 − f)²·4)·0.05 + 0.002 (the ease single, the scale and offset double). It SHALL hold while the holder attacks more than 0.49 from the middle, and land past 0.99 or below 0.01, all double compares, a NaN running on. The record SHALL then be rebuilt from the landed side.

#### Scenario: Attack near the end
- **WHEN** an attacking holder's flip stands at progress 0.99
- **THEN** the flip holds, since 0.99 in single precision is past 0.49 from the middle in double

#### Scenario: Flip lands
- **WHEN** a flip request runs to its end
- **THEN** the item is flipped, the flip is inactive and the record is rebuilt from the flipped side

### Requirement: Parry weld
A held weapon SHALL get a stiff disabled world weld at its origin, and a soft disabled weld of strength 0.05 only when its holder may parry. The soft weld point SHALL be 0.3 times the top of the item's node box, in double, or the grip itself for the weapon class word 0x30. Death SHALL leave every grip link as it is, and a dead holder SHALL keep its held items.

#### Scenario: Holder in the heavy-claws state
- **WHEN** a holder in the heavy-claws state holds a weapon
- **THEN** the weapon gets the stiff origin weld alone

#### Scenario: Corpse keeps its shield
- **WHEN** the holder of a shield dies
- **THEN** the shield's soft weld keeps its strength of 0.05 and the shield stays held

### Requirement: Base collision sound
A non-striking held item's contact SHALL ring the base collision sound when the strength S is over 0.5 and either its clock has passed or S beats its last threshold. The volume SHALL be 1.5·v with v = min(0.01·S, 1), and the pitch (0.02·m + 0.9)·(0.6·v + 0.7) for mobility m.

#### Scenario: Full strength
- **WHEN** a strength of 100 rings on a body of mobility 0.5
- **THEN** the volume is 1.5 and the pitch 1.183

### Requirement: Base collision sound clock
Under mobility 0.1 the base sound's volume SHALL take the factor 8·m + 0.2 and its pitch 7·m + 0.3 in place of 0.02·m + 0.9. The threshold SHALL become 2·S and the clock now + 220 − round-half-even(170·v) plus a spread below 100 ms, drawn only when the sound rings.

#### Scenario: Half rounds to even
- **WHEN** a strength of 5 rings
- **THEN** the cooldown before the spread is 212 ms

### Requirement: Camera kick
A hard overhead by an attacker of scale over 1.3, past 0.6 of its clip with an impact over 60 and not yet struck, SHALL kick the chase camera upward by (2000 − d)·0.001 cm when its distance d is under 1800 cm.

#### Scenario: Far away
- **WHEN** such an overhead lands 19 m from the camera
- **THEN** the camera is not kicked

### Requirement: Off-hand item
A shield SHALL be held by a back pivot at (−20, 0, 0) cm to its hand's forearm, two face pivots at (5, 0, ±15) cm to the left hand (the forearm-frame point less the left hand's rest translation) and a soft weld of strength 0.05 at its origin. A second weapon SHALL take the grip table's off-hand entry and strike from the second slot.

#### Scenario: Left-held shield
- **WHEN** a shield is held in the left hand
- **THEN** its face pivots join the left hand, not the right

### Requirement: Torch links
A held torch SHALL be joined to the hand holding it by two pivots at (0, −20, 0) and (10, 0, 0) cm and carry a soft weld of strength 0.05 at its origin.

#### Scenario: Off-hand torch
- **WHEN** the torch is held in the left hand
- **THEN** both pivots join the left hand

### Requirement: Flip command gate
A flip request SHALL be refused while the holder is switching (condition 0x10000) or when the first held item's section has a zero flipped-side kind byte, and SHALL turn only the first held item.

#### Scenario: No flipped side
- **WHEN** a flip is requested for a weapon whose flipped-side kind is 0
- **THEN** no flip starts

### Requirement: Collapse release
A collapsing holder SHALL let go of its weapons and its torch and keep its shield.

#### Scenario: Knocked down with a sword and shield
- **WHEN** the holder collapses
- **THEN** the sword is released and the shield stays

### Requirement: Limit link
A living body's link without the pivot flag SHALL move its part alone by strength·d once |d|² (cm²) exceeds the slack, capped at 10 cm, or 2 cm with the vertical gap zeroed and only in the first two iterations for a horizontal link. A parented part's skeleton links SHALL be pivot links.

#### Scenario: Shipped skeleton
- **WHEN** a character body is built from humanoid.rsq
- **THEN** none of its links is a limit link

### Requirement: Shield equip factors
A shield's equip SHALL give its holder the parry factor 0.5 + 1.5·(1 − w)², with w halved under the Mobility technique, the leak 1 − block/2 and the stamina cost equal to the leak, times 0.6666667 under Endure, all in single precision. The equip SHALL NOT turn every blow on the holder into a block.

#### Scenario: Round shield, no techniques
- **WHEN** a shield of weight 0.5 and block 0.58 is equipped by a holder with no shield techniques
- **THEN** the parry factor is 0.875 and the leak and cost are 0.71

#### Scenario: Blow on the arm
- **WHEN** a blow strikes a limb of a shield holder
- **THEN** it takes the flesh path, not the block path

### Requirement: Starting flip
A held weapon whose character starts flipped SHALL be placed half a turn about y and built from the flipped section. Every held item SHALL take the character's flip, which the main hand's item sets.

#### Scenario: Flipped second weapon
- **WHEN** the main hand's item starts flipped and a second weapon is attached
- **THEN** the second weapon is placed and built flipped too

### Requirement: Weapon switch swap
On the tick a weapon switch's condition passes from switching to switched, a holder with two held weapons in different slots SHALL trade their slots and move its primary striker with them, once. Its main and second weapon SHALL trade places too, so the moveset is chosen again from the new main weapon.

#### Scenario: Next tick
- **WHEN** the tick after the swap is still switched
- **THEN** the slots stay as swapped

#### Scenario: A two-hander and a one-hander
- **WHEN** a holder with a two-hander in the main hand and a one-hander second reaches the swap
- **THEN** the one-hander is the main weapon and the moveset choice gives the two-weapon stance

### Requirement: Held refresh
The held refresh SHALL count the held weapons only and take the first slot's weapon for the reach (start byte × 0.01 in double × the item height + 60 cm, or the race's reach without the hands bits), the aim height 0.9 × the item height and the 0x521 flag; empty hands SHALL clear the parry flag and give no measures. It SHALL weigh the aim point (0, aim height, 0) on the item body's four rest particles with four triple products times one exact reciprocal of the determinant.

#### Scenario: Weapon and shield
- **WHEN** a holder holds one sword and a shield
- **THEN** the refresh counts one held weapon

#### Scenario: Aim weights
- **WHEN** the rest particles are the origin and 10 cm along each axis, and the aim height is 9 cm
- **THEN** the weights are 0.1, 0, 0.90000004 and 0

### Requirement: Unarmed hand mobility
When the equip refresh finds no weapon class in the first held item, each hand's contact mobility SHALL be its base times 0.5, or 0.25 with heavy claws; an armed character's hands SHALL keep their base. Every attack start in the heavy-claws state SHALL set both hands to their base times 0.25, and a collapse of an unarmed character SHALL put both back at their base.

#### Scenario: Heavy claws
- **WHEN** a hand's base is 2 and the race has heavy claws
- **THEN** its contact mobility is 0.5

#### Scenario: Unarmed collapse
- **WHEN** an unarmed human collapses
- **THEN** both hands' contact mobility is their base again

### Requirement: Foot thin-plate bit
The leg routine of every living tick that is not a collapse SHALL give each lower leg's first child the level mask's thin-plate bit (4), whatever its limit link. The collapse's start and death SHALL clear that bit and the strike-chain bit on those children only.

#### Scenario: Collapse
- **WHEN** a living body's collapse starts
- **THEN** its feet meet thin plates and cannot strike, and the collapsing ticks do not give the bit back

### Requirement: Gobbler collapse slack
When a gobbler (race index 8) starts to collapse, each lower leg's femur limit link SHALL take a slack of 10 cm.

#### Scenario: Other races
- **WHEN** a human collapses
- **THEN** its femur limit links keep their slack

### Requirement: Shield body words
A held shield's body SHALL take the held and mark bits (0x20002) and a link mobility equal to its contact mobility.

#### Scenario: Contact mobility 3.5
- **WHEN** a shield whose contact mobility is 3.5 is held
- **THEN** its link mobility is 3.5

### Requirement: Revive
A revive SHALL act only on a dead character: it is knocked out at once (condition down, the wake nibble 0x30, the wake clock now), its yellow health becomes half its red, and its hands are set up unarmed again when the record's weapon slots are empty. Its body SHALL mark every limb below the pelvis muscled with its base contact mobility, give each foot the thin-plate bit and the 0x30 layer, turn the muscles on with tone and stance 0 and the living pin gain, and keep death's link-iteration count.

#### Scenario: Red 0.3
- **WHEN** a dead character with red health 0.3 is revived
- **THEN** its yellow health is 0.15 and it is down

#### Scenario: Not dead
- **WHEN** a collapsing character is revived
- **THEN** nothing changes

### Requirement: Live undead rise
Each frame, a non-player character in the scene that is dead, whose rule word masked by 0x110 is 0x10, and whose rise clock the game clock is strictly past SHALL rise when its red health widened to double does not compare under 0.01; with less red and without the race's keep-rising flag its may-rise rule SHALL clear. A rise SHALL revive it with muscles built over the race factors and stats its body had when it went limp.

#### Scenario: Due and healthy
- **WHEN** a dead body with red 0.6 and a rise clock at 10 000 ms is checked at 10 008 ms
- **THEN** it is down and knocked out with yellow 0.3, and its drive carries its own race factors and stats

#### Scenario: Not yet due
- **WHEN** the same body is checked at 9 996 ms
- **THEN** it stays dead

#### Scenario: Revived character
- **WHEN** a body rises
- **THEN** its voice volume is its body scale times the race's voice factor, empty hands are set up unarmed, an AI brain applies its roles again and its rise clock is dropped

#### Scenario: Too little red
- **WHEN** a due body has red 0.005 and no keep-rising flag
- **THEN** its may-rise rule clears and it stays dead

### Requirement: Undoing go-limp
Going limp SHALL keep the body's clip player and its character's controller; once the body's muscles are back, both SHALL be given back, the body SHALL face the flattened direction from its pelvis to its chest, and the controller SHALL stand under the pelvis.

#### Scenario: Revived body
- **WHEN** a revived body's chest lies toward +x of its pelvis
- **THEN** it faces +x and has its clips and controller again

### Requirement: Collapse drops the held items from the records
When a character's collapse starts, every hand that does not hold a shield SHALL let go of its item in the character's hands, and the character's possessions record SHALL lose those items from its out set together with their held entries in the carried list; a shield SHALL stay.

#### Scenario: Armed collapse
- **WHEN** a character holding a weapon in the main hand and a shield on the off arm starts to collapse
- **THEN** its main hand is empty, the shield stays, and on the next tick its fists are set up again

#### Scenario: Record drop
- **WHEN** a character whose record holds a sword in the main hand starts to collapse
- **THEN** the record's main hand is empty and the sword's held entry is gone from the carried list

### Requirement: Unarmed set-up on a rise
A rising character SHALL be set up unarmed exactly when both out-set hands of its possessions record are empty; a character without a record SHALL be read from its hands, where a shield or an off-hand light fills the off hand. A rise SHALL end the collapse, so a later collapse drops again.

#### Scenario: Dropped weapon
- **WHEN** a character that collapsed with a sword rises
- **THEN** it is set up unarmed

#### Scenario: Kept shield
- **WHEN** a character that collapsed with a sword and a shield rises
- **THEN** it is not set up unarmed
