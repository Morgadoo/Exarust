# Spec Delta

## ADDED Requirements

### Requirement: Race skin maps only for races that ship them
A character's body SHALL be re-skinned with its race's skin maps only when the race's look word has the men's or the women's texture bit; any other race SHALL keep its model's own materials and SHALL not look the maps up.

#### Scenario: The ogre keeps its skin
- **WHEN** the arena's Beast is built
- **THEN** no `ogredifm`, `ogrespcm` or `ogrenrmm0` is looked up
