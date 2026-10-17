# combat Specification

## Purpose
Resolves weapon attacks between fighters so the game's combat loop can be
played, as the base for the physically driven combat of the original.

## Requirements

### Requirement: Wielded weapons
The system SHALL hold a weapon model in a fighter's right hand so it follows the hand through every clip.

#### Scenario: Combat stance
- **WHEN** a character with `--weapon swordex01` plays the one-handed combat idle
- **THEN** the sword is held in the right fist with the blade forward and up

### Requirement: Weapon contacts
The system SHALL register at most one contact per attack and target, when the segment of a striker the attack strikes with (a held weapon's blade, or a fist) passes within a body part's volume during the attack's active window.

#### Scenario: Strike lands
- **WHEN** an opponent's attack sweeps its blade through the player's body
- **THEN** one hit on the nearest body part is reported

### Requirement: Blow damage
The system SHALL resolve each weapon contact by the original's damage model. The inputs are the weapon's stat record, where along the weapon and how fast the struck point met the part, and whether the blow was a stab. The blow passes through the struck part's two armour layers by a coverage roll, giving impact, crush, slash and pierce through padding and resistances. Part multipliers apply.

#### Scenario: Armour stops the edge
- **WHEN** a sword cut lands on a chest fully covered by plate
- **THEN** its penetration is less than a third of the same cut on bare flesh

### Requirement: Blow reactions
The system SHALL take the yellow and red health bars by a resolved blow, daze the victim, shove it on heavy impact, and start the victim's collapse when yellow health falls below 0.01 (a body without muscles goes limp at once). A blow that does not kill SHALL NOT start a reaction clip on the struck fighter; its stance changes come from its balance. A fighter struck during an attack SHALL stop that attack and return to its stance.

#### Scenario: Knock-down
- **WHEN** a fighter's yellow health falls below 0.01
- **THEN** it collapses with Fall Backward, dies 3 s later and lies limp where it fell, and it is no longer attacked

#### Scenario: Cut on the arm
- **WHEN** a bare forearm of a fighter in its stance takes a sword cut and the fighter stays up
- **THEN** it keeps playing its stance clip

#### Scenario: Struck mid-swing
- **WHEN** a fighter is cut during an attack
- **THEN** that attack deals no further hit and the fighter returns to its stance

### Requirement: Duellist AI
The system SHALL drive an opponent from the start of the fight: it enters combat in the Defence stance, faces its target, approaches to striking distance and attacks at intervals with varied strikes, using the same animation and attack paths as the player. Its stance SHALL be the Defence stance at 4 reaches or more from its target and Cmbt - Idle within, its reach 60 cm plus its weapon's sweet spot, or 50 cm unarmed.

#### Scenario: Opponent engages
- **WHEN** `exarust play arena --opponent` starts
- **THEN** the opponent closes in and attacks the player

#### Scenario: Far and near
- **WHEN** `exarust --still-opponent --at 0,0,6 play arenao1` runs, then `--at 0,0,1.7`
- **THEN** the duellist guards in the Defence stance 8 m from the player, and stands in Cmbt - Idle at 3.7 m, within four reaches of its sword

### Requirement: Characters block each other
The system SHALL keep character bodies from passing through each other.

#### Scenario: Close quarters
- **WHEN** two characters move into each other
- **THEN** their capsules stop at contact

### Requirement: Factory weapons
The system SHALL assemble weapons named by factory recipes from their component models, choosing parts, shapes and materials from the item's seeds as the original does, and keeping an item's stored choices.

#### Scenario: Worked example
- **WHEN** the sword factory `sword_lnga1` generates with seeds 0x12345678 and quality 128
- **THEN** it chooses grip `sword grp m01`, guard `sword grd 17`, blade `sword bld dl02` and pommel `sword pml 11`, the pommel at −22.14 cm

### Requirement: Weapon stats
The system SHALL derive a weapon's stats from its item's weapon section. For a factory weapon it SHALL apply the recipe's variant bonuses and deform rules to the template's bytes, then scale them by the item's condition.

#### Scenario: Longer blade
- **WHEN** a blade part's Y-scale deform grows the blade by a factor 1.2
- **THEN** the weapon's weight byte is multiplied and its balance byte divided by 1.2, rounded

### Requirement: Armour
The system SHALL give each body part two armour layers: the race's natural layer, then the worn items in order, folding the older outer layer inwards by effective coverage. Each item's protection SHALL be read as the item stores it: an item made from a template stores it scaled by its condition. With Deflection learnt, an item covering c of a part SHALL cover c + 0.1 up to half, else c + 0.1 × (1 − c) + 0.05, at most all of it.

#### Scenario: Two shirts
- **WHEN** a character wears two mail shirts covering 80 % of a part
- **THEN** the inner layer has full coverage and the first shirt's slash resistance at its effective coverage, and the second shirt is the outer layer

#### Scenario: Deflection
- **WHEN** a character with Deflection wears items covering 30 %, 75 % and 95 % of a part
- **THEN** they cover 0.4, 0.825 and all of it

#### Scenario: Stored protection
- **WHEN** a placed character wears trousers made at quality 160 and damage 160, which store padding 17 (their template's 20 scaled by that condition)
- **THEN** their layer's padding is 17 × 0.005, not scaled again

### Requirement: Moveset from held items
The system SHALL pick a fighter's moveset config from what it holds, in and out of combat mode, as the original: empty hands none (weapon class 0x80); one one-handed weapon "One Handed", two "2W Light"; a two-hander "Greatsword" for flags 0x521, "2H Light" with kind bit 0, else "Two Handed"; a polearm "Polearm"; a shield a shield moveset (plus Torch beside a light); an off-hand light "1H & Torch"; a main-hand light "One Handed" plus Torch. A clip playing SHALL go on in the new table.

#### Scenario: Unarmed player
- **WHEN** `exarust --opponent play arena` runs without `--weapon`
- **THEN** the player fights with the "Default" moveset's guard and punches, and the duellist with its sword in "One Handed"

#### Scenario: Two-handers
- **WHEN** a fighter holds a two-handed weapon whose flags are 0x521, then one with kind 1, then one with kind 2
- **THEN** it fights with "Greatsword", then "2H Light", then "Two Handed", stance class 2 and weapon class 0x20 each time

#### Scenario: Torch beside a sword
- **WHEN** the player carries the torch with `--weapon ub05ssworda` in a story level
- **THEN** the torch is in the left hand and the player moves with "1H & Torch", stance class 4

#### Scenario: Race kept
- **WHEN** an ogre's hands change from empty to a one-handed weapon
- **THEN** its base moveset stays "Ogre" and its config becomes One Handed (0x1)

### Requirement: Player attacks
The system SHALL turn the left button in combat mode into an attack request: a press requests one (Attack Mid, Down or Up at random, never the side's last pick; Attack Combo 1–3 the same way with the cursor more than 20° left of the facing, 15° for weapon class 0; Overhead Swing for a press within 300 ms of the press before; a Stab with Alt and a held weapon); holding keeps it standing, so attacks repeat; releasing drops it.

#### Scenario: Punch
- **WHEN** the unarmed player clicks in combat mode with the cursor ahead
- **THEN** it throws the "Default" right straight, the right arm out and the body twisted into it

#### Scenario: Left straight
- **WHEN** the unarmed player clicks with the cursor 25° to the left of its facing
- **THEN** it throws one of the "Default" combos, the left straight

#### Scenario: Double click
- **WHEN** the player clicks, then presses again within 300 ms and holds the button
- **THEN** the overhead swing starts once the first attack has ended and its cooldown has passed

#### Scenario: Hold to repeat
- **WHEN** the player holds the left button in combat mode
- **THEN** a main-side attack starts after each attack's cooldown, never the same pick twice in a row, until the button is released

### Requirement: Combat stance
The system SHALL keep each fighter's stance, the looping clip it returns to. The player's SHALL be Idle out of combat mode, starting Idle only when the old stance's clip plays; in combat mode Cmbt - Idle while its target (the best within 5 m and 120° of the cursor) is nearer than 3.5 m × its body scale or within 3 s of its last attack, else, or dying, Defence stance. A clip played once SHALL end in the stance; a balanced fighter SHALL switch to it from looping clips but Cmbt - Idle and walks.

#### Scenario: Guard at the fight start
- **WHEN** `exarust --opponent play arenao1` starts without `--weapon`
- **THEN** the player raises its fists in the "Default" Defence stance, and drops to Cmbt - Idle once the duellist comes within 3.5 m

#### Scenario: Target leaves
- **WHEN** the player stands in Cmbt - Idle and its target moves beyond 3.5 m
- **THEN** its stance turns to the Defence stance but it keeps playing Cmbt - Idle

#### Scenario: After an attack
- **WHEN** the player attacks with no target within 3.5 m
- **THEN** the attack ends in Cmbt - Idle, its stance stays Cmbt - Idle for 3 s after the attack's last tick, and the clip stays Cmbt - Idle after that

#### Scenario: Combat mode off in the combat idle
- **WHEN** Tab turns combat mode off while the player plays Cmbt - Idle with the Defence stance
- **THEN** its stance turns to Idle and it keeps playing Cmbt - Idle until it walks

#### Scenario: Combat mode off in the stance's clip
- **WHEN** Tab turns combat mode off while the player plays the Defence stance, or Cmbt - Idle within 3 s of an attack
- **THEN** it starts Idle at once

### Requirement: Combat readiness
The system SHALL raise a fighter's readiness by 0.004 per tick, up to 1, while its stance and its clip are both Cmbt - Idle, and SHALL reset it to 0 otherwise. A blow SHALL scale the victim's penetration, impact and stab energy by 1 − 0.5 × its readiness, after the shove is taken from the impact.

#### Scenario: Ready for 3 s
- **WHEN** a fighter that has stood in Cmbt - Idle for 3 s takes a sword cut on bare flesh
- **THEN** the cut's penetration is half what it would be on a fighter in the Defence stance, and the shove is the same

### Requirement: Weapon record
The system SHALL build a held weapon's record as the original does: from its section's bytes, its length along its model's first-child chain of rigid parts (an assembled factory weapon's top) and its holder's own encumbrance and size, giving held weight and balance, swing speed, recoil, swing shaping, stance blend, arm stiffness, sweet spot, reach, second-hand place, swing sound and AI turn factors and mass. It SHALL hold for every weapon item of the installed `objdb.rdb`.

#### Scenario: One-handed sword
- **WHEN** an average human holds a 90 cm sword of weight 50, balance 80, striking from 20 to 200
- **THEN** its swing speed is 1.125, recoil 0.42, swing shaping 0.2, blend 7, arm stiffness 0.574, sweet spot 57.6 cm and reach 117.6 cm

#### Scenario: Armour slows the swing
- **WHEN** the same sword is held with encumbrance 0.5
- **THEN** its recoil is 0.67, its swing shaping 0.275 and its blend 8

#### Scenario: Torch and flail
- **WHEN** the installed "Torch A01" and "Flail A" are measured
- **THEN** the torch is 58.3 cm long, its flame and shadow nodes not counted, and the flail 84.4 cm, its four rigid links added to the handle

#### Scenario: Held by its character
- **WHEN** a tall human (size 1.05) in armour of encumbrance 0.5 holds the same sword
- **THEN** its reach is 120.6 cm, its recoil 0.67, its swing shaping 0.275 and its blend 8

### Requirement: Unarmed records
The system SHALL give unarmed hands the original's built-in fist and claw records: the right hand's by race index, the left hand's by the race's clawed-hands word, measured across the hand's box, held like a weapon by the character's own size and encumbrance; an unarmed character's reach SHALL be its race's unarmed reach. It SHALL hold for the hands of all 18 races of the installed `races.rdb`.

#### Scenario: Human fists
- **WHEN** an average human fights unarmed
- **THEN** each fist is 17.66 cm long with impact 0.43, swing speed 1.44, recoil 0.1, blend 0 and arm stiffness 0.5, and its reach is 50 cm

#### Scenario: Harvester
- **WHEN** a Harvester fights unarmed
- **THEN** it strikes with a right fist and a left heavy claw, and its hands weigh four times their part's mass

#### Scenario: A tall human in armour
- **WHEN** a tall human (size 1.05) in armour of encumbrance 0.5 fights unarmed
- **THEN** its fists' arm stiffness is 0.476 and their recoil 0.35

### Requirement: Shield record
The system SHALL read a shield item's record from its section `0x8000` (weight, block and bulk, bytes 4 to 6 × 0.01 in double precision) and derive what it gives its holder: parry effectiveness 0.5 + 1.5·(1 − weight)², block leak 1 − block/2, stamina cost equal to the leak, and an encumbrance share of bulk/2, as changed by the Mobility and Endure techniques. It SHALL hold for the 22 shield sections of the installed `objdb.rdb`.

#### Scenario: Round shield
- **WHEN** "RoundShield R01" (bytes 50, 58, 60) is read
- **THEN** its parry effectiveness is 0.875 and its leak and stamina cost 0.71; with Endure the stamina cost is 0.473

#### Scenario: Buckler
- **WHEN** "Buckler A" (bytes 15, 50, 8) is read
- **THEN** its weight is 0.15 (not the 0.14999999 of single-precision scaling), its block 0.5 and its bulk 0.08

### Requirement: Attack executor
The system SHALL start a requested attack, the player's and the AI's, only with at least 80 % of the fighter's most energy and once its cooldown has passed since its last attack's last tick on the game clock (12 ms a tick): 400 ms with no primary striker, else round(300 × its recoil) + 300 ms. An overhead SHALL not start over a stab or an overhead, a stab over a stab, a side attack over a side attack; a stab SHALL need the weapon's long axis forward of −0.367.

#### Scenario: Fist cooldown
- **WHEN** the unarmed player holds the left button while its punch plays and after it ends
- **THEN** nothing starts while the punch plays, and the next punch starts on the first tick 330 ms or more after the punch's last tick

#### Scenario: Dazed
- **WHEN** the player's energy is 0.79 of its most 1.0 and it requests an attack
- **THEN** no attack starts until the energy is back to 0.8

#### Scenario: Sword cooldown
- **WHEN** the player holding the arena sword (recoil 0.42) holds the left button
- **THEN** its attacks start 426 ms or more after the last tick of the one before, from the first

#### Scenario: Feral attacker
- **WHEN** a fighter whose most energy is 0.9 requests an attack at energy 0.71, then at 0.72
- **THEN** no attack starts at 0.71, and one starts at 0.72

### Requirement: Fist strikers
The system SHALL give an unarmed fighter's hands strikers built from its race's fist or claw records, each a segment from the hand's origin across its box. With the fists, claws or two weapons only the motion's hand SHALL strike: the right in the main side, stab and overhead (both claws in a clawed overhead), the left in the other side; else every held weapon strikes and fists do not. A fist SHALL meet a part at its knuckles.

#### Scenario: Punch lands
- **WHEN** `exarust --still-opponent --at 0,0,-1.1 --press LMB@120 play arenao1` runs
- **THEN** the right straight lands on the duellist with the right fist, the arm fully out, and takes its yellow health

#### Scenario: Left straight strikes with the left fist
- **WHEN** the unarmed player's other-side attack drives both fists into a target
- **THEN** only the left fist strikes, 17.56 cm along its 17.66 cm

#### Scenario: Armed
- **WHEN** a fighter holding a sword attacks with its fists' strikers still on its hands
- **THEN** only the sword strikes

### Requirement: Swing pace
An attack's clock SHALL gain speed × g × 1.125 × base frames a tick, g = 1 − 0.833·m·(d + d²), d = |progress − 0.5|; a stab ×1.2, and below 40 % × (1.2·progress + 0.5); nothing under 0.8 of its most energy, (E/Emax)² under all of it. Speed, m and the recovery SHALL be the swing speed, swing shaping and stance blend of the fighter's primary striker's record (its first held item before any).

#### Scenario: Sword
- **WHEN** a sword of weight 0.3 and balance 0.6 swings
- **THEN** its clock gains 0.87 frames a tick at progress 0, 1.30 mid-swing and 0.97 at the end, and the attack ends into the stance with a blend of 14 frames

#### Scenario: Fists
- **WHEN** a fighter attacks unarmed
- **THEN** its clock gains 1.62 frames a tick throughout and the attack ends into the stance with no blend

#### Scenario: Primary claw
- **WHEN** an unarmed fighter with a right fist and a left heavy claw throws an other-side attack
- **THEN** its swing is the claw's

#### Scenario: Duel
- **WHEN** `exarust --opponent play arenao1` runs
- **THEN** each of the duellist's attacks takes about 1 s from its start to its stance, and its strikes land

#### Scenario: Feral swing
- **WHEN** a fighter whose most energy is 0.9 punches at energy 0.9, then at 0.81
- **THEN** its clock gains 1.62 frames a tick, then 0.81

### Requirement: Dazed stance
While a fighter's energy is more than 0.05 under its most (in double precision), its stance SHALL be the Defence stance every body tick, and the player's stance rule SHALL not give Cmbt - Idle. Unless it is down or attacking, its clip SHALL start the Defence stance when another clip plays, with a blend of round((1.05 − E) × 300) frames, eased smoothly. A daze SHALL be decided before a fall in the same tick, so the fall's clip wins.

#### Scenario: Struck in the combat idle
- **WHEN** the player in combat mode, in Cmbt - Idle, is dazed to energy 0.62
- **THEN** its stance turns to the Defence stance, which starts with a blend of 129 frames, eased smoothly, and does not restart while the daze lasts

#### Scenario: Wits back
- **WHEN** the dazed player's energy is back within 0.05 of its most with its target near
- **THEN** its stance turns to Cmbt - Idle and the posture switch starts it with a blend of 30 frames

#### Scenario: Down or attacking
- **WHEN** a fallen or attacking fighter is dazed
- **THEN** its stance turns to the Defence stance and its clip plays on

#### Scenario: Duel
- **WHEN** `exarust --opponent --trace t.csv play arenao1` runs until the duellist kills the player
- **THEN** after each blow the player guards while dazed, and when its energy is back its targets turn under 0.5° a tick (10–18° in one tick without the daze's start)

#### Scenario: Shoved over
- **WHEN** `exarust --shove 60 --trace t.csv play arena` dazes the player to energy 0.13 and it falls
- **THEN** the Defence stance starts (blend 276), the fall replaces it, and the get-up ends in Idle without a jump

### Requirement: Stance blends
Each stance start SHALL blend from where the body is to the stance's first key: the set-stance helper (Tab, a duellist entering combat) 30 frames, smooth; the posture switch 30 frames, linear; a duellist's stance 40 frames without a primary striker, else twice its primary's stance blend, smooth, its ease set smooth on every pass even when nothing starts.

#### Scenario: Tab
- **WHEN** Tab turns combat mode off while the player plays the Defence stance
- **THEN** Idle starts with a blend of 30 frames, eased smoothly

#### Scenario: Posture switch
- **WHEN** a balanced fighter in Idle has the Defence stance
- **THEN** the Defence stance starts with a blend of 30 frames, linear

#### Scenario: Duellist
- **WHEN** a duellist holding a sword of stance blend 7, or with no primary striker, takes its stance from its combat walk
- **THEN** the stance starts with a blend of 14 frames, or 40, eased smoothly

### Requirement: Attack starts
An attack's clip SHALL start from where the body is, eased smoothly: the main side, the stab and the overhead with no blend past the first key, the other side with round((20 − W19) × 0.6) frames, W19 the stance blend of the striker it makes primary.

#### Scenario: Left straight
- **WHEN** the unarmed player throws an other-side attack
- **THEN** it starts with a blend of 12 frames, eased smoothly

#### Scenario: Main side
- **WHEN** a fighter throws a main-side attack
- **THEN** it starts with no blend, eased smoothly
