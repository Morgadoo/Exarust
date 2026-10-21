# Spec Delta

## MODIFIED Requirements

### Requirement: Binding file
The system SHALL save bindings as version 101, the controller-mode flag, the count 23 and the 23 little-endian rows of the saved table, in the Controls page's order: keyboard scan codes (bit `0x100` for an extended key) and `0x8000` plus a mouse button's virtual-key code, with row 21 left 0. Loading SHALL read the controller flag and then the count when the version is above 100, and otherwise the count with the flag off; SHALL let the count's rows overwrite the shipped saved table from its first row; SHALL clear row 21; and SHALL fill each slot from its row, a code with a `0xC000` bit giving its low 14 bits and any other its low byte's scan code mapped to a virtual-key code, the modifiers to their left keys' one-sided codes. Slot 0 SHALL keep the primary mouse button. A payload that ends before a word it needs SHALL load no bindings.

#### Scenario: Round trip
- **WHEN** a controller binding set is saved and loaded
- **THEN** the controller flag and every slot's key are unchanged, except that a right control or alt reads as the left one

#### Scenario: Shipped rows
- **WHEN** the shipped saved table is loaded
- **THEN** every slot holds the shipped table's key, the modifiers their left keys' own codes

#### Scenario: Old file
- **WHEN** a file of version 100 holds the count after the version
- **THEN** it loads with controller mode off and the rows that follow

#### Scenario: Malformed file
- **WHEN** a file's count is 2, or the file ends before its rows
- **THEN** the first loads two rows over the shipped ones and the second loads no bindings
