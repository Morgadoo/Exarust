## ADDED Requirements

### Requirement: A removed searcher lets its object go
A searching brain that leaves the scene SHALL release the object its search held: the object's reservation count falls by one, once.

#### Scenario: One of two searchers despawns
- **WHEN** two searchers each hold an object and one is despawned
- **THEN** its object's count drops to 0 and the other's stays 1
