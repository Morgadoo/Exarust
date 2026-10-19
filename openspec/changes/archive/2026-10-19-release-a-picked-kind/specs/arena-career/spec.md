## MODIFIED Requirements

### Requirement: Kind table
An item id of 0 SHALL not count a kind call. Any other id SHALL search the high table when bit 0x80000000 is set, else the wide table when bit 0x40000000 is set, else the plain table. A high or wide id SHALL drop those two bits before the search. A found row whose span holds 0x20000 SHALL count one kind call. A missing key, a size of 0, a short span, or a span without that mark SHALL not. A step of 0 stops the walk.

#### Scenario: A plain id counts when the span holds the mark
- **WHEN** an open world picks item 7 whose plain row holds 0x20000, and another pick of 7 does not
- **THEN** the first counts one kind call and the second counts none

#### Scenario: The high bit chooses the high table
- **WHEN** id 0x80000005 has an unmarked plain key 5 and a marked high key 5, and id 0xC0000005 has those tables reversed
- **THEN** the first counts one kind call and the second counts none

#### Scenario: The wide bit drops itself before the search
- **WHEN** id 0x40000009 has a marked plain row of that whole id and an unmarked wide key 9, and the wide row is then marked
- **THEN** the first counts none and the second counts one kind call

## ADDED Requirements

### Requirement: Kind release
A cursor below 0, or at the view width or height, SHALL clear the hit and SHALL not count a kind call. A cursor inside SHALL keep a supplied hit. A marked item SHALL then release: a count above 0 drops and becomes 0, and a count of 0 or less clears a pointer, counting a close when the byte is 1. With no matching pane a marked span counts a build. The first pane tagged 0xce1c2013 for that item is shown, and a later pane is not. The scene walk and the pane builder are not part of this step.

#### Scenario: A cursor on the view edge does not release
- **WHEN** the cursor is at width 8 on a view of 8, the item is marked, and the held count is 2
- **THEN** the hit is clear, the kind call is not counted, and the count stays 2

#### Scenario: A marked item drops a count and shows the first pane
- **WHEN** a count of 3 is released, a pointer with byte 1 is released, and a view corner shows the second of two tagged panes
- **THEN** the count becomes 0 with one drop, the pointer is cleared with one close, and pane 11 is shown once
