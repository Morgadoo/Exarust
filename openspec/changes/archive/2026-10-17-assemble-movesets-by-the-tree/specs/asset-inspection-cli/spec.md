## ADDED Requirements

### Requirement: Moveset inspection
`exa-tool rsq` SHALL number the motion set's movesets and name the moveset each parent resolves to. `exa-tool clips <base> [config]` SHALL print the clip table a character with that base moveset and held-item config (hex or decimal) plays: every action with the moveset, and its number, its clip comes from; it SHALL fail when no moveset bears the base's name.

#### Scenario: Fael with a sword
- **WHEN** `exa-tool clips Fael 0x1` runs against a retail installation
- **THEN** Walk comes from `Demon A01 #23` and the attacks from `One Handed #24`

#### Scenario: Numbered tree
- **WHEN** `exa-tool rsq` runs against a retail installation
- **THEN** moveset #16 `1H & Shield` is shown under `One Handed #15`
