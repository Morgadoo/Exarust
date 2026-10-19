## ADDED Requirements

### Requirement: Skin shader programs
With the guard clear the method SHALL return three programs in order: kind 5 with `MPrj` set to 2, kind 0x15 with `MPrj` set to 2 and a `Scale` lookup, and kind 5 with `MPrj` set to 2, `Tex` set to 0 and a `TexCol` lookup. It SHALL count six uniform operations. With the guard set it SHALL return none.

#### Scenario: The three programs
- **WHEN** the guard is clear
- **THEN** three programs are returned with kinds 5, 0x15 and 5
- **AND** the first sets `MPrj` to 2 and nothing else
- **AND** the second sets `MPrj` to 2 and looks up `Scale`
- **AND** the third sets `MPrj` to 2, sets `Tex` to 0 and looks up `TexCol`
- **AND** six uniform operations are counted

#### Scenario: The guard
- **WHEN** the guard is set
- **THEN** no program is returned and no uniform operation is counted
