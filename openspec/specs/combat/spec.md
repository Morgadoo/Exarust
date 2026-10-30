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
The system SHALL drive an opponent from the start of the fight with the NPC combat state, once per body tick: it enters combat in the Defence stance with its eagerness at a half, turns to the state's heading, steps to the places its footwork search chooses and starts the blows it chooses, through the player's animation and attack paths. On guard it SHALL stand in Cmbt - Idle when fresh and in the Defence stance when tired; its reach SHALL be 60 cm plus its weapon's sweet spot, or 50 cm unarmed.

#### Scenario: Opponent engages
- **WHEN** `exarust play arena --opponent` starts
- **THEN** the opponent closes in and attacks the player

#### Scenario: Stepping in and striking
- **WHEN** an unarmed duellist faces its target 5 m away, and another faces its target 3 m away, for up to 1500 body ticks each
- **THEN** the first walks toward its target in the combat walk, and the second strikes from where it stands, its attack ending in its stance, while a duellist under eight tenths of its energy never strikes

#### Scenario: Far and near
- **WHEN** `exarust --still-opponent --at 0,0,6 play arenao1` runs, then `--at 0,0,1.7`
- **THEN** the fresh duellist stands in Cmbt - Idle both 8 m and 3.7 m from the player, facing it

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
The system SHALL raise a fighter's readiness by 0.004 a tick, up to 1, while its stance is Cmbt - Idle or it parries, and its clip is Cmbt - Idle, it parries or its attack is released; else it SHALL reset it to 0. A blow SHALL scale the victim's penetration, impact and stab energy by 1 − 0.5 × its readiness, after the shove is taken from the impact.

#### Scenario: Ready for 3 s
- **WHEN** a fighter that has stood in Cmbt - Idle for 3 s takes a sword cut on bare flesh
- **THEN** the cut's penetration is half what it would be on a fighter in the Defence stance, and the shove is the same

#### Scenario: Readiness through a parry
- **WHEN** a fighter in the Defence stance playing an attack clip parries
- **THEN** its readiness rises by 0.004 a tick, and it resets to 0 once the parry ends

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
While a fighter's energy is more than 0.05 under its most (double precision) and it does not parry, its stance SHALL be the Defence stance every body tick, and the player's stance rule SHALL not give Cmbt - Idle. Unless helpless, attacking or shock-staggered, the Defence clip SHALL start when another plays, blend round((1.05 − E) × 300), eased smoothly. A daze is decided before the posture stage, so a fall's clip wins.

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

### Requirement: Shock stagger
While the shock-stagger flag is set, a body tick SHALL set energy to its most minus 0.5 in single precision, set the stance to Defence (when the body has one) without starting its clip, start Rebalance with no blend unless it already plays, and clear the flag once the game clock is past the next shock. The daze's Defence clip SHALL NOT start on that tick. A shock on a fighter that is not collapsing or dead SHALL set the flag.

#### Scenario: The flag holds, then the clock passes
- **WHEN** a fighter with the flag, energy 1 and most 1, plays Idle with the next shock due at 500 ms, and is ticked at 0, 144 and 504 ms
- **THEN** the first tick leaves energy at 0.5, the stance Defence and Rebalance started with blend 0, the flag kept; the second keeps the flag without restarting Rebalance; the third clears the flag

#### Scenario: A body without a stance
- **WHEN** a staggered body that has no stance plays Idle
- **THEN** its energy is pinned and Rebalance starts as for a fighter

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
A guarded blow SHALL meet no armour layer and draw no coverage roll. A weapon's parry SHALL deal nothing: the parrying fighter's attack state clears, it is kept from attacking 600 ms unless it has Riposte, and its stance sags by half the impact at the hit, at most 1. A blow on a shield SHALL deal no lasting wound (Shield stamina). Either guard SHALL make a striker smaller than 1.4 recoil. An open guard SHALL keep the flesh wound.

#### Scenario: Parry
- **WHEN** the same cut that wounds bare flesh is parried
- **THEN** the wound and the yellow damage are zero, the fighter's health is unchanged, it may not attack for 600 ms, and the striker recoils

#### Scenario: Tired shield
- **WHEN** that cut meets a shield of block 0.58 with an empty shield arm
- **THEN** the lasting wound is zero and yellow falls to 0.22621973 (bits 0x3e67a625), less than the flesh wound

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
The measure pair SHALL store the given reach as the combatant's reach and the body value times 1.0 as its body measure.

#### Scenario: The measures
- **WHEN** the reach is 12.5 and the body value is 0.75
- **THEN** the reach is 12.5 and the body measure is 0.75

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
A blow on a body whose state flag 0x400 is set SHALL deal nothing (no health, no push, no daze) and SHALL make the striker recoil, whatever its size; the blow sag the armour pass set before the turn SHALL stay.

#### Scenario: Turned
- **WHEN** the tests' cut lands on a body that turns blows
- **THEN** its health is whole, the only shove carries no push and no daze but the blow sag min(2·I, 1), and the striker's swing recoils at strength 0.5

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

### Requirement: Shield stamina
A blow on a shield not parrying SHALL cost the arm's stamina cost·0.8·I, or cost·(0.3x + 0.3)·I with x = clock/0.3 under Synchrony while the holder counters this attack before 0.3 of its clock; the stamina stays at least 0. With x above 0 the arm SHALL leak (1 − S)·(0.4·I + 0.4·x)·(1 − block/2), taking 0.05 of it off yellow (0.025 under a potion), and daze the holder (0.3 − 0.2·S) times the leak.

#### Scenario: A fresh arm
- **WHEN** a cut of impact 0.6746667 meets a shield of block 0.58 on a fresh arm
- **THEN** the arm keeps 0.61678928 of its stamina and yellow falls from 0.25 to 0.24088715

### Requirement: Shield parry
A shield parry SHALL cost the arm cost·0.4·I and leak nothing, lock the holder 600 ms unless it has Counter, sag its stance (0.7 − S/2)·I (at most 1) and daze it (0.3 − 0.2·S)·I; yellow under 0.05 SHALL run the low-health check after any block.

#### Scenario: Parried on the shield
- **WHEN** that cut meets a shield parry on a fresh arm
- **THEN** the arm keeps 0.80839467, the sag is 0.19956821 and the holder is locked

### Requirement: Swing end rounding
The end a recoil or a follow-through writes SHALL be its scale times 400 rounded half to even, not truncated, plus the clock (and the recoil's 200 ms).

#### Scenario: A short recoil
- **WHEN** a striker recoils with a drive of 0.013 at 1000 ms
- **THEN** its stance returns at 1201 ms (0.52 rounds to 1)

#### Scenario: A glancing follow-through
- **WHEN** a body blow at 0.009 cm a tick with gain and recoil 1 is followed through at 1000 ms
- **THEN** it ends at 1001 ms

### Requirement: Stab pierce without a number
A stab whose energy is no number SHALL take the pierce's root branch: no pierce while the shove is nothing, never the thrust's √2.

#### Scenario: A lost contact
- **WHEN** the tests' stab lands with no number for its contact
- **THEN** its penetration, wound and shove are 0

### Requirement: Turned parry
A parrying body whose state flag 0x400 is set SHALL take nothing from a clash (no lock, no attack-state clear, no sag) and SHALL make any striker recoil, whatever its size.

#### Scenario: Turned parry
- **WHEN** a striker of size 1.5 is parried by a body that turns blows
- **THEN** the parrier's swing state and stance are untouched and the striker recoils at strength 0.5, where a plain parry leaves that striker's swing

### Requirement: Striker per contact
The system SHALL deal each weapon contact's striker (its wound follow-through, cleave or recoil) right after that contact's struck side, before the next contact of the frame, as the original's damage routine runs once per contact.

#### Scenario: Two fighters strike each other
- **WHEN** two fighters wound each other in one frame
- **THEN** the first contact's striker takes its follow-through before the second contact stalls it as a struck attacker

### Requirement: Guard sounds once a swing
A clash and a block SHALL set the swing's weapon-sound bit 0x1000; a block SHALL set its shield-sound bit 0x200 once a swing and draw the shield's sound cooldown from the generator then (the wound bits already hold 0x200).

#### Scenario: Two blocks in one swing
- **WHEN** a swing is blocked twice
- **THEN** the generator steps once, and not at all for a swing that already wounded

### Requirement: Main-side start sound
A free main-side attack start SHALL play its sound, unless the body's quiet byte is set, with the pitch (0.2·u + 0.8) / √scale from the first uniform draw and the gain 0.4·v + 0.6 from the second, each computed in double precision and stored as single.

#### Scenario: Draws of one half and one quarter
- **WHEN** the draws are 0.5 then 0.25 and the body's √scale is 2
- **THEN** the pitch is 0.45 and the gain 0.7

### Requirement: The parry check in the body tick
The system SHALL run the parry check for the ticked fighter after its swing routine: from the combat idle when its muscle tone is at least 0.85, its auto-parry deadline has passed and its hands hold something, and on every tick it already parries. A parry that starts SHALL advance the counter twice in that tick. A fighter holding a weapon SHALL NOT parry until the armed grip is modelled.

#### Scenario: A mid swing in front
- **WHEN** an unarmed fighter in the combat idle at game time 12 000 ms faces a foe 1 m ahead and 0.5 m higher attacking with Attack Mid at clock 0.5
- **THEN** it parries with Parry Left, blend 20 frames, fast ease, last-check time 12 000 and counter 2

#### Scenario: The deadline is strict
- **WHEN** the last attack tick was at 11 600 ms
- **THEN** the idle starts no parry at 12 000 ms, and one at 11 599 ms does

#### Scenario: Tone and hands
- **WHEN** the muscle tone is 0.84, or the hands are empty, or a weapon is held
- **THEN** the idle starts no parry

### Requirement: The parry trigger's window
The trigger SHALL skip the foe's clock window (0.3 through 0.9) only for a swing carrying the continuation flag; an open strike window SHALL NOT skip it.

#### Scenario: Clock edges
- **WHEN** the foe's clock is 0.29, 0.3, 0.9 and 0.91
- **THEN** the parry starts at 0.3 and 0.9 only

### Requirement: The parry's re-pick and counter
The check SHALL store the countered motion and the picked clip whenever it picks again, advance the counter from the stored clip on every checked tick, and let the counter of a fighter not parrying drift one step a tick toward 0.

#### Scenario: A new motion
- **WHEN** the foe changes from the main side to the other side while the fighter parries a level foe
- **THEN** the countered motion becomes 8, the clip Parry Rgt Dn with blend 20

### Requirement: The parry's end
A parry that ends SHALL return to the Defence stance when yellow health is under 0.01 or the fighter is dazed, else to the combat idle, with a 40-frame smooth blend, unless the condition's clip stands still. A fighter without the permission SHALL only drop the parrying flag.

#### Scenario: Dazed
- **WHEN** the foe stops attacking and the fighter's energy is 0.1 under its most
- **THEN** the parry ends into the Defence stance

#### Scenario: Shocked
- **WHEN** the fighter is shocked
- **THEN** the parry ends and its clip and stance stay

### Requirement: The parry guard
The fighter's guard SHALL be the parry's only while it parries or counters a motion, SHALL NOT replace a shield's guard, and SHALL open when the hit path ends the parry.

#### Scenario: A shield held
- **WHEN** a fighter holding a shield parries
- **THEN** its guard stays the shield's

### Requirement: Level impact damage
A character's level impact SHALL count on its part, and once the root's sound clock is past, take the limb curve (a third of the strength unless the character is down, then half; silent under 0.5), sound at the character, and over a volume of 0.25 deal the limb impact damage, daze and collapse a body's blow deals, then set the clock; a hard impact on a striking part in an overhead swing SHALL be dropped before the clock is set. A level impact SHALL NOT tell any other character.

#### Scenario: A hard fall
- **WHEN** a character's ball part drops from 5 m onto a floor
- **THEN** it sounds once, its yellow health falls when the volume passes 0.25, no limb impact message is sent and the root's clock is set

### Requirement: Armour fold precision
The armour pass SHALL compute a layer's effective coverage as (1 − c)·((c − ½)·1.75) + c in float above one half and c otherwise (a NaN keeps c), the natural layer's coverage and padding as double products narrowed once and its resistances as float products, and the folded resistance as (1 − s0′)·(s1·γ1) + s0′ with s0′ = γ0·s0.

#### Scenario: Fold grouping
- **WHEN** a part is covered by items of 80 % slash 30, 51 % slash 90 and then any third item
- **THEN** the inner layer's slash resistance is the float `0x3eece03c`

#### Scenario: Effective coverage
- **WHEN** a layer covers 0.8 of a part
- **THEN** its effective coverage is the float `0x3f67ae15`

### Requirement: Second wind from worn items
A character SHALL be granted the second wind when one of its worn items, found as the encumbrance finds it, has special kind 2; placed characters and scene bodies SHALL both resolve it.

#### Scenario: Kind-2 item
- **WHEN** a record wears an item whose special kind is 2
- **THEN** its fighter's second wind is granted, and a record whose worn ids find no such item is not

### Requirement: Factory emblem layers
The system SHALL paint an area's emblem as a layer after the area's own one, with the same mask, material and wear weight, the emblem's colour and the picture `emblembkg0` followed by the id's two digits; the layer SHALL paint only the colour pass, and with a painted-pattern material (1, 7, 8, 25) its mask SHALL be multiplied by the picture's red at the region coordinate times `PtrnPos.xy` plus `PtrnPos.zw`, otherwise it SHALL cover its whole mask.

#### Scenario: A cloth emblem
- **WHEN** a ClothBasic layer carries an emblem whose picture is opaque on its left half
- **THEN** the emblem colour shows on the left half only and the specular and normal maps are those of the base layer alone

#### Scenario: A material without the program
- **WHEN** a ClothFine layer carries the same emblem
- **THEN** the emblem colour covers the whole mask

### Requirement: Factory pattern position
The system SHALL start `PtrnPos` at (atlas width / height, 1, 0, 0) and, for every assembled part whose chosen variant has flag `0x1000`, multiply its scale by f = 1 − byte/254 (byte/127 when negative), move its offset by −(f − 1)/2 and then by f · offset byte/127.

#### Scenario: Half scale
- **WHEN** a 512 by 256 atlas has one placing variant with scale byte 127 and offset bytes 127 and −127
- **THEN** `PtrnPos` is (1, 0.5, 0.75, −0.25)

### Requirement: Blade texture stretch
The system SHALL take a paint region's `UVScale` from the last of its part's deforms whose header word is 6, as the deform factor of the negated value, else 1, and SHALL sample only blade layers' mask, shade and region normal at (u, v · UVScale).

#### Scenario: A stretched blade
- **WHEN** a texture-only deform of amount 0.5 has value 0.5
- **THEN** the region's `UVScale` is 0.8

### Requirement: Armed parry blade height
The armed parry SHALL meet the attacker's blade at the blade length less 20 cm when the chest gap less 20 cm is beyond the attacker's reach, and otherwise at the blade length less the unused reach less 20 cm, floored at 0; a NaN gap SHALL take the second branch.

#### Scenario: Within reach
- **WHEN** the chests are 100 cm apart, the reach is 100 cm and the blade 90 cm
- **THEN** the parry meets the blade 50 cm up

### Requirement: Armed parry sweep
The armed parry's sweep direction SHALL be the blade point's travel this tick, its height scaled by 16 − 16·clock outside the stab and the overhead, less its part toward the attacker's chest, normalised with the approximate reciprocal square root.

#### Scenario: Side swing
- **WHEN** a side swing at clock 0.75 rises 1 cm and travels 1 cm across
- **THEN** the sweep runs 4 up for every 1 across

### Requirement: Armed parry point
The parry point SHALL be the blade point moved along the sweep by the defender chest's lead less 60 cm (80 cm for an overhead), floored at 0; when the blade point already lies further along the sweep than the defender's chest the blow SHALL be passed and the point SHALL be 10 cm on. The falloff SHALL read the point's ground distance from the defender's chest.

#### Scenario: Blade ahead of the chest
- **WHEN** a stab's blade point sweeps toward a defender chest 100 cm along the sweep
- **THEN** the point sits 40 cm along the sweep and the blow is not passed

#### Scenario: Blade past the chest
- **WHEN** the defender's chest lies 5 cm behind the blade point along the sweep
- **THEN** the blow is passed and the point is 10 cm on

### Requirement: Armed grip arithmetic
A weapon's grip strength SHALL be (1.4 − handling) × (falloff × 0.75) in double, stored as a float, then × 0.6 in double; a shield's SHALL be stamina × (weight × (falloff × 0.75 × 0.4)) in double. A NaN distance SHALL give a NaN falloff, and the parry's blend SHALL be 20 × its speed rounded half to even.

#### Scenario: Half-way blend
- **WHEN** the parry speed is 0.625
- **THEN** the blend is 12 frames

### Requirement: Stance by energy
The stance a fighter returns to by its energy SHALL be the Defence stance when its yellow health is under 0.01 or its most energy less 0.05 exceeds its energy, both compared in double, and the combat idle otherwise; a NaN energy SHALL give the combat idle.

#### Scenario: Energy at the threshold
- **WHEN** a fighter's energy is the float 0.95 and its most is 1
- **THEN** it takes the Defence stance

### Requirement: Tangle break
Each tick of a living body (muscles on) that is neither down nor dying SHALL run the tangle counter outside a parry: it falls by one (floored at 0), then each hand part inside the box y −5…60 cm, z < 0, |x| < 20 cm of the character frame adds 2; past 60 the counter clears and the Break Tangle clip starts from where the body is, returning to the stance and ending an attack in progress. A parry SHALL leave the counter as it is.

#### Scenario: Hands behind the back
- **WHEN** both hands stay behind the back of a standing fighter
- **THEN** the counter reads 4 after one tick, 58 after 19, and the 20th tick clears it and starts Break Tangle

#### Scenario: A parry holds the counter
- **WHEN** the fighter is parrying
- **THEN** the counter does not change

### Requirement: Pain sounds
After the damage call the system SHALL derive the victim's pain sounds from the wound and the yellow (both before the bars' 0.75): above a wound of 0.3 the wound sound at 0.8 of the voice's volume and a groan; otherwise, when the wound passes the epsilon or the yellow 0.2, the wound sound past a wound of 0.1 and a groan over a yellow of 0.5, else a grunt.

#### Scenario: A deep wound groans
- **WHEN** a blow leaves a wound of 0.5
- **THEN** the victim's wound sound plays and its voice groans

#### Scenario: A bruise grunts
- **WHEN** a blow leaves no wound and a yellow of 0.3
- **THEN** no wound sound plays and the victim grunts

### Requirement: Mute victims
A victim whose voice level is under 0.01 SHALL make no pain line, and one without a wound sound SHALL play none.

#### Scenario: A mute victim
- **WHEN** a victim with voice level 0.009 takes a deep wound
- **THEN** its wound sound plays and it makes no voice line

### Requirement: Player jolt
A struck player SHALL feel 7 × the yellow plus 7 × the wound; a striker of scale above 1.3 SHALL make it twice its speed plus four times that, times 0.4 for an overhead swing. The camera SHALL jolt by its square root when it passes 1 and the pad rumble SHALL rise by it. A striking player's pad rumble SHALL rise by half the yellow it dealt.

#### Scenario: A hard blow on the player
- **WHEN** the player takes a yellow of 0.1 and a wound of 0.05 from a human
- **THEN** the camera jolts by the root of 1.05

### Requirement: Held strikers gate the executor
The attack executor SHALL start nothing, not even a released swing's overhead, for a fighter that holds no striker. The held count SHALL count each held weapon and each fist or claw that the empty hands hold: 2 unarmed, 1 with one weapon, 2 with two.

#### Scenario: Unarmed and armed fighters attack
- **WHEN** an unarmed fighter (two fists) and a fighter with one sword request an attack once their cooldowns have passed
- **THEN** both start a side attack

#### Scenario: No striker held
- **WHEN** a fighter with no hand strikers requests an attack, free or with a released swing
- **THEN** nothing starts

### Requirement: Procedural pass tiles
The system SHALL compute `MatTile` as tile size times region size over the layer material's first colour texture size and `DmgTile` as tile size times region size over 1024, where the tile size is the job size, doubled below texture detail 3.

#### Scenario: Low-detail tiles
- **WHEN** a region of size (0.5, 0.5) is drawn into a 256 by 128 job at texture detail 2 against a 256 by 256 texture
- **THEN** `MatTile` scales are (1, 0.5) and `DmgTile` scales (0.25, 0.125)

### Requirement: Procedural shade tile
The system SHALL bind `ShdTile` as the region rectangle (size, offset) when the region has its own shade tile and as (1, 1, 0, 0) otherwise, with its y multiplied by `UVScale` in every pass; `PtrnPos` SHALL be bound only in the painted-pattern pass and `NrmTile`, the unfolded region rectangle, only in the specular pass.

#### Scenario: A stretched region
- **WHEN** a region of size (0.5, 0.5) at offset (0.25, 0.5) with `UVScale` 0.5 is drawn
- **THEN** `ShdTile` is (0.5, 0.25, 0.25, 0.5) in the colour, pattern, specular and normal passes, and the specular `NrmTile` is (0.5, 0.5, 0.25, 0.5)

### Requirement: Factory paint targets
The system SHALL paint a factory part at its atlas region's texel size (rounded to nearest, magnitude of a negative scale), halved by integer division below texture detail 3, and SHALL draw only the target columns whose centre lies left of the region's width crop (1 − texture parameter) times the width, sampling the source at an unscaled u.

#### Scenario: Low detail
- **WHEN** a 512 by 256 region is painted at texture detail 2
- **THEN** the target is 256 by 128

#### Scenario: A cropped region
- **WHEN** a region with crop 0.5 is painted 4 texels wide
- **THEN** columns 0 and 1 paint as without the crop and columns 2 and 3 keep the cleared colour and surface (0, 0.01, 0)

### Requirement: Factory repaint
The system SHALL keep a factory weapon's assembled meshes when only its condition changes and repaint them from the same seeds and stored choices.

#### Scenario: Damage only
- **WHEN** a weapon with the same seeds is loaded again with a higher damage byte
- **THEN** its mesh handles are the same and its materials differ

### Requirement: Paint job ring and surface cache
The system SHALL model the 256-slot paint-job ring (a push stores first and a full ring flushes every job but the new one) and the surface cache of 64 slots with a separate pool of 64 >> 4 for extra-layer jobs, where a get scans from the read cursor and a return to a full ring frees the entry at the write cursor.

#### Scenario: A full ring
- **WHEN** 256 jobs are pushed into an empty ring
- **THEN** the last push runs 255 jobs and the 256th stays queued

### Requirement: Factory impact sound
The system SHALL give a factory weapon the hit sound of the factory's table by its section's sound byte (`WH_SwordA`, `WH_MaceMA`, `WH_AxeA`, `WH_HammerA`, `WH_MaceMA`, `WH_ClubMW`, `WH_WoodA`, `WH_ClawsL`, `BdyCol`), and `WH_SwordA` without a section.

#### Scenario: Class 3
- **WHEN** the sound byte is 3
- **THEN** the sound is `WH_HammerA`

### Requirement: Stab aim
While an AI fighter's stab plays, its behaviour SHALL keep a stab aim, which starts on the attack's aim yaw with pitch 0. Each behaviour tick the aim SHALL turn toward the fighter's target point by at most 0.0314 rad in yaw and in pitch, look up by no more than 0.6 rad and stay within 45° of the body's heading. Directions SHALL be measured from an anchor 20 cm to the side of and 10 cm above the character frame's origin.

#### Scenario: The aim turns a little each tick
- **WHEN** a stab starts facing along +z and its target lies 300 cm to the side
- **THEN** after one tick the aim's yaw is 0.0314 rad, and it stops 45° from the heading

#### Scenario: A high target
- **WHEN** the target lies far above the anchor
- **THEN** the aim's pitch settles at 0.6 rad up

### Requirement: Stab aim point
A target closer than 50 cm across the ground SHALL leave the stab aim as it was. For a target between 50 and 100 cm, the height of the direction SHALL be (10000 − h²)·0.0001, h being the distance across the ground in cm. The aim point SHALL lie 250 cm out from the anchor along the aim.

#### Scenario: A close target ignores its height
- **WHEN** two targets lie 60 cm ahead, one 5 m above the anchor and one 5 m below
- **THEN** the aim settles at the same pitch for both

#### Scenario: The aim point
- **WHEN** the target lies straight ahead, level with the anchor
- **THEN** the aim point lies 250 cm ahead of the anchor

### Requirement: Stab arm drive across the line
Past clock 0.1, a stab that has a held weapon and an aim point and has not landed a blow, on a body that is not a quadruped, SHALL push the weapon point and the left hand toward the line from the anchor to the aim point. The weapon point is 0.9 of the first held item's length up from its grip. With s = S((c − 0.1)·1.65), where S(u) = (3 − 2u)·u² floored at 0, the weapon point's push SHALL be 6·c·s of its offset to the line and the hand's push c²·s of its own.

#### Scenario: Across first
- **WHEN** a stab's weapon point lies 30 cm beside the line at clock 0.3
- **THEN** it is pushed 30·6·0.3·s cm toward the line and not along it

### Requirement: Stab arm drive along the line
Past clock 0.4, with g = 4·(1 − c)²·(3 − 2t)·t and t = c − 0.4, the stab drive SHALL also push the weapon point 48·g·(W7 + 0.8) cm along the line and the left hand 12·g·(W7 + 0.8)·(1 − c)² cm, W7 being the weapon's balance. A weapon with no body of its own SHALL take its push through the hand that grips it.

#### Scenario: The thrust drives on
- **WHEN** the clock is 0.6 and the weapon's balance is 0.5
- **THEN** the weapon point is pushed 20.77 cm along the line and the hand 0.831 cm

#### Scenario: A golem's halberd is driven at the player
- **WHEN** a woken golem stabs the player with its halberd
- **THEN** the halberd's point is pulled toward the line to the player's chest and driven along it, and a stab that meets the body with its point deals damage
