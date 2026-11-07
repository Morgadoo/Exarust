## MODIFIED Requirements

### Requirement: Apparel condition
Apparel SHALL scale its resistances by the factor (each rounded product stored as a byte, so a product past 255 wraps, then capped at 160), its coverage by 0.3c + 0.7 (stored as a byte), divide its encumbrance by (1 − d·0.15/255)(q·0.3/255 + 0.85), then apply its armour variant's uncapped scales. The copies, the shop's copies, a repair's copy and the template migration SHALL use this one routine.

#### Scenario: Variant 4
- **WHEN** a pristine coat of slash 80 rolled variant 4
- **THEN** its slash is 140: 80 × 1.3 is 104, then × 1.35

#### Scenario: A byte past 255
- **WHEN** a pristine copy's template has padding 250 and coverage 240
- **THEN** its padding is 69, the low byte of 325, under the cap, and its coverage 6, the low byte of 262

### Requirement: Repaired copy refresh
After each write of a repair's timed wear byte the system SHALL refresh the live record when its id has a tier tag and its parent word names a record: it SHALL take the template's model price word; an apparel copy SHALL take the apparel condition from the template, priced on the template's quality; a weapon copy SHALL take the template's bytes 0x30 to 0x3F, run the factory stat pass (model name less its first letter) and its own condition, priced on its own quality.

#### Scenario: A repaired weapon copy
- **WHEN** a quality 255, damage 0 weapon copy is refreshed from a template whose side and thrust bytes are 100
- **THEN** its slash, pierce and thrust are 130 and its crush 121, and its adjust and skill bytes stay its own

#### Scenario: A factory weapon's repair
- **WHEN** a quality 255 copy of "@Sword" is repaired and the factory's stat pass writes slash 50
- **THEN** the pass is asked for "Sword" and the slash becomes 65
