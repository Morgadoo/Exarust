# Spec Delta

## ADDED Requirements

### Requirement: Shield bars
A shield card SHALL show weight as that byte times 0.01 and impact as that byte times 0.05. It SHALL show encumbrance as that byte times 0.05, and SHALL omit it when the byte is 0. When the flags share a bit with 0x06000000, weight SHALL be 1 and encumbrance SHALL be 8.

#### Scenario: A zero burden stays hidden
- **WHEN** flags are 0 with weight 5, impact 20, and encumbrance 0, then encumbrance is 1, then flags are 0x02000000 with weight 5, impact 0, and encumbrance 0
- **THEN** the first weight is 0.05, impact is 1, and encumbrance is absent, the second encumbrance is 0.05, and the marked weight is 1, impact is 0, and encumbrance is 8
