# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Unarmed hand mobility
When the equip pass finds no weapon class, each hand's mass word SHALL be its base times 0.5, or 0.25 with heavy claws.

#### Scenario: Heavy claws
- **WHEN** a hand's base is 2 and the race has heavy claws
- **THEN** its mass word is 0.5
