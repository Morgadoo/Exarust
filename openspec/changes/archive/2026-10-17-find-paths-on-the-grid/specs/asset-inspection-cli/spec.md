## MODIFIED Requirements

### Requirement: Navigation grid inspection
`exa-tool nav <level>` SHALL build the level's navigation grid as the game does at load, without a window, and print its size, inputs, columns by span count and, per layer, its floor, passable and open-sky cells; `--png <file>` SHALL draw both layers side by side, a pixel per cell. With `--from` and `--to` (metres) it SHALL plan a path between them as the AI's planner does, print its points and draw it on the picture.

#### Scenario: exanima01
- **WHEN** `exa-tool nav exanima01 --png nav.png` runs against a retail installation
- **THEN** it prints 640 × 800 cells with 219292 lower floors and writes a 1284 × 800 picture of the dungeon's rooms, black over the rock between them

#### Scenario: A path in the arena
- **WHEN** `exa-tool nav arena --from 0,0,5 --to 0,0,-3` runs against a retail installation
- **THEN** it prints a path that reaches the goal, 87 cells met, cost 135, from (−0.15, 0, 4.95) to (−0.15, 0, −3.15)
