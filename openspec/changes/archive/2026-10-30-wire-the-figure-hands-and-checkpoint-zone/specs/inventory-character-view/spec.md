# Spec Delta

## ADDED Requirements

### Requirement: Figure hand scene
While the character view is open, each figure hand whose record item differs from the one its model shows SHALL get that item's model, when it has one, as a weapon (the hand's held-slot grip, flipped by `0x1000000`), a shield (left forearm frame at the hand's rest offset, −5° about z) or a light (its grip for that hand): old models released, new ones attached, the figure's held items set (its moveset follows), the stance restarted with blend 20. A hand left empty SHALL release its model. 
#### Scenario: Weapon and shield
- **WHEN** a one-handed axe and then a buckler are dropped on the figure
- **THEN** the axe hangs in the right hand by its grip and the buckler on the left forearm, and the figure's off hand shows a shield

#### Scenario: Two weapons
- **WHEN** a one-handed axe is dropped, then a second one on the view's right half by a player who knows Insight's technique 8
- **THEN** both hands show an axe, the second by the off hand's grip

### Requirement: Carried torch on the figure
A carried torch outside the record SHALL show on the figure in the hand the player holds it in when the record leaves that hand empty, else in the other hand when that one is empty.

#### Scenario: Weapon beside the torch
- **WHEN** a player carrying the torch in the right hand takes a one-handed axe
- **THEN** the axe replaces the torch in the right hand and the torch shows in the left, raised by the one-hander-and-torch moveset
