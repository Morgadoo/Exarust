# input-bindings Specification

## Purpose
The player's key bindings: the original's 23-slot virtual-key table, how its modifier keys are read, and the file it is saved in.

## Requirements

### Requirement: Default bindings
The system SHALL provide the original's 23-slot virtual-key table: primary (left mouse), W, A, S, D, X, shift, alt, middle mouse, V, control, I, tab, K, R, F, J, T, C, P, Q, right arrow and left arrow. Shift, alt and control SHALL accept either side.

#### Scenario: Shipped table
- **WHEN** a player starts with default bindings
- **THEN** Cursor Look is V and Run is either shift

#### Scenario: Unbound key
- **WHEN** an action's code has no mapping
- **THEN** the action stays inactive

### Requirement: Binding file
The system SHALL save bindings as version 101, the controller-mode flag, the count 23 and the 23 little-endian key codes, and SHALL accept an older file whose word after the version is the count instead of the flag.

#### Scenario: Round trip
- **WHEN** a controller binding set is saved and loaded
- **THEN** the controller flag and all 23 codes are unchanged

#### Scenario: Old file
- **WHEN** a file has no controller flag after version 101
- **THEN** it loads with controller mode off and the 23 codes that follow

#### Scenario: Malformed file
- **WHEN** the version or count is wrong, or the file is truncated
- **THEN** loading returns no bindings
