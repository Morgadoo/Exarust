## ADDED Requirements

### Requirement: Material providers from the last
The material lookup SHALL ask the project's material libraries from the last registered to the first.

#### Scenario: Two libraries
- **WHEN** the project lists Common then Underworld0
- **THEN** Underworld0 is asked first

### Requirement: Manager record for a model's materials
A model's rigid-body divisor and sound set, its GI thinness and its split flags SHALL read the material manager's record: a library's over the model file's own record of the same name, the file's only for a name no library holds.

#### Scenario: A name in both
- **WHEN** a model file and a library both define a material name
- **THEN** the library's record decides
