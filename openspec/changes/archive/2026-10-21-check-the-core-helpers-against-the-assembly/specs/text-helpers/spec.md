# Spec Delta

## ADDED Requirements

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
