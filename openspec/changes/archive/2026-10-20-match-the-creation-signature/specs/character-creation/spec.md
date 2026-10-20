## ADDED Requirements

### Requirement: Creation signature match
Matching SHALL be true when the query equals the signature's id. Otherwise the binding looked up by the query SHALL be required, its id SHALL equal the signature's id, and its four bytes SHALL equal the signature's four bytes.

#### Scenario: Direct, bound and missing
- **WHEN** the query equals the signature's id
- **THEN** the match is true
- **WHEN** the query differs, the binding's id equals the signature's id and all four bytes match
- **THEN** the match is true
- **WHEN** a byte differs
- **THEN** the match is false
- **WHEN** the binding's id differs
- **THEN** the match is false
- **WHEN** there is no binding
- **THEN** the match is false
