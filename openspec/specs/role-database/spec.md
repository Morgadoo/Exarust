# role-database Specification

## Purpose
The role database (`charroles.rdb`): role records, their entries and sections, and dialogue graphs (also `narrator.rcd`).

## Requirements

### Requirement: Role records
The system SHALL read every `charroles.rdb` role record into its id, flags, name, parent, weight, init script, variables (name and type word) and entries, and SHALL refuse a database whose magic is not the role database's. This SHALL hold for every record of the installed `charroles.rdb`.

#### Scenario: The shipped database
- **WHEN** the installed `charroles.rdb` is read
- **THEN** 49 roles are read, Explorer (31) among them, and Golem Srv (46) has the init script `Actor.Activity = Role.Activity;`

#### Scenario: A wrong magic
- **WHEN** a database's first word is not the role database magic
- **THEN** reading it is an error

### Requirement: Entry sections
The system SHALL read an entry's target, condition and value (attitude in byte 0, tolerance in byte 1) and its sections in the stored order: the unknown block, the dialogue graph, the encounter script, two unknown blocks, the interaction condition and the game-event line, each present when its kind bit is set.

#### Scenario: A dialogue and an interaction condition
- **WHEN** an entry of kind 0x1042 aimed at role 0x1E holds a graph and the text `result = 1;`
- **THEN** its attitude and tolerance are 100, it has a dialogue aimed at 0x1E and its interaction condition is `result = 1;`

### Requirement: Dialogue graphs
The system SHALL read a dialogue graph (a role entry's or `narrator.rcd`) into its target word, actors, and nodes in order, an empty node being a single zero word, each node with its flags, title, editor position, links and nested branch groups, each branch with its flags, lines (speaker slot and text), condition, action and replies. Nodes SHALL be found by title without case over all 16 bytes. Nesting deeper than 64 levels and any size past the data SHALL be errors.

#### Scenario: The narrator
- **WHEN** the installed `narrator.rcd` is read
- **THEN** it has five nodes, the first titled Corlian whose opening branch has three replies

#### Scenario: Nested replies
- **WHEN** a node's branch has a reply whose own reply has flags 0x31
- **THEN** that innermost branch's link is 1

#### Scenario: Truncation
- **WHEN** a graph is cut short anywhere
- **THEN** reading it is an error, never a panic
