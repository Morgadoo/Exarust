## ADDED Requirements

### Requirement: Prop shove
`--shove-props <frame>` SHALL, at that frame after the load in play mode, deal every prop body within 3 m of the player a blow of 1.5 m/s away from it and 1.5 m/s up, log how many, and once all of them sleep again log after how many ticks and how far each moved. `--shove` SHALL deal its blows to character bodies only.

#### Scenario: Arena tavern table
- **WHEN** `exarust --at 11.2,4.5,12.8 --shove-props 120 play arenao1` runs
- **THEN** 8 prop bodies are shoved (the cups, bowls, tankards and spoons on the tavern table, which is level geometry) and all sleep again 236 ticks later, seven on the table and one 0.8 m lower on the floor

## MODIFIED Requirements

### Requirement: Body overlay
`--debug-bodies`, or F3 in any view, SHALL draw every simulated character body and every awake prop body over its mesh between its last two ticks: each part's particles and tetrahedron; each link from its point on the part to its point on the parent, green within its slack (1 mm tolerance), yellow up to 1 cm past it, red beyond; pivots as crosses; a living body's muscle anchors, faint. F4 SHALL draw only the inspected body, and F5 SHALL inspect the character body under the cursor.

#### Scenario: Standing duel
- **WHEN** `exarust --debug-bodies --opponent play arena` runs and both fighters stand
- **THEN** both are drawn over their meshes and their links are green

#### Scenario: Limp body
- **WHEN** `exarust --debug-bodies --limp character` runs
- **THEN** the falling body's parts and links are drawn on it as it falls and once it sleeps

#### Scenario: Off
- **WHEN** neither the flag nor F3 turned the overlay on
- **THEN** nothing is drawn and no measure is taken
