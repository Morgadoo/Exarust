# Spec Delta

## ADDED Requirements

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
