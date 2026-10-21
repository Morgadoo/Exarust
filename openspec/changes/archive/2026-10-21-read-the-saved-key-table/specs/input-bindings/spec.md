# Spec Delta

## MODIFIED Requirements

### Requirement: Binding file
The system SHALL save bindings as version 101, the controller-mode flag, the count 23 and the 23 little-endian rows of the saved table in the Controls page's order: scan codes (bit `0x100` when extended) and `0x8000` plus a mouse button's virtual-key code, row 21 left 0. Loading SHALL read the flag and then the count when the version is above 100, else the count with the flag off, and SHALL let the count's rows overwrite the shipped saved table from its first row.

#### Scenario: Round trip
- **WHEN** a controller binding set is saved and loaded
- **THEN** the controller flag and every slot's key are unchanged, except that a right control or alt reads as the left one

#### Scenario: Old file
- **WHEN** a file of version 100 holds the count after the version
- **THEN** it loads with controller mode off and the rows that follow

#### Scenario: Malformed file
- **WHEN** a file's count is 2, or the file ends before its rows
- **THEN** the first loads two rows over the shipped ones and the second loads no bindings, as no payload that ends before a word it needs loads

## ADDED Requirements

### Requirement: Saved rows to slots
After loading, the system SHALL clear row 21 and fill each slot from its row: a code with a `0xC000` bit gives its low 14 bits, any other its low byte's scan code mapped to a virtual-key code, the modifiers to their left keys' one-sided codes. Slot 0 SHALL keep the primary mouse button.

#### Scenario: Shipped rows
- **WHEN** the shipped saved table is loaded
- **THEN** every slot holds the shipped table's key, the modifiers their left keys' own codes
