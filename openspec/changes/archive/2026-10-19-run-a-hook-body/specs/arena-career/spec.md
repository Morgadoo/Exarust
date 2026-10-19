## ADDED Requirements

### Requirement: Gate hook
The gate hook SHALL clear two result words and set the result flags to 1. A set mid byte with an alternate hook SHALL count that hook. A clear mid byte SHALL leave the flags at 1 and SHALL not count it. The pick under the cursor is not part of this step.

#### Scenario: A set mid byte counts the alternate hook
- **WHEN** the gate hook runs with empty results, and again with a set mid byte and an alternate hook
- **THEN** both runs set the flags to 1, the first counts nothing, and the second counts the alternate hook

### Requirement: Watch hook
A watch without bit 0x2000 SHALL store a stamp of 0. With that bit, a target, and a ready byte, a held byte of 0 SHALL copy the clock. Code 1 SHALL clear bit 0x800000 when it is set and count a release. Code 2 SHALL set that bit and count a press. A pending count above 0 SHALL count a refresh. A leave with a focus SHALL clear the focus and enter the session when the mark is not 0. The hook SHALL return 1.

#### Scenario: A watch stamps, releases, and enters
- **WHEN** a watch without the bit has stamp 9, a ready watch with code 1 and the mark bit has clock 12 and a pending count, and a leave has a focus and mark 2
- **THEN** the first stamp is 0, the second stamp is 12 and the mark bit is clear, and the leave clears the focus and enters the session
