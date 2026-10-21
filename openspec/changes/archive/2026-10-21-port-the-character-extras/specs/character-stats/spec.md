## ADDED Requirements

### Requirement: Race voice templates
The race build SHALL expand each voice template by writing `m` and `f` for `{S}`, dropping any other `{…}` group, copying the character after a group untested and lowercasing the rest; only the first template SHALL set the gendered flag (it has `{S}`) and the numbered flag (its expanded name does not end in a digit).

#### Scenario: A numbered gendered template
- **WHEN** the first template is `VH{S}`
- **THEN** the names are `vhm` and `vhf`, gendered and numbered

### Requirement: Race voice names
A voice's sound names SHALL be the slot's male name, or its female name for a female voice of a gendered race, then, when numbered, two digits of the voice number plus one (first builder) or the voice number (second builder) with the tens digit unreduced, then `_GruntS`, `_GroanS`, `_GroanH` or `_Scream`, cut at 15 characters; only the first builder SHALL share the short groan for a missing heavy one, else the heavy for a missing short one.

#### Scenario: Voice 99
- **WHEN** the first builder names voice 99 of `vhm`
- **THEN** its grunt is `vhm:0_GruntS`

### Requirement: Built-in race record
A race id the database lacks SHALL take the executable's built-in record: no name or flags, the `human` stem, no shared moveset and `Default` as its own, every body factor 1, no gaits, 1 m of unarmed reach and health 1.

#### Scenario: A missing race
- **WHEN** race 9 is looked up in a database holding only race 1
- **THEN** the race has an empty name, the `humanbase.rfc` model and does not bleed
