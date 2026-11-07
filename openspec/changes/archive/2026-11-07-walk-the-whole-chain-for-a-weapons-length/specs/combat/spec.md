## MODIFIED Requirements

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

#### Scenario: Length walk past a plain node
- **WHEN** a weapon's first-child chain is a rigid 50 cm part, a plain 5 cm node and a rigid 20 cm part, striking from 0 to 1
- **THEN** its length is 70 cm, the plain node passed over, and its sweet spot 30 cm, that walk stopping at the plain node

#### Scenario: Held by its character
- **WHEN** a tall human (size 1.05) in armour of encumbrance 0.5 holds the same sword
- **THEN** its reach is 120.6 cm, its recoil 0.67, its swing shaping 0.275 and its blend 8
