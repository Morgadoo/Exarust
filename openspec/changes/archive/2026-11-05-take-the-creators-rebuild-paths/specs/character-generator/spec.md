# Spec Delta

## ADDED Requirements

### Requirement: Creator re-texture path
A change of the creation preview's record of only its skin words, hair colour bytes or beard bits (`0x00F00000`) SHALL keep the built figure and rebuild only its skin job.

#### Scenario: A skin tone edit
- **WHEN** the creator's skin tone changes and nothing else
- **THEN** the figure is kept and only its skin job is built again

### Requirement: Creator hair path
A change of the creation preview's record of only its hair style or hair-shading bit (`0x10000`) SHALL remove the figure's hair and grow it again, growing nothing when the race has no hair or the head word's bit 1 is set.

#### Scenario: A hair style edit
- **WHEN** the hair list selects another style
- **THEN** only the figure's hair is removed and grown again

### Requirement: Creator whole rebuild
Any other change of the creation preview's record SHALL rebuild the whole figure, which SHALL keep its motion and its turn.

#### Scenario: A physique edit
- **WHEN** the physique changes along with the skin tone
- **THEN** the whole figure is rebuilt, keeping its animator and turn
