## MODIFIED Requirements

### Requirement: Attach pass
The system SHALL hang the main hand's weapon and then one off-hand item: a shield from either hand first, else hand 1's torch, else hand 1's weapon. A weapon-section item whose hand has `0x2000000` SHALL start on its flipped side, per item; the last weapon-section item's `0x1000000` SHALL set the character's half-sword hold. A level's living placed characters and the scene bodies SHALL hang the shield or second weapon the pass chooses.

#### Scenario: Shield in the main hand
- **WHEN** hand 0 holds a shield and hand 1 a sword
- **THEN** only the shield is hung, on the off arm

#### Scenario: Hand flags
- **WHEN** hand 0 holds a sword flagged `0x2000000` and hand 1 a dagger flagged `0x1000000`
- **THEN** the sword starts flipped, the dagger does not, and the character holds half-sword
