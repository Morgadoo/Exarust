# combat Specification

## Purpose
Resolves weapon attacks between fighters so the game's combat loop can be
played, as the base for the physically driven combat of the original.

## Requirements

### Requirement: Wielded weapons
The system SHALL hold a weapon model in the hand its grip names (the right hand, the left for a polearm) so it follows the hand through every clip, placed by the original's grip table.

#### Scenario: Combat stance
- **WHEN** a character with `--weapon swordex01` plays the one-handed combat idle
- **THEN** the sword is held in the right fist with the blade forward and up

#### Scenario: Polearm
- **WHEN** a character with `--weapon "@Halberd A1"` plays the polearm combat idle
- **THEN** the halberd is held in the left hand at the chest, its haft running down past the right hand

### Requirement: Weapon contacts
The system SHALL register at most one contact per attack and target, when the segment of a striker whose strike window is open (a held weapon's blade, or a fist when the fists are what is held) passes within a body part's volume; the blow reads the swing's clock, the clip's clock less its blend over its last key.

#### Scenario: Strike lands
- **WHEN** an opponent's attack sweeps its blade through the player's body
- **THEN** one hit on the nearest body part is reported

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

### Requirement: Blow reactions
The system SHALL take the yellow and red health bars by a resolved blow, daze the victim, shove it on heavy impact, and start the victim's collapse when yellow health falls below 0.01 (a body without muscles goes limp at once). A blow that does not kill SHALL NOT start a reaction clip on the struck fighter; its stance changes come from its balance. A fighter struck during an attack SHALL have its swing stalled by the daze (Struck swing); a killing blow ends its attack.

#### Scenario: Knock-down
- **WHEN** a fighter's yellow health falls below 0.01
- **THEN** it collapses with Fall Backward, dies 3 s later and lies limp where it fell, and it is no longer attacked

#### Scenario: Cut on the arm
- **WHEN** a bare forearm of a fighter in its stance takes a sword cut and the fighter stays up
- **THEN** it keeps playing its stance clip

#### Scenario: Struck mid-swing
- **WHEN** a fighter is cut during an attack
- **THEN** its attack's clip plays on, slowed by its stall, and returns to its stance within the swing's expected rest

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
The system SHALL derive a weapon's stats from its item's weapon section. For a factory weapon it SHALL apply the recipe's variant bonuses and deform rules to the template's bytes, then scale them by the item's condition. The haft, head taper, head radial and blade bend rules SHALL use the curve 1 + x (or 1/(1 − x)) without the deform's amount, and the flipped side's deforms SHALL change no byte.

#### Scenario: Longer blade
- **WHEN** a blade part's Y-scale deform grows the blade by a factor 1.2
- **THEN** the weapon's weight byte is multiplied and its balance byte divided by 1.2, rounded

#### Scenario: Haft with a small amount
- **WHEN** a haft's Y-scale deform of amount 0.5 has the value 0.5 and the template's weight and balance are 40 and 80
- **THEN** the weapon's weight is 44 and its balance 67

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
The system SHALL turn the left button in combat mode into an attack request: a press requests one (Attack Mid, Down or Up at random, never the side's last pick; Attack Combo 1–3 the same way with the cursor more than 20° left of the facing, 15° for weapon class 0; Overhead Swing for a press within 300 ms of the press before; a Stab with Alt and a held weapon); holding keeps it standing, so attacks repeat; releasing drops it and releases the swing in progress.

#### Scenario: Punch
- **WHEN** the unarmed player presses and holds the left button in combat mode with the cursor ahead
- **THEN** it throws the "Default" right straight, the right arm out and the body twisted into it

#### Scenario: Left straight
- **WHEN** the unarmed player presses and holds with the cursor 25° to the left of its facing
- **THEN** it throws one of the "Default" combos, the left straight

#### Scenario: Double click
- **WHEN** the player clicks, then presses again within 300 ms and holds the button
- **THEN** the first attack is released and the overhead swing starts at once, before its cooldown

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
The system SHALL start a requested attack (player or AI) only with at least 80 % of the fighter's most energy and, unless a released swing allows it (Released swing), while no swing stalls and once its cooldown has passed since its last attack tick on the game clock (12 ms a tick): 400 ms with no primary striker, else round(300 × recoil) + 300 ms. No overhead over a stab or an overhead, stab over a stab, side attack over a side attack; a stab needs the weapon's long axis forward of −0.367.

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

#### Scenario: Struck and stalling
- **WHEN** a fighter whose held swing stalls after a blow requests an attack
- **THEN** nothing starts until the stall has ended

### Requirement: Fist strikers
The system SHALL give an unarmed fighter's hands strikers built from its race's fist or claw records, each a segment from the hand's origin across its box. With the fists, claws or two weapons only the motion's hand SHALL strike: the right in the main side, stab and overhead (both claws in a clawed overhead), the left in the other side; else every held weapon strikes and fists do not. A fist SHALL meet a part at its knuckles.

#### Scenario: Punch lands
- **WHEN** `exarust --still-opponent --at 0,0,-1.1 --press LMB@120-200 play arenao1` runs (the button held)
- **THEN** the right straight lands on the duellist with the right fist, the arm fully out, and takes its yellow health

#### Scenario: Left straight strikes with the left fist
- **WHEN** the unarmed player's other-side attack drives both fists into a target
- **THEN** only the left fist strikes, 17.56 cm along its 17.66 cm

#### Scenario: Armed
- **WHEN** a fighter holding a sword attacks with its fists' strikers still on its hands
- **THEN** only the sword strikes

### Requirement: Swing pace
An attack's clock SHALL gain speed × g × 1.125 × base frames a tick, g = 1 − 0.833·m·(d + d²), d = |progress − 0.5|; a stab ×1.2, below 40 % × (1.2·progress + 0.5); a feral level f > 0 × ((1 − f)/2 + f·smoothstep(progress) + 1/2); × the swing strength while it stalls; nothing under 0.8 of its most energy, (E/Emax)² under all of it. Speed, m and the recovery SHALL be its primary striker's swing speed, swing shaping and stance blend (its first held item before any).

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

#### Scenario: Feral record
- **WHEN** a fully feral character punches
- **THEN** its clock gains 0.81 frames a tick before the clip's first key, 1.62 mid-swing and 2.43 at the end

#### Scenario: Held punch
- **WHEN** `exarust --opponent --press LMB@200-420 play arenao1` runs
- **THEN** each uninterrupted punch (110 frames) plays 67 ticks and ends into its stance on the 68th

### Requirement: Dazed stance
While a fighter's energy is more than 0.05 under its most (in double precision), its stance SHALL be the Defence stance every body tick, and the player's stance rule SHALL not give Cmbt - Idle. Unless its condition is helpless or attacking, its clip SHALL start the Defence stance when another clip plays, with a blend of round((1.05 − E) × 300) frames, eased smoothly. A daze SHALL be decided before the posture stage in the same tick, so a fall's clip wins.

#### Scenario: Struck in the combat idle
- **WHEN** the player in combat mode, in Cmbt - Idle, is dazed to energy 0.62
- **THEN** its stance turns to the Defence stance, which starts with a blend of 129 frames, eased smoothly, and does not restart while the daze lasts

#### Scenario: Wits back
- **WHEN** the dazed player's energy is back within 0.05 of its most with its target near
- **THEN** its stance turns to Cmbt - Idle and the posture switch starts it with a blend of 30 frames

#### Scenario: Down or attacking
- **WHEN** a fighter down, getting up or attacking is dazed
- **THEN** its stance turns to the Defence stance and its clip plays on

#### Scenario: Duel
- **WHEN** `exarust --opponent --trace t.csv play arenao1` runs until the duellist kills the player
- **THEN** after each blow the player guards while dazed, and when its energy is back its targets turn under 0.5° a tick (10–18° in one tick without the daze's start)

#### Scenario: Shoved over
- **WHEN** `exarust --shove 60 --trace t.csv play arena` dazes the player to energy 0.13 and it falls
- **THEN** the Defence stance starts (blend 276), Rebalance and then the fall replace it each tick until the player is down, and the get-up ends in Idle without a jump

### Requirement: Stance blends
Each stance start SHALL blend from where the body is to the stance's first key: the set-stance helper (Tab, a duellist entering combat) 30 frames, smooth; the posture switch 30 frames, linear, once a Rebalance's wait has run out; a duellist's stance 40 frames without a primary striker, else twice its primary's stance blend, smooth, its ease set smooth on every pass even when nothing starts.

#### Scenario: Tab
- **WHEN** Tab turns combat mode off while the player plays the Defence stance
- **THEN** Idle starts with a blend of 30 frames, eased smoothly

#### Scenario: Posture switch
- **WHEN** a balanced fighter in Idle has the Defence stance
- **THEN** the Defence stance starts with a blend of 30 frames, linear

#### Scenario: Duellist
- **WHEN** a duellist holding a sword of stance blend 7, or with no primary striker, takes its stance from its combat walk
- **THEN** the stance starts with a blend of 14 frames, or 40, eased smoothly

#### Scenario: After a Rebalance
- **WHEN** a balanced fighter plays a Rebalance started off balance, its wait 41
- **THEN** the posture switch starts its stance on the 41st tick after it, not before

### Requirement: Attack starts
An attack's clip SHALL start from where the body is, eased smoothly: the main side with no blend past the first key, the other side with round((20 − W19) × 0.6) frames, W19 the stance blend of the striker it makes primary; the stab and the overhead with the executor's blend (10 for Feint's, else none).

#### Scenario: Left straight
- **WHEN** the unarmed player throws an other-side attack
- **THEN** it starts with a blend of 12 frames, eased smoothly

#### Scenario: Main side
- **WHEN** a fighter throws a main-side attack
- **THEN** it starts with no blend, eased smoothly

#### Scenario: Feint's overhead
- **WHEN** Feint renews a released swing with a double click's request
- **THEN** the overhead starts with a blend of 10 frames

### Requirement: Swing release
When an attack's request drops during the attack, unless its swing recoiled, cleaved or was released already, the system SHALL release it: strength P = 1 drains by max(1 − 0.05·|v|, 0.1)/W17 × 0.03 a tick (at least 0.01, v its striker's tracked speed), the clip × P, the stance back after 1000/(80 × that) ms, at most the swing's expected rest. A released main-side attack SHALL return to the stance once that time has passed (blend 2·W19, 30 without a primary striker).

#### Scenario: Click
- **WHEN** the unarmed player clicks once in combat mode
- **THEN** its punch starts, is released the next tick (stall 0.3 a tick) and is back in its stance 5 ticks after it started

#### Scenario: Late fast swing
- **WHEN** a swing of recoil 0.42 is released at 0.6 of its clip, its striker tracked at 20 cm a tick
- **THEN** it stalls by 0.01 a tick and its stance is due at the swing's expected rest, 426 ms later for Sword A

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

### Requirement: Remise
A fighter with Remise holding a side attack that is not released, stalling or a chain past its first quarter SHALL chain the other side's attack once the swing passes 0.8 of its clip: one of that side's three clips without repeat, with round(6·W18) + 2 frames of blend, eased smoothly, its strike window carried until 0.25 of the new clip. Without Remise no swing chains.

#### Scenario: Fists
- **WHEN** `exarust --opponent --perks remise --press LMB@200-600 play arenao1` runs
- **THEN** each held main-side punch chains a left straight after 55 ticks of its 110 frames, with a blend of 2 frames

#### Scenario: Greatsword
- **WHEN** the player holding a greatsword of swing speed 1.086 and shaping 0.299 holds the button with Remise
- **THEN** its Attack Mid (110 frames) chains a combo 78 ticks after it starts, with a blend of 4 frames

### Requirement: Released swing
After a released swing, a request within 300 ms of the side attack's start SHALL start the overhead at once with no blend, whatever it asks and before any cooldown. With Feint, from (2f + 1) × (round(100·W17) + 100) ms after the release (f = smoothstep(progress/0.7), 1 past 0.7) a request SHALL start the next attack despite the stall or the cooldown: a stab or an overhead with a blend of 10, a side attack with its own.

#### Scenario: Tap and press
- **WHEN** the unarmed player taps the button, then presses again 216 ms after the punch's start and 324 ms after the first press
- **THEN** the overhead starts at once with no blend, though the cooldown has not passed

#### Scenario: Feint
- **WHEN** a fighter with Feint released a punch at its start and requests again past 12 122 ms (110 ms later) with the cooldown running to 12 630 ms
- **THEN** its next punch starts at once; without Feint it waits for the cooldown

### Requirement: Strike window
Each swing SHALL open its strike window past 0.3 of its clip (stab 0.4, overhead 0.5): with the fists, claws or two weapons only the motion's hand, which becomes the primary striker, else every held weapon. The overhead's SHALL close past 0.9 with weapon class 0, else 0.95, or at once past 0.85 once the swing falls slower than 3 cm a tick; the stab's past 0.8, or 0.95 while its thrust drives on at 2.5 cm a tick or more. Any clip start SHALL shut it.

#### Scenario: Left straight
- **WHEN** an unarmed fighter's other-side punch passes 0.3 of its clip
- **THEN** only its left fist may strike and it becomes the primary striker

#### Scenario: Stab
- **WHEN** a stab passes 0.8 of its clip with its thrust slowed under 2.5 cm a tick
- **THEN** its window shuts

### Requirement: Attack turn
While a fighter attacks, its body SHALL turn toward its heading at (1 − 0.6·enc)·(1 − 0.25·progress)·B·W16·P / scale a tick (B = 240° a second, W16 its primary striker's handling), slower over the last 0.6 rad, the step divided by √scale.

#### Scenario: Punching
- **WHEN** an unencumbered human of scale 1 starts a punch (handling 0.8) at full strength
- **THEN** its body turns at most 0.0404 rad a tick (192° a second), 0.0353 half way through

### Requirement: Held item grips
The system SHALL place every held item by the original's grip table: the second held item takes the off-hand grip, else the weapon class picks the fists', one-handed (also two weapons'), two-handed, polearm or claws grip. The item sits at the grip's offset times the character's size (an ogre's own offset, unscaled) plus the race's held offset (x away from the middle), turned by the grip's angles about the hand's X, then Z, then Y.

#### Scenario: One-handed sword
- **WHEN** a human of average height holds a one-handed sword
- **THEN** it sits 12 cm along −X, 1.5 cm down and 6 cm along +Z of the right hand, its blade along (−0.36, 0.32, 0.88) of the hand

#### Scenario: Size and race
- **WHEN** a one-handed weapon is held by a human of size 1.05, by an ogre of any size, and by the demon (size 1.5)
- **THEN** it sits at (−12.6, −1.575, 6.3) cm, at (−27.5, −7, 9) cm, and at (−27, −2.25, 9) cm of the right hand

#### Scenario: Shipped weapons
- **WHEN** every weapon item of the installed object database is held as the first item
- **THEN** 136 take the one-handed grip (the torch among them), 40 the two-handed and 32 the polearm's, and none is left without one

#### Scenario: Two-hander
- **WHEN** a character with `--weapon "sword gsf 01"` plays the greatsword combat idle
- **THEN** the blade stands upright, tip up, before the chest, as in the live capture of the original's two-handed stance

#### Scenario: Torch
- **WHEN** a human of average height carries the torch in the right hand, or in the left beside a weapon
- **THEN** it sits at (−12, −1.5, 6) cm turned (70°, −15°, 20°), or at (11, −0.5, 6) cm turned (110°, 15°, 160°), as before

### Requirement: Knockback step
A blow whose impact is over 0.5 SHALL start a knockback step of 2·(20·strikes + 10)·impact centimetres along the horizontal swing, plus 20 ticks of the chest's fast velocity, capped at the step length. Strikes SHALL be 0 until that power exists. Impact of 0.5 or less SHALL NOT start that step.

#### Scenario: A full impact
- **WHEN** impact is 1, strikes are 0, the swing is horizontal and the chest is still
- **THEN** the step is 20 cm along the swing

#### Scenario: Force strikes
- **WHEN** impact is 1 and strikes are 1, with a still chest
- **THEN** the step is 60 cm along the swing

#### Scenario: At the line
- **WHEN** impact is 0.5
- **THEN** no knockback step is started

#### Scenario: A moving chest
- **WHEN** impact is 1, strikes are 0 and the chest moves 1 cm a tick along the swing
- **THEN** the step is 40 cm along the swing

### Requirement: Guarded blow
A parry SHALL stop the blow: no wound and no health. A shield that is not parrying SHALL deal no lasting wound and yellow 0.05·(1 − stamina)·(0.4 + 0.4·I)·leak, I the impact at the hit before any armour, with leak = 1 − block/2. A guarded blow SHALL meet no armour layer and draw no coverage roll, and SHALL make a striker smaller than 1.4 recoil. An open guard SHALL keep the flesh wound.

#### Scenario: Parry
- **WHEN** the same cut that wounds bare flesh is parried
- **THEN** the wound and the yellow damage are zero, and the fighter's health is unchanged

#### Scenario: Tired shield
- **WHEN** that cut meets a shield of block 0.58 with an empty shield arm
- **THEN** the lasting wound is zero and the yellow damage is the leak, less than the flesh wound

### Requirement: A deep wound is noticed
A lasting wound SHALL be noticed when that wound times 0.75, widened, times 0.2, is greater than 0.25 widened times 0.05. Bits 0x3daaaaab are not noticed. The next float is. A non-number is not. The hit path records the notice for a chest cut.

#### Scenario: The next float after the line is noticed
- **WHEN** a lasting wound of bits 0x3daaaaab is weighed, the next float is weighed, a non-number is weighed, and a chest cut is dealt through the hit path
- **THEN** the first is not noticed, the next float is, the non-number is not, and the chest cut records one notice for that fighter

### Requirement: A second wind fires instead of the heartbeat
When a blow leaves a fighter alive under 0.05 yellow, a granted second wind whose ready clock is behind the game clock SHALL fire, wait 180000 ms, and skip the heartbeat message. At that ready clock it waits and the heartbeat message is written. The heal sound uses volume bits 0x3dcccccd and pitch bits 0x3f800000.

#### Scenario: Clock 12 fires a wind that was ready at 11
- **WHEN** a parried fighter already under 0.05 yellow has a granted wind ready at 11, the game clock is 12, and the hit path runs, and the same blow runs with the wind ready at 12
- **THEN** the first fires, waits until 180012, writes no heartbeat message, and uses volume bits 0x3dcccccd and pitch bits 0x3f800000, and the second writes the heartbeat message and does not fire

### Requirement: An active second wind heals yellow
While a second wind is active, one living health tick SHALL add the double 0.0003 to yellow and store the float. Yellow bits 0x3d000000 become 0x3d013a93. The wind stays active at game clock 3012 when its ready clock is 180012, and at 3024 it still adds and then ends. A quiet wind and a dead fighter add nothing.

#### Scenario: Clock 3024 ends a wind that was ready at 180012
- **WHEN** an attacking fighter with yellow bits 0x3d000000 and an active wind ready at 180012 is ticked at game clock 12, at 3012, and at 3024, and the same tick runs for a quiet wind and for a dead fighter
- **THEN** the first three yellow bits are 0x3d013a93, the first two stay active, the third ends, and the quiet and dead fighters keep yellow bits 0x3d000000

### Requirement: First-hit scaling
The hit SHALL return unchanged for a blocked or helpless combatant. Otherwise a first hit SHALL scale by `(1 + stat) × 0.025 + 0.95` in double precision. A target lacking bit 0x100000 SHALL take the threshold when its hit is lower and gain the bit. A body with that bit SHALL make one special call.

#### Scenario: The scaling
- **WHEN** the hit is first, the current value is 10 and the stat is 0
- **THEN** the scaled value is 9.75
- **WHEN** the stat is 1
- **THEN** the scaled value is 10
- **WHEN** the hit is not first
- **THEN** the current value is unchanged

#### Scenario: The target mark
- **WHEN** the target flag lacks 0x100000, its hit is 3 and the threshold is 5
- **THEN** its hit becomes 5 and the flag gains 0x100000
- **WHEN** the target already has the bit
- **THEN** its hit is unchanged
- **WHEN** the combatant is blocked or helpless
- **THEN** nothing changes

### Requirement: Combat wake
With the wake argument set the method SHALL clear bits 0x6 from the body's 0xdf8 word, wake the body once and call the 0x148 virtual once for each listed target. Without the argument it SHALL do nothing.

#### Scenario: The wake
- **WHEN** the wake argument is set, the body word is 0x7 and the list holds 3 targets
- **THEN** the word becomes 0x1
- **AND** one body wake and three target calls run

#### Scenario: No wake
- **WHEN** the wake argument is clear
- **THEN** nothing runs and the body word is unchanged

### Requirement: Combat forget
The forget SHALL remove the first entry equal to the target by moving the last entry over it and decrementing the count, or leave the list unchanged when absent. It SHALL zero the slots at 0x778 and 0x770 and the body's hit and contact pointers when they equal the target.

#### Scenario: The removal
- **WHEN** the entries are 1, 2, 3 and the target is 2
- **THEN** the entries become 1, 3 and the count 2
- **WHEN** the target is the last entry
- **THEN** it is dropped
- **WHEN** the target is absent
- **THEN** the list is unchanged

#### Scenario: The clears
- **WHEN** a stored slot or the body's hit or contact pointer equals the target
- **THEN** it becomes zero

### Requirement: Combatant reset
The reset SHALL zero the count, the 0x770/0x788/0x798 qwords, the 0x200/0x158 words, the 0x87e byte and the body's 0xe38/0xe50 pointers, and set 0x7a0 to −1. It SHALL clear a marker's two words and decrement a reference's count before clearing it.

#### Scenario: The reset
- **WHEN** a marker and a reference of count 3 are present
- **THEN** the listed fields are zero, 0x7a0 is −1, the marker clears and the reference becomes 2 with its pointer cleared
- **WHEN** neither is present
- **THEN** the fields still reset and neither path runs

### Requirement: Combatant measures
The measure pair SHALL store the given reach at 0x7b0 and the body value times 1.0 at 0x7b8.

#### Scenario: The measures
- **WHEN** the reach is 12.5 and the body value is 0.75
- **THEN** 0x7b0 is 12.5 and 0x7b8 is 0.75

### Requirement: Combat replace
The replace SHALL set the first entry equal to the old target to the new one and leave the list alone when absent. It SHALL replace slots 0x778 and 0x770 when equal, the body's hit pointer with the new target's 0x10 word and its contact pointer with the new target, each when equal to the old.

#### Scenario: The replacement
- **WHEN** the entries are 1, 2, 3, the old target is 2 and the new is 9
- **THEN** the entries become 1, 9, 3
- **WHEN** the old target is absent
- **THEN** the list is unchanged
- **AND** matching slots, hit and contact pointers take the new target

### Requirement: Weapon blood spots
Adding a hidden source SHALL add nothing and return 0. Otherwise the source SHALL take the first free spot, or grow the list, with an amount 0, a scale of its two lengths summed and divided by 80 in double precision, an inverse of the square root of 80 over that sum, a tag 0, and the 1-based index returned.

#### Scenario: Two sources
- **WHEN** a source of lengths 40 and 40 is added
- **THEN** it takes spot 1 with amount 0, scale 1 and inverse 1
- **WHEN** a source of lengths 10 and 30 is added
- **THEN** it takes spot 2 with scale 0.5 and inverse the square root of 2

#### Scenario: Hidden source
- **WHEN** a source carrying `0x400000` is added
- **THEN** nothing is added and the return is 0

### Requirement: Weapon blood clearing
Clearing the count SHALL drop every spot. Clearing a spot SHALL drop its source, amount and scale. Adding to a spot's amount SHALL keep it within 0 through 1.

#### Scenario: Reuse and clamp
- **WHEN** spot 1 has had 0.4 and then 0.8 added, and then −2
- **THEN** its amount is 1 and then 0
- **WHEN** spot 1 is cleared and a new source is added
- **THEN** the new source takes spot 1 and the list keeps two spots
- **WHEN** the count is cleared
- **THEN** the list is empty

### Requirement: Head look direction
The head tracking SHALL take the look-at minus the eye, reuse the previous unit direction when the squared xz distance is under 2,500, and in the band under 10,000 replace the vertical component with (10000 − d²)·1e-4 before normalizing.

#### Scenario: Distant and close targets
- **WHEN** the target is far away and off to the right
- **THEN** the direction comes from the normalized difference
- **WHEN** the target is under 50 cm away
- **THEN** the previous direction is used unchanged

### Requirement: Head angles
Yaw SHALL chase the approximate xz atan2 by at most π/100 a tick and stay within ±π/4 of the body's facing. Pitch SHALL chase the floored negative asin by at most π/100 a tick, with a −0.6 floor.

#### Scenario: Slew and body clamp
- **WHEN** the target is 45° to the right
- **THEN** the yaw moves one π/100 step
- **WHEN** the yaw would pass 45° from the body's facing
- **THEN** it is held at π/4

### Requirement: Head look result
The result SHALL be the unit direction (cos p·sin y, −sin p, cos p·cos y), the point 250 cm along it, and the flat unit direction.

#### Scenario: A step ahead
- **WHEN** the yaw is one step and the pitch is zero
- **THEN** the point is 250 cm along that direction and the flat direction keeps y at 0

### Requirement: Far head tracking
The far head variant SHALL use a 150 cm look-ahead and no pitch floor, keeping the same look bands, π/100 slew and ±π/4 yaw clamp.

#### Scenario: A far point and a steep look
- **WHEN** the target lies straight ahead a kilometre away
- **THEN** the published point is 150 cm along the direction
- **WHEN** the target lies far above and the pitch starts at −1.02
- **THEN** the pitch stays below −1, unfloored

### Requirement: Factory area colours
The system SHALL roll a factory area's colour, when its candidate does not fix one, by the material's colour class as the item randomiser does: steel 3 and iron 4, painted steel the item palette's metal colour, material 15 one of three colours and copper one of six by one draw, gems and resin none, and keep it as the lifted palette colour stored in bytes.

#### Scenario: Copper alloy
- **WHEN** a factory area rolls copper alloy with the generator in state 1
- **THEN** its colour is palette index 0x5D

#### Scenario: Random set 8
- **WHEN** a layer draws from random set 8
- **THEN** it chooses among materials 1, 2, 3, 4, 17, 13 and 20

### Requirement: Factory part placement
The system SHALL attach each assembled part at its parent's attach point, read from the parent component's stored box, moved by the parent's scale and band deforms, scaled by the parent's recipe scale and moved to the parent's position; a part without a parent SHALL keep its recipe position. Parts SHALL turn about X, then Z, then Y.

#### Scenario: Box corner
- **WHEN** a part at (1, 2, 3) attaches by mode 1 to a parent scaled 3 at the origin whose box minimum X is −1
- **THEN** it is placed at X 0 and Y 6

### Requirement: Factory paint job
The system SHALL paint each assembled part with its masks' area materials (an empty area as SteelGen), the areas' lifted colours and finish levels, the item's quality and dirt bytes over 255 and its damage remapped as 2/(2 − d) − 1, repeating library textures by the part's atlas region and shifting them by four offsets drawn from the item's second seed.

#### Scenario: Finish level
- **WHEN** an area's finish byte is 0x8B
- **THEN** its layer paints with wear weight 0.75

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
