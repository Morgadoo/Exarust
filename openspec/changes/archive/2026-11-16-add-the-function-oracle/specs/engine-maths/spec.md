## ADDED Requirements

### Requirement: Denormals read as zero by the conversions
The floor and ceiling conversions SHALL read a denormal input as a zero of its sign, as the original's conversion does under denormals-are-zero.

#### Scenario: The smallest denormals
- **WHEN** the smallest positive denormal is ceiled and its negative floored
- **THEN** both give 0
