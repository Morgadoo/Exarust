# Spec Delta

## Purpose

Resolves weapon attacks between fighters so the game's combat loop can be
played, as the base for the physically driven combat of the original.

## ADDED Requirements

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

### Requirement: Damage and reactions
The system SHALL reduce the target's health by an amount that grows with the square of the blade tip's speed, play a stagger reaction, and play a fall into the unconscious pose when health runs out.

#### Scenario: Knock-down
- **WHEN** a fighter's health drops to zero or below
- **THEN** it falls, stays down, and is no longer attacked

### Requirement: Duellist AI
The system SHALL drive an opponent to face its target, approach to striking distance and attack at intervals with varied strikes, using the same animation and attack paths as the player.

#### Scenario: Opponent engages
- **WHEN** `exarust play arena --opponent` starts
- **THEN** the opponent closes in and attacks the player

### Requirement: Characters block each other
The system SHALL keep character bodies from passing through each other.

#### Scenario: Close quarters
- **WHEN** two characters move into each other
- **THEN** their capsules stop at contact
