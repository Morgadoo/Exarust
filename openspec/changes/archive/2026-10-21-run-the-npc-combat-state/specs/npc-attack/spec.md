# Spec Delta

## ADDED Requirements

### Requirement: Practice from fighting
A lesson above 0.005 SHALL feed the gain by `(1 − fatigue)·lesson·0.1` and the fatigue by `lesson·0.0001`, capped at 0.9; a gain above 10 SHALL pay four times its rounded value into the record's practice count and keep the remainder; the lesson SHALL decay by 0.995 a tick and the fatigue always by 0.9998.

#### Scenario: A gain past ten
- **WHEN** a lesson of 1 meets a gain of 9.95 and no fatigue, then a lesson of 0.005 meets a gain of 3 and a fatigue of 0.5
- **THEN** the first pays 40 and keeps a gain of about 0.05 with the lesson at 0.995, and the second pays nothing and only rests the fatigue to 0.5 × 0.9998

### Requirement: Entering combat
Entering the combat state SHALL set the eagerness to a half, stop the press and stamp the clock; a fighter already in combat SHALL keep its state.

#### Scenario: Enter once
- **WHEN** a fighter with eagerness 3 that presses enters combat at 1234 ms, and is entered again later
- **THEN** its eagerness is 0.5, it no longer presses, its stamp is 1234 ms, and the second entry changes nothing

### Requirement: The foe estimate
With probability `(0.8·k² + 0.2)²` (k the skill) the fighter SHALL look at its foe again and blend the new position back toward the old by `0.6 − 0.6·k²`; the distance SHALL be at least 1 cm (the direction then left as it is) and 800 cm when the estimate is more than 300 cm above or below the chest. A stepping foe SHALL be led toward its step's goal by three quarters of the skill and a running one ahead by its speed × 30.

#### Scenario: Novice and master
- **WHEN** a novice's generator at 0 looks at a foe 300 cm ahead after an estimate 100 cm aside, a master looks at a foe 400 cm above, and a stepping foe is estimated by a master
- **THEN** the novice's estimate is 0.6 of the way back toward the old one, the master's distance is 800 cm, and the stepping foe is estimated three quarters of the way to its goal at its chest height

### Requirement: Drawing a weapon and lunging
A fighter with fists or a one-handed class and nothing held SHALL draw a carried weapon, holding its level at a watch while the draw is asked for or plays. Engaged and outside a swing, one tick in 300 SHALL start a lunge that measures the reach 80 cm longer.

#### Scenario: Draw and lunge
- **WHEN** a bare-handed fighter carries a weapon, then asks for the draw, then plays it; and an engaged fighter of reach 90 cm draws 0 of 300
- **THEN** the first asks for the draw, the other two hold the watch, and the lunge measures 170 cm

### Requirement: Chasing a foe
An engaged fighter with balance above 0.85 SHALL count itself stuck while its foe is beyond 600–800 cm (400 cm lying, 200–300 cm running, three quarters of that when it runs), more than 150 cm above or below, or out of sight, and SHALL start a chase after 21 such ticks unless the foe is marked unreachable.

#### Scenario: Stuck, then close
- **WHEN** a novice's foe stays 900 cm away for 22 ticks, then comes to 100 cm
- **THEN** it counts 1 to 21, chases on the 22nd tick, and the count resets close in

### Requirement: Eagerness
With full energy an engaged fighter's eagerness SHALL drift by `(3 − k)·(0.02·draw − 0.006)` up to 6 and fall by 0.01 above it; short of energy it SHALL be scaled by `1 − (1 − E/Emax)²`, and under eight tenths of the energy the press SHALL stop. Off balance or not engaged, the eagerness SHALL be 0.

#### Scenario: Tired
- **WHEN** a fighter of eagerness 2 that presses has half its energy, and then stands at balance 0.85
- **THEN** its eagerness is 1.5 and its press stops, and then its eagerness is 0

### Requirement: The footwork drives
Outside its own early swing the attack drive SHALL be the eagerness plus a half (halved by skill while it recovers or swings), the strike drive the eagerness (half while recovering) and the spacing `(2k + 3)·(2 + k)`. Below the engage level there SHALL be no attack or strike drive. The attack and strike drives SHALL scale with the blend and the square of the energy.

#### Scenario: Below the engage level
- **WHEN** a novice at level 2 works out its drives
- **THEN** its attack and strike drives are 0 and it does not press

### Requirement: Energy and the press scale the drives
At full energy a press SHALL multiply the attack drive by `2 + k` (twice more with fists) and the strike drive by 3; at full energy without a press both SHALL halve and the spacing double; short of energy the attack SHALL halve and the spacing quadruple.

#### Scenario: Fresh and tired
- **WHEN** a novice with fists at eagerness 0.5 presses at full energy, and then at half its energy
- **THEN** its drives are 4, 1.5 and 6, and then 0.125, 0.125 and 24

### Requirement: The swing's commitment
Early in its own swing (before 0.7 of the clock, 0.9 with fists) the fighter SHALL spend eagerness past 0.6 by `(1 − age/2)·(0.5 − 0.2k)·draw` and record its foe's lesson, measure its overshoot past 0.5 (twice the lunge reach less its distance, within −50 and 100, averaged by a tenth), press past 0.4 and commit past 0.3, its attack drive `(eagerness − k)·2 + 2`, doubled committed (four times with a shield). Without a press, the press SHALL come within 3 s of the last.

#### Scenario: Mid-swing
- **WHEN** a novice with a weapon swings at clock 0.55 with a lunge reach of 130 cm, 100 cm from its foe
- **THEN** its overshoot is 60 with a mean of 6, and its drives are 12, 1.5 and 4

### Requirement: The places weighed
The fighter SHALL weigh where it stands (or where its step lands) and eight steps (ahead, back, both sides, the four diagonals), and with skill and energy eight longer steps of `1.6 − 0.3·encumbrance` steps (ten places for a body scale above 1.25). A long step past the first two directions SHALL cost 2 more.

#### Scenario: A master and a big body
- **WHEN** a master at full energy plans its footwork, then one of body scale 1.3
- **THEN** it weighs seventeen places, and the big body eleven

### Requirement: The cost of a place
Each place SHALL cost its ground, the change from its step, three times each contact's danger and twice its closeness; where it stands it SHALL also re-choose its foe, the most pressing contact, whose closeness is then added once. A place SHALL be a strike place when its approach times the attack drive, less half its danger times the spacing, is above 1. The strike drive SHALL pull a place toward the foe by its strike term.

#### Scenario: A crowded place
- **WHEN** a master weighs seventeen places against a contact at level 1 that is close (1) and dangerous (0.5) everywhere
- **THEN** where it stands costs 3.5, the steps 4.5, the long steps past the second 6.5, it stays, nothing is a strike place, and the contact becomes its foe at level 1

### Requirement: The attack decision
A strike place SHALL win when it is cheaper than standing (by at least 2, once committed by 1), when no friend stands ahead within reach + 80 cm (one aside stops it in a swing), and a contact whose concern and hostility pass 1.8 SHALL not count as a friend. Outside its own swing a forward strike place SHALL give way to the place it stands on. A step SHALL keep off the foe's 60 cm × scale circle. Without a strike, a place SHALL be stepped to only when it is cheaper than standing by 1 or more.

#### Scenario: Friends, a foe in reach and a foe out of it
- **WHEN** a friend stands 100 cm ahead, 100 cm aside, behind, 131 cm ahead, or is hostile; a pressing fighter with a foe 100 cm ahead can strike within 120 cm; and the same fighter has a friend 60 cm ahead
- **THEN** the friend tests give 2, 1, 0, 0 and 0; the fighter strikes from where it stands; and with the friend in the way it does not strike

### Requirement: The guard
Without a blow a fighter of size under 1.3 SHALL let its swing go and take the guard when tired (yellow health under 0.01 or energy under its most less 0.05), else the combat idle, and SHALL lean 25° × skill against a foe's side swing inside the foe's reach + 50 cm. A fierce body SHALL instead take the guard at four reaches or more and the combat idle within, re-aiming only past 54°. The heading SHALL move 0.03 of its gap each tick.

#### Scenario: Lean
- **WHEN** a master on guard faces a foe of reach 100 cm swinging its first side 100 cm away
- **THEN** it aims (150 − 100)/120 × 25° aside and its heading moves 0.03 of that

### Requirement: The swing's body turn
During a swing the heading SHALL turn by σ(clock) × the swing's turn amount, σ −80° at the start, −60° to 0.2, a ramp to 30° at 0.5; open hands SHALL turn 50° less, a heavy claw 50° more, and a lunge 1.5 times as far. Late in the swing (0.68 for the main side, 0.78 for the other) the commitment SHALL halve, or clear with a stall, a hit or a release.

#### Scenario: Sweep
- **WHEN** a swing's sweep is read at clocks 0, 0.1, 0.35 and 0.6, then with open hands, a heavy claw and a lunging heavy claw at 0.6
- **THEN** the sweep is −80, −60, (0.35 − 0.2)·300 − 60 and 30, then −20, 80 and 120

### Requirement: The strike plateau
A fight score SHALL be 0.25 at 25 cm when the reach is 50 cm and the size is 1, and 1 from 120 cm through 130 cm when the step is 80 cm. At 600 cm the strike term SHALL be 0 and the fighter SHALL keep closing. The close term SHALL be 0 one centimetre past reach plus 150 cm. Outside a fight the close term SHALL double and the strike term SHALL be 0. A large opponent SHALL make 100 cm score 1. Wanting melee SHALL double 0.25 to 0.5.

#### Scenario: Plateau at 120 cm
- **WHEN** a fight with reach 50 cm, size 1 and a step of 80 cm is scored at 25 cm, 120 cm, 130 cm and 600 cm, the same spot is scored outside a fight, a large opponent is scored at 100 cm, and melee doubles 25 cm
- **THEN** the strike terms are 0.25, 1, 1 and 0, only 600 cm keeps closing, the close term past the limit is 0, the outside-fight strike term is 0 and its close term is doubled, 100 cm scores 1, and melee scores 0.5

## REMOVED Requirements

### Requirement: A fighter stands on the strike plateau
**Reason**: Its last sentence stated the duellist's interim walk (stand at 1.20 m, walk at 2 m), which the combat state replaces; the scoring rules move to "The strike plateau".
**Migration**: The duellist's footwork is the combat state's ("The cost of a place", "The attack decision").
