## ADDED Requirements

### Requirement: Span square
Stored word 4 at 10, 20 SHALL draw (3, 29), (3, 13), (19, 13), (19, 29). Stored word 6 SHALL draw (-1, 33), (-1, 9), (23, 9), (23, 33). Stored word -1 SHALL place the first corner at bits 0xdf000000, 0xdf000000. The cursor SHALL start at 0. Flag 4 and mark 9 SHALL store indexes 0 through 3 and finish at 4. Flag 0 SHALL store none and still finish at 4. Colour 17 SHALL be stored four times. The scale rebuild is not in.

#### Scenario: Word 4 draws the square
- **WHEN** a square is drawn at 10, 20 with stored word 4, colour 17, flag 4, mark 9 and cursor 99
- **THEN** the corners are those above, indexes 0 through 3 store 9, the cursor finishes at 4, and colour 17 is stored four times

#### Scenario: A clear flag still finishes at 4
- **WHEN** the same square is drawn with flag 0
- **THEN** no mark is stored, and the cursor finishes at 4
