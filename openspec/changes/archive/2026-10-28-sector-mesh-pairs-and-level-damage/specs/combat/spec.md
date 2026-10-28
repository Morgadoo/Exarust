# Spec Delta

## ADDED Requirements

### Requirement: Level impact damage
A character's level impact SHALL count on its part, and once the root's sound clock is past, take the limb curve (a third of the strength unless the character is down, then half; silent under 0.5), sound at the character, and over a volume of 0.25 deal the limb impact damage, daze and collapse a body's blow deals, then set the clock; a hard impact on a striking part in an overhead swing SHALL be dropped before the clock is set. A level impact SHALL NOT tell any other character.

#### Scenario: A hard fall
- **WHEN** a character's ball part drops from 5 m onto a floor
- **THEN** it sounds once, its yellow health falls when the volume passes 0.25, no limb impact message is sent and the root's clock is set
