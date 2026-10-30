## ADDED Requirements

### Requirement: Factory weapon migration
Migrating a weapon that takes its template's stat bytes and whose model section (visited before it) names a factory with a leading `@` SHALL hand the name after the `@` and the weapon body to the caller's factory pass when there is one. Then, with or without a pass, it SHALL scale the stat bytes by the weapon's own quality and damage (capped at 160) and rescale the model's value word with the weapon's own quality as the base.

#### Scenario: A factory weapon with no factory
- **WHEN** a weapon copy of quality 0 and no damage, whose template model is `@sword_lnga` with stat bytes 9, migrates without a pass
- **THEN** its slash, crush and pierce bytes are 7, 8 and 7 and its value word is rescaled by 1.25

#### Scenario: A plain weapon
- **WHEN** a weapon whose model name has no `@` migrates with a pass
- **THEN** the pass is not called and the stat bytes stay the template's

### Requirement: Factory pass write-back
The item side's factory pass SHALL, for a body with no stored choices and only its first seed, copy that seed to the second, set the stored and rerolled marks and write the initial choices (picked parts' selection bytes, the first three areas). For a random roll with a first seed it SHALL set the stored mark and write every part's selection byte and the first four areas. It SHALL then write the recipe's stat bytes.

#### Scenario: First seed only
- **WHEN** a body with flags 0 and seeds 0x12345678 and 0 passes through a factory with no parts
- **THEN** both seeds read 0x12345678, the flags are the stored and rerolled marks and the selection bytes are unchanged

### Requirement: Editor look roll
The object editor's look roll SHALL draw in order: a top bit choosing one of the families 0, 4, 8, 14, 19, 25, 28 or any of 32; the first shade 7 − round(6r²) from the unit draw r; the second colour (a family, shade 7 or 6); a third shade of 3 or 4; with the full flag, a one-in-five trim row (else 3, 0); the light byte; and a finish of 3–4 one time in three, else 83–85. It SHALL clear two editor bytes.

#### Scenario: Seed zero
- **WHEN** the roll runs with the full flag from generator state 0 and a unit draw of 0.36
- **THEN** the first colour is 6, the second 223, the light 1, the finish 84 and the state 0x5F4B0359
