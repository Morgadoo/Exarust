## ADDED Requirements

### Requirement: Inset bevel
At place 10, 20 of size 100 by 80 with inset 5 and unit 5, flag bit 0x40 SHALL draw the inner fill at 17, 27 of size 86 by 66 with slots 0x74 and 0x80, the border at 16, 26 of size 88 by 68 with slots 0x7c and 0x88, bind slot 0x6c, draw the outline at 15, 25 of size 90 by 70, and run the corner detail at 16, 26 of size 88 by 68 with mode 0xa0. Without bit 0x40 it SHALL draw one virtual rectangle at 15, 25 of size 90 by 70 with booleans from flag bit 0x10 and false.

#### Scenario: The bevel arm
- **WHEN** flag bit 0x40 is set
- **THEN** the four rectangles and the slot bind are those listed
- **AND** one prepare call is counted

#### Scenario: The plain arm
- **WHEN** flag bit 0x40 is clear and bit 0x10 is set
- **THEN** the first boolean is true and the second is false
- **AND** the rectangle is 15, 25 of size 90 by 70
- **AND** the extra height argument is 70
