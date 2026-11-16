## ADDED Requirements

### Requirement: Skin maps follow the texture detail
The skin job of a game body SHALL be 512 texels a side below texture detail 3 and 1024 from 3, read from the texture settings when the body is built.

#### Scenario: The retail detail
- **WHEN** a body is built at texture detail 3
- **THEN** its skin maps are 1024 a side
