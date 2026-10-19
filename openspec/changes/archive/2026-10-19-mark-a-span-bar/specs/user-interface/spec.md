## ADDED Requirements

### Requirement: Span mark
Bit 4 of the flag SHALL store the mark word once per corner, at the cursor, and the cursor SHALL then advance by one. Twelve corners from cursor 3 with mark 9 SHALL use indexes 3 through 14 and finish at 15. Flag 5 and a flag with the high bit and bit 4 set SHALL still store them. Flag 0 and flag 1 SHALL store none and still finish 12 ahead. Cursor 4294967294 SHALL store 4294967294, 4294967295 and 0, then finish at 10. An unset flag and mark read as 0. The other line-class methods are not in.

#### Scenario: Cursor 3 stores twelve marks
- **WHEN** unit 5 draws a span from 10, 20 with span 100, stored word 4, an unset view, flag 4, mark 9 and cursor 3
- **THEN** indexes 3 through 14 each store 9, and the cursor finishes at 15

#### Scenario: A clear bit still advances
- **WHEN** the same span is drawn with flag 0 and with flag 1, mark 9 and cursor 3
- **THEN** no mark is stored, and the cursor finishes at 15
