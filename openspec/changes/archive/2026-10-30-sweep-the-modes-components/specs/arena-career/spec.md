# Spec Delta

## ADDED Requirements

### Requirement: Arsenal lend release per box
The company arsenal's lend query SHALL clear the mask from the first entry holding the item with a mask bit in each box of the item's shelf, and SHALL answer whether any box did.

#### Scenario: Two boxes lent
- **WHEN** boxes 0 and 3 each hold the item with 0x200 and box 3 holds a second copy with 0x200
- **THEN** the query answers true, both first copies read 0 and box 3's second copy keeps 0x200

### Requirement: Loadout arsenal drop write-back
An item dropped back to the arsenal SHALL ask the company arsenal's lend query only when the form's own arsenal list held no entry of the item lent under the form's mask, and SHALL be written back as a new flagless entry only when it is of another category than the form's and neither had lent it.

#### Scenario: The form's own lend
- **WHEN** the form's list holds item 7 lent under 0x400 and the company arsenal also holds it lent
- **THEN** the form's entry loses 0x400 and the company arsenal's entry keeps it
