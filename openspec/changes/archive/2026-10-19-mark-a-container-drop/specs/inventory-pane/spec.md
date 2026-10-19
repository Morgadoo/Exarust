# Spec Delta

## ADDED Requirements

### Requirement: Marked drops
A pane SHALL refuse an item that shares a bit with its reject mask. It SHALL refuse an item whose flags, masked by the required mask, are not the required value. An own-only pane SHALL accept only an id it already lists with the low bit set. A limit of 0 SHALL not refuse for size. A capacity of 0 SHALL not refuse for bulk.

#### Scenario: A mark, a list, and a zero limit
- **WHEN** a marked item is dropped, an item without the required bit is dropped, an unlisted id is dropped into an own-only pane, a listed id with the low bit is dropped, and a large item is dropped into a limit of 0 and a capacity of 0
- **THEN** the marked item is refused, the missing bit is refused, the unlisted id is refused, the listed id lands, and the large item lands
