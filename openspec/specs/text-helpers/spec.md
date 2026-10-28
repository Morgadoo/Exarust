# text-helpers Specification

## Purpose
The engine's text and number helpers: integer, hex and float formatting, the parsers, the substring search and the name, article and title-case helpers, as the original's instructions compute them.

## Requirements

### Requirement: Engine integer formatting
The engine's integer formatter SHALL build digits from the least significant, giving "-" for the most negative word; the two-digit formatter SHALL pad one digit with a leading zero and give "0" for a negative; the hex formatter SHALL pad with zeros only when the digits are fewer than the width, after the sign.

#### Scenario: Integers and hex
- **WHEN** −1234, 9 (two digits) and 0x2A (hex, width 4, then −0x2A width 4) are formatted
- **THEN** the texts are "-1234", "09", "002A" and "-002A"

### Requirement: Engine float formatting
The float formatter SHALL convert through 64-bit words, truncating the fraction digits and rounding the last to nearest even, carry a last digit of ten through trailing nines, keep or trim trailing zeros at the caller's request, and give the rounded integer for fewer than one decimal. The frame-rate readout SHALL keep its two decimals' zeros.

#### Scenario: Floats
- **WHEN** 1.2 is formatted with three decimals keeping zeros, 9.9995 with three keeping zeros, 2.5 with none and 3e9 with none
- **THEN** the texts are "1.200", "10.000", "2" and "-1294967296"

### Requirement: Engine text helpers
The substring search SHALL take a 1-based start and return a 0-based position or −1; the floor log2 SHALL treat its argument as signed (a top bit set gives 0); the a/an test SHALL look only at a lower-case first vowel; title-casing SHALL raise a lower-case letter after a space or underscore and change nothing else; the name helpers SHALL work on 16-byte names and split paths at backslashes only.

#### Scenario: Search
- **WHEN** "bc" is searched for in "abcdbc" from 3 and from 6
- **THEN** the results are 4 and −1

### Requirement: Engine fixed names and string compares
A sixteen-byte name SHALL be zero-padded; an ANSI text longer than sixteen bytes SHALL keep fifteen bytes and store 0x85 in the sixteenth. Byte compares SHALL answer the unsigned difference of the first differing pair; the ANSI compares SHALL answer the length difference when the common part is equal, the case-insensitive one comparing raised letters. The ANSI-string lowering SHALL lower every byte but the last.

#### Scenario: Ellipsis past sixteen
- **WHEN** the ANSI text `0123456789abcdefgh` becomes a name
- **THEN** its first fifteen bytes are `0123456789abcde` and the sixteenth is 0x85

#### Scenario: Case-insensitive ANSI compare
- **WHEN** `ABC` is compared with `ab` and `a` with `Abcd`
- **THEN** the answers are 1 and -3

#### Scenario: Lowering stops before the last byte
- **WHEN** `AbC\xE9 Z` is lowered in place
- **THEN** it reads `abc\xE9 Z`
