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
The system SHALL register at most one contact per attack and target, when the blade segment passes within a body part's volume during the attack's active window.

#### Scenario: Strike lands
- **WHEN** an opponent's attack sweeps its blade through the player's body
- **THEN** one hit on the nearest body part is reported

### Requirement: Blow damage
The system SHALL resolve each weapon contact by the original's damage model. The inputs are the weapon's stat record, where along the weapon and how fast the struck point met the part, and whether the blow was a stab. The blow passes through the struck part's two armour layers by a coverage roll, giving impact, crush, slash and pierce through padding and resistances. Part multipliers apply.

#### Scenario: Armour stops the edge
- **WHEN** a sword cut lands on a chest fully covered by plate
- **THEN** its penetration is less than a third of the same cut on bare flesh

### Requirement: Blow reactions
The system SHALL take the yellow and red health bars by a resolved blow, daze the victim, shove it on heavy impact, and make the body go limp when yellow health falls below 0.01. A blow that does not kill SHALL NOT start a reaction clip on the struck fighter; its stance changes come from its balance. A fighter struck during an attack SHALL stop that attack and return to its stance.

#### Scenario: Knock-down
- **WHEN** a fighter's yellow health falls below 0.01
- **THEN** its body goes limp and falls under the blow, it stays down, and it is no longer attacked

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
The system SHALL give each body part two armour layers: the race's natural layer, then the worn items in order, folding the older outer layer inwards by effective coverage. Each item's protection is scaled by its condition.

#### Scenario: Two shirts
- **WHEN** a character wears two mail shirts covering 80 % of a part
- **THEN** the inner layer has full coverage and the first shirt's slash resistance at its effective coverage, and the second shirt is the outer layer

### Requirement: Moveset from held items
The system SHALL pick a fighter's moveset from what it holds, in and out of combat mode, as the original: empty hands the race's own moveset (weapon class 0x80); one one-handed weapon "One Handed", two "2W Light"; a two-hander "Greatsword" for flags 0x521, "2H Light" with kind bit 0, else "Two Handed"; a polearm "Polearm"; a shield a shield moveset; an off-hand light "1H & Torch"; a main-hand light "Torch". A clip playing SHALL go on in the new moveset.

#### Scenario: Unarmed player
- **WHEN** `exarust --opponent play arena` runs without `--weapon`
- **THEN** the player fights with the "Default" moveset's guard and punches, and the duellist with its sword in "One Handed"

#### Scenario: Two-handers
- **WHEN** a fighter holds a two-handed weapon whose flags are 0x521, then one with kind 1, then one with kind 2
- **THEN** it fights with "Greatsword", then "2H Light", then "Two Handed", stance class 2 and weapon class 0x20 each time

#### Scenario: Torch beside a sword
- **WHEN** the player carries the torch with `--weapon ub05ssworda` in a story level
- **THEN** the torch is in the left hand and the player moves with "1H & Torch", stance class 4

### Requirement: Player attacks
The system SHALL start an attack on a left click in combat mode from the player's moveset: Attack Mid, Down or Up at random, never the one picked last; Attack Combo 1, 2 or 3 the same way when the cursor lies more than 20° left of the body's facing (15° for weapon class 0); Overhead Swing on a click within 300 ms of the one before, unless a Stab or an Overhead plays.

#### Scenario: Punch
- **WHEN** the unarmed player clicks in combat mode with the cursor ahead
- **THEN** it throws the "Default" right straight, the right arm out and the body twisted into it

#### Scenario: Left straight
- **WHEN** the unarmed player clicks with the cursor 25° to the left of its facing
- **THEN** it throws one of the "Default" combos, the left straight

#### Scenario: Double click
- **WHEN** the player clicks twice within 300 ms
- **THEN** the second click starts the overhead swing

### Requirement: Combat stance
The system SHALL keep each fighter's stance, the looping clip it returns to. The player's SHALL be Idle out of combat mode; in combat mode Cmbt - Idle while its target (the best fighter within 5 m and 120° of the cursor) is nearer than 3.5 m × its body scale or within 3 s of its last attack, else Defence stance, also while dying. A clip played once SHALL end in the stance; a balanced fighter SHALL switch to it from other looping clips but Cmbt - Idle and walks.

#### Scenario: Guard at the fight start
- **WHEN** `exarust --opponent play arenao1` starts without `--weapon`
- **THEN** the player raises its fists in the "Default" Defence stance, and drops to Cmbt - Idle once the duellist comes within 3.5 m

#### Scenario: Target leaves
- **WHEN** the player stands in Cmbt - Idle and its target moves beyond 3.5 m
- **THEN** its stance turns to the Defence stance but it keeps playing Cmbt - Idle

#### Scenario: After an attack
- **WHEN** the player attacks with no target within 3.5 m
- **THEN** the attack ends in Cmbt - Idle, its stance stays Cmbt - Idle for 3 s after the attack's last tick, and the clip stays Cmbt - Idle after that

### Requirement: Combat readiness
The system SHALL raise a fighter's readiness by 0.004 per tick, up to 1, while its stance and its clip are both Cmbt - Idle, and SHALL reset it to 0 otherwise. A blow SHALL scale the victim's penetration, impact and stab energy by 1 − 0.5 × its readiness, after the shove is taken from the impact.

#### Scenario: Ready for 3 s
- **WHEN** a fighter that has stood in Cmbt - Idle for 3 s takes a sword cut on bare flesh
- **THEN** the cut's penetration is half what it would be on a fighter in the Defence stance, and the shove is the same
