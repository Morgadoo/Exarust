## ADDED Requirements

### Requirement: Resource names from project paths
A project path SHALL name the resource pack entry after its last backslash, or the whole path when it has none; a forward slash SHALL be part of the name.

#### Scenario: A material library path
- **WHEN** the project names `Resource\Common.rml`
- **THEN** the entry looked up is `Common.rml`

### Requirement: Drive test on the first colon
A project path whose first colon is past its first byte SHALL be taken as it is; any other path SHALL be joined to the project's folder.

#### Scenario: A leading colon
- **WHEN** the path is `:a:b`
- **THEN** it is joined to the project's folder
