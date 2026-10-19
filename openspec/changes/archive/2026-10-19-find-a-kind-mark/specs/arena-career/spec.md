## ADDED Requirements

### Requirement: Kind table
An item id of 0 SHALL not count a kind call. Any other id SHALL search the high table when bit 0x80000000 is set, else the wide table when bit 0x40000000 is set, else the plain table. A high or wide id SHALL drop those two bits before the search. A found row whose span holds 0x20000 SHALL count one kind call. A missing key, a size of 0, a short span, or a span without that mark SHALL not. A step of 0 stops the walk. The two calls after a hit are not part of this step.

#### Scenario: A plain id counts when the span holds the mark
- **WHEN** an open world picks item 7 whose plain row holds 0x20000, and another pick of 7 does not
- **THEN** the first counts one kind call and the second counts none

#### Scenario: The high bit chooses the high table
- **WHEN** id 0x80000005 has an unmarked plain key 5 and a marked high key 5, and id 0xC0000005 has those tables reversed
- **THEN** the first counts one kind call and the second counts none

#### Scenario: The wide bit drops itself before the search
- **WHEN** id 0x40000009 has a marked plain row of that whole id and an unmarked wide key 9, and the wide row is then marked
- **THEN** the first counts none and the second counts one kind call
