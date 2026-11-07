# Spec Delta

## ADDED Requirements

### Requirement: Role inspection
`exa-tool roles` SHALL print every role of the installation's `charroles.rdb` with its id, name, parent, weight and flags, and each of its attitude entries with its target (a role, one character in one locale, or anyone), attitude and tolerance.

#### Scenario: The demon role
- **WHEN** `exa-tool roles` runs against a retail installation
- **THEN** role 11 Demon lists attitude -100 toward role 1 Human and role 2 Undead, and 100 with tolerance 60 toward role 11 Demon

### Requirement: Power ids of character records
`exa-tool characters` SHALL print, after each record's count of known powers, the ids of those powers.

#### Scenario: The summoner
- **WHEN** `exa-tool characters exanima67` runs against a retail installation
- **THEN** the record GdnShaperD lists powers 0x10601 0x602 0x20603 0x604
