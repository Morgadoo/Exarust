# Spec Delta

## Purpose

The practice ladder: which sparring partner comes next, and the health restored between fights.

## ADDED Requirements

### Requirement: The next practice partner is named and the fighter heals
The next practice fight SHALL advance the count by one and stop at 8. Novice count 1 names opponent01 and count 8 names opponent08. Expert count 1 names opponent09 and count 8 names opponent16. Count 8 is the last fight, and a count already at 8 stays. Red SHALL gain a tenth of the maximum, in double precision, stay within the maximum, and yellow SHALL copy red. Red bits 0x3e4ccccd with maximum 0.25 become 0x3e666667. Red bits 0x3e75c28f become 0x3e800000.

#### Scenario: Novice count 0 and expert count 7
- **WHEN** a novice fight at count 0 with red bits 0x3e4ccccd and maximum 0.25 advances, the same novice advances from 7 and from 8 at full health, and an expert advances from 0 and from 7 with red bits 0x3e75c28f
- **THEN** the first is count 1 named opponent01 and not last, with yellow and red bits 0x3e666667, the second is count 8 named opponent08 and last, the third stays count 8 at bits 0x3e800000, the fourth is named opponent09, and the fifth is count 8 named opponent16 and last, with bits 0x3e800000
