## ADDED Requirements

### Requirement: Skill key move
Moving SHALL step the index by the key's low mask. With a high selection it SHALL step back one, or forward while below two. Without one it SHALL scan the grid back from the index less two, or forward from the index less one, for the first zero and select the slot after it. A positive moved index different from the old SHALL be published.

#### Scenario: Simple, backward and forward
- **WHEN** the high selection is positive, the index 3 and the key portion `0x1001`
- **THEN** the index becomes 2 and 2 is published
- **WHEN** the high selection is absent, the index 4 and grid slot 2 is zero
- **THEN** the index becomes 3
- **WHEN** the high selection is absent, the index 4 and grid slot 4 is zero
- **THEN** the index becomes 5

### Requirement: Skill key dispatch
A masked key SHALL run the base call, or the hint with a 200 hide and bit `0x40` cleared. Key `0x100` SHALL store the high or low slot and the scanned index, publish it, hint, hide for 300 and set bit `0x40`, then run the closing, cursor and clear steps. Any other repage SHALL end. The result SHALL be 0.

#### Scenario: Base, key 0x100 and closing
- **WHEN** the key has bit `0x100` and the high selection is 4
- **THEN** the base runs and 4 is published
- **WHEN** the key is `0x100`, the repage 1 and the index 5
- **THEN** the high slot is 5, 1 is published, the hint runs, the hide is 1, the deadline is clock plus 300 and bit `0x40` is set
- **WHEN** the repage is 0
- **THEN** nothing further runs
- **WHEN** the closing word is 1
- **THEN** the handler ends before the cursor and clear steps
