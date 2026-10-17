## ADDED Requirements

### Requirement: Swing release
When an attack's request drops during the attack, unless its swing recoiled, cleaved or was released already, the system SHALL release it: strength P = 1 drains by max(1 − 0.05·|v|, 0.1)/W17 × 0.03 a tick (at least 0.01, v its striker's tracked speed), the clip × P, the stance back after 1000/(80 × that) ms, at most the swing's expected rest. A released main-side attack SHALL return to the stance once that time has passed (blend 2·W19, 30 without a primary striker).

#### Scenario: Click
- **WHEN** the unarmed player clicks once in combat mode
- **THEN** its punch starts, is released the next tick (stall 0.3 a tick) and is back in its stance 5 ticks after it started

#### Scenario: Late fast swing
- **WHEN** a swing of recoil 0.42 is released at 0.6 of its clip, its striker tracked at 20 cm a tick
- **THEN** it stalls by 0.01 a tick and its stance is due at the swing's expected rest, 426 ms later for Sword A

### Requirement: Struck swing
A fighter dazed by D (at most 0.9) while attacking SHALL have its swing stall by r = 4·D·f − (1 − D)·min(0.005·s, 1)·W17 a tick when r ≥ 0.001 (f = 0.25 over its most health, halved under a potion; s its strikers' larger tracked speed squared), added to a stall in progress up to 1; its stance SHALL return 80·P/stall ms later, at most the swing's expected rest. A spent swing (P = 0) takes no stall.

#### Scenario: Cut mid-punch
- **WHEN** a human punching (12 of 100 frames into its clip, its fist still) takes a forearm cut of daze 0.428
- **THEN** its punch plays on, stalled by 1.713 a tick, and its stance is due 47 ms later

#### Scenario: Fast swing shrugs off a light daze
- **WHEN** a swing of recoil 0.42 tracked at 30 cm a tick takes a daze of 0.05
- **THEN** it does not stall

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

## MODIFIED Requirements

### Requirement: Weapon contacts
The system SHALL register at most one contact per attack and target, when the segment of a striker whose strike window is open (a held weapon's blade, or a fist when the fists are what is held) passes within a body part's volume; the blow reads the swing's clock, the clip's clock less its blend over its last key.

#### Scenario: Strike lands
- **WHEN** an opponent's attack sweeps its blade through the player's body
- **THEN** one hit on the nearest body part is reported

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

### Requirement: Attack executor
The system SHALL start a requested attack (player's or AI's) only with at least 80 % of the fighter's most energy and, unless a released swing allows it (Released swing), while no swing stalls and once its cooldown has passed since its last attack tick on the game clock (12 ms a tick): 400 ms with no primary striker, else round(300 × recoil) + 300 ms. No overhead over a stab or an overhead, stab over a stab, side attack over a side attack; a stab needs the weapon's long axis forward of −0.367.

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
