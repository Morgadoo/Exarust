## ADDED Requirements

### Requirement: Counter roll
A performance counter SHALL replace the day's roll before the six templates are chosen. Only the counter's low 32 bits are kept, and those bytes are reversed. Counter 0x04000000 SHALL choose the roll-4 row and end at 0xb761a41e. Counter 0 SHALL choose the roll-0 row and end at 0x1210d4e5. Bits above the low 32 SHALL not change that row.

#### Scenario: The counter replaces a roll of 99
- **WHEN** a roll of 99 is replaced by counter 0x04000000, by counter 0, and by counter 0x100000000 plus 0x04000000
- **THEN** the first and the third choose 0, 1, 6, 4, 4, 1 and end at 0xb761a41e, and the second chooses 0, 1, 4, 1, 2, 3 and ends at 0x1210d4e5

### Requirement: Daily fee scale
A placed match SHALL copy the template, then multiply the fee and the prize by one more than the slot. Bit 2 SHALL keep the slot as the tier. Without it the tier SHALL be the slot times one minus the fourth power of the unit, rounded nearest even. A negative tier SHALL become -1. Kind 8 SHALL show 0 and 5. Bit 4 SHALL step the roll and store one more than the high half of that word times the slot span. The spans are 8, 11, 10, 14, 4 and 4.

#### Scenario: Slot 3 quadruples the coins and kind 8 shows 5
- **WHEN** a fee of 3 and a prize of 12 are placed at slot 3 with bit 2, the same template of kind 8 is placed at slot 3, a tier of -4 is placed at slot 2 with bit 2, a clear flag at slot 4 is placed with units 0 and 1, and bit 4 at slot 0 is placed from roll 0x10000000
- **THEN** the fee is 12 and the prize is 48, the tier is 3, kind 8 shows 0 and 5, the negative tier is -1, unit 0 leaves tier 4, unit 1 leaves tier 0, and the extra is 3 at roll 0x50000001
