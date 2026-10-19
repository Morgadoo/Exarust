## ADDED Requirements

### Requirement: Install body
A ready index below 1 SHALL return when the current index is already 0. Otherwise it SHALL clear that index, show a fallback when it differs and is not zero and a block word is 0, arm, and stamp the clock. A higher index SHALL return when it matches the sign-extended current index. Otherwise it SHALL store the index, show the record before it on that rule, and run its hook. A zero value, the same value, a block, and an index past the table SHALL not show. The platform call is counted.

#### Scenario: Index 1 shows its record and runs its hook
- **WHEN** a match begin installs index 1 with value 42 and a hook, the same index is installed again, index 2 has value 0 and a hook, and a block word refuses value 42
- **THEN** the first shows 42 and runs the hook, the second does not show or run it again, the zero value runs the hook without a show, and the block word leaves the shown value

#### Scenario: Index 0 shows the fallback
- **WHEN** a match begin installs index 0 while the current index is 5, the fallback is 9, and the clock is 12, and another begin is not ready
- **THEN** the current index is 0, fallback 9 is shown, the armed byte is set, the stamp is 12, and the unready begin leaves the index
