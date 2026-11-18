## ADDED Requirements

### Requirement: Garment paint runs
A dressed character's merge SHALL group every `&` part of one worn garment into one run drawn with that garment's painted material, and SHALL keep two garments' `&` parts in separate runs even when their material names match; every other part SHALL be grouped by its material name.

#### Scenario: Two garments with the same paint name
- **WHEN** two worn garments both have an `&cloth` part with different paints
- **THEN** the merged mesh has two runs, each drawn with its own garment's paint
