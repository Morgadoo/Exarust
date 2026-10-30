## ADDED Requirements

### Requirement: Repaired copy refresh
After a repair's timed write the system SHALL refresh a made copy from its template: the copy SHALL take the template's model price word (doing nothing when only the copy has a model section); an apparel copy's protection, coverage and encumbrance SHALL be the template's at the copy's quality, damage and variant, its price based on the template's quality; a weapon copy SHALL take the template's weapon bytes 0x30 to 0x3F and be scaled by its own condition, its price based on its own quality.

#### Scenario: A repaired weapon copy
- **WHEN** a quality 255, damage 0 weapon copy is refreshed from a template whose side and thrust bytes are 100
- **THEN** its slash, pierce and thrust are 130 and its crush 121, and its adjust and skill bytes stay its own
