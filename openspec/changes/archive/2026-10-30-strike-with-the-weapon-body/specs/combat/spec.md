# Spec Delta

## MODIFIED Requirements

### Requirement: Weapon contacts
The system SHALL register at most one contact per attack and target. A held weapon that is its own body SHALL register one where its body touches a muscled limb of another character while its strike window is open, at the contact point and on the blade side of the grip. A striker without a body of its own (a fist) SHALL register one when its segment passes within a body part's volume. The blow reads the swing's clock, the clip's clock less its blend over its last key.

#### Scenario: Strike lands
- **WHEN** an opponent's sword strikes the player's body while its window is open
- **THEN** one hit on the touched body part is reported at the contact point

#### Scenario: The weapon is stopped by what it strikes
- **WHEN** a woken golem's halberd meets the player's body
- **THEN** the halberd does not pass through the body, and the hit is where the two touch

## ADDED Requirements

### Requirement: Weapon contact precedence
A weapon body's wound SHALL be refused while a clash or block waits for the damage routine and on the hand holding the struck character's shield. Within one tick a clash or block SHALL replace a pending wound, and a wound SHALL replace nothing.

#### Scenario: A clash takes precedence
- **WHEN** in one tick a striking weapon touches the defender's arm and then the defender's held weapon
- **THEN** the clash is the contact the damage routine deals
