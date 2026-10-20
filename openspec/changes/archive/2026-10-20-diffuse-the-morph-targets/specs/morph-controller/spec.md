## ADDED Requirements

### Requirement: Morph target diffusion
Target diffusion SHALL give every unknown vertex with known neighbours the weighted mean of those neighbours at 1/√(distance squared), mark it known for the next pass, and repeat until a pass changes nothing.

#### Scenario: A chain and a stranded vertex
- **WHEN** vertex 0 is known and vertices 1 and 2 form a chain at squared distance 4
- **THEN** two passes fill vertices 1 and 2, both taking vertex 0's delta
- **WHEN** no vertex is known
- **THEN** no pass changes anything and the deltas stay
