# Spec Delta

## MODIFIED Requirements

### Requirement: Older layouts
A `0xDBAC0D08` database SHALL have its operative sections converted to the 60-byte layout (class and kind through the original's tables, a relay keeping its kind). A `0xDBAC0D07` one SHALL first convert its weapon sections to 96 bytes; a `0xDBAC0D06` one SHALL before that convert its bodies and random tables. Each conversion SHALL replace the old section by an appended new one; unknown magics SHALL be errors.

#### Scenario: An old key
- **WHEN** a `0xDBAC0D08` record holds an operative section of class 2 and kind 3
- **THEN** after loading it holds a 60-byte operative section of class 4 and kind 7 as its last section

#### Scenario: A body of the middle layout
- **WHEN** a `0xDBAC0D07` record holds a body and a random table
- **THEN** both are left as they were

### Requirement: Packed fields
The packed setters SHALL round to nearest with ties to even and OR their values in unmasked as the original's. The signed field SHALL keep the count bits, clear bits 2–7, store round(|31v|) above the count, a magnitude past 31 setting the sign bit, then the sign; the three-level fractions of byte 3 SHALL store n = round(8v), above 1 as (n >> 1) + 1 capped at 5 (3 in the two-bit field).

#### Scenario: Half a step
- **WHEN** the signed field is set to 0.5
- **THEN** it holds 16 (15.5 rounded to even), not 15

#### Scenario: A value past the field
- **WHEN** the signed field is set to 1.5
- **THEN** its byte is `0xB8` and it reads back negative

#### Scenario: A positive value after a negative one
- **WHEN** a field holding count 3 and a negative value is set to 1.0
- **THEN** its byte is `0x7F` and it reads back 1.0

## ADDED Requirements

### Requirement: Oldest body conversion
The `0xDBAC0D06` body converter SHALL keep the blocks below `0x1A0`, copy the power word from the old bytes `0x1AC`–`0x1B3`, map the old category to the armour type (1–8 to 1, 2, 3, 1, 2, 4, 5, 6; others keep the byte), apply the category's finish changes when the finish is 1–7 without bit 7, draw one paint seed per body as a 63-bit value modulo `0xFFFFFFFF`, and turn the four old 16-byte parts into the first four appearance layers.

#### Scenario: An unknown category
- **WHEN** an old body's category byte is 9
- **THEN** the converted armour type is 9

### Requirement: Oldest random-table condition words
After the bodies, each random-table row's condition word SHALL be rewritten by the setters applied in turn to the same word. An unrolled word (bit 7 clear) SHALL become `0x5284`. A rolled one SHALL take wealth 1, then the damage read from the updated word as the sign-magnitude value of bits 0–2 plus 2 quarters, then the spread from bits 4–6 of the word after that, then dirt 0.5 and spread 0.25.

#### Scenario: A rolled condition word
- **WHEN** an old row's condition word is `0x00F0`
- **THEN** it becomes `0x57B4`

### Requirement: Use action and experience table
Using a class-0 object SHALL hand its header name to the reader for kind 7 and close the item container and open the divination-card form for kind 5; other kinds and classes do nothing. The experience amount for a level SHALL be 10, 20, 50, 100, 200, 500, 1000 and 2000 for levels 0–7 and 2000 above.

#### Scenario: A letter
- **WHEN** an object of class 0 and kind 7 is used
- **THEN** its header name goes to the reader

### Requirement: Placed light share precision
A condition-scaled item's placed light share SHALL be (1 − c·(1/255))² computed in double precision and cast once to single, c being bits 16–23 of the placement flags.

#### Scenario: Condition 27
- **WHEN** a condition-scaled item is placed with condition 27
- **THEN** its share has the bits `0x3F4CA884`
