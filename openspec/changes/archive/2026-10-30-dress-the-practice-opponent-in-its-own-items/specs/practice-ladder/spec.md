# Spec Delta

## ADDED Requirements

### Requirement: Practice opponent's own items
The practice opponent's worn and held item references with the local bit SHALL name the items of its template's own database, any other reference the game's, and the opponent SHALL be dressed, armed and weighed from those items.

#### Scenario: The first opponent's own clothes
- **WHEN** opponent01 is read from the installed actors
- **THEN** it wears its own local items 2, 3 and 4, shown as `Trouser_A`, `Shirty_A` and `Sandals_B`, and the trousers cover the groin

#### Scenario: Local and game ids
- **WHEN** a template's own item 2 and the game's item 2 differ
- **THEN** a reference to 2 with the local bit names the template's item, one without it the game's, and a local id the template lacks names nothing
