## ADDED Requirements

### Requirement: Daily match choice
Six daily matches SHALL be chosen from seven templates. Each try SHALL step the roll by multiplying by 0x08088405 and adding 1, and the high half of that word times 7 SHALL choose. A limit above 0 that is already reached SHALL refuse it. A slot before the template's earliest slot SHALL refuse it. A second step SHALL refuse it when the mask shares a bit with the roll's top three bits.

#### Scenario: Rolls 0 and 1 agree, and roll 4 does not
- **WHEN** the daily board is filled from rolls 0, 1 and 4
- **THEN** rolls 0 and 1 both choose 0, 1, 4, 1, 2, 3 and end at 0x1210d4e5, and roll 4 chooses 0, 1, 6, 4, 4, 1 and ends at 0xb761a41e
