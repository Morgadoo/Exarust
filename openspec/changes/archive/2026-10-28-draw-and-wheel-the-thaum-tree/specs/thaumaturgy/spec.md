## ADDED Requirements

### Requirement: Tree zoom about the cursor
Each drawn frame SHALL show the zoom `(value − previous)·share + previous` for the frame's share of a 17 ms step, SHALL rescale the pan's goal, position and previous point about the cursor by the shown zoom over the zoom shown the frame before, and SHALL show the pan blended from previous to position by the same share. Placement, the fixed zoom and the node hit tests SHALL use the shown zoom.

#### Scenario: Half a step
- **WHEN** the zoom spring is at 1.2 from 1.0, the last shown zoom is 1.0, the cursor is at (50, 60) and the pan's goal, position and previous point are (100, 200), (300, 400) and (200, 300), at half a step
- **THEN** the shown zoom is 1.1, the points become (105, 214), (325, 434) and (215, 324), and the shown pan is (270, 379)

### Requirement: Tree opening scale
While the window's or a domain's fade-in runs, the drawn nodes SHALL be scaled about the first node's placed point from before the frame's placement by `ease(s) = 1 + 0.111·s − 1/(8s + 1)`, `s` the running fade's `1 − r²`; otherwise the scale SHALL be one.

#### Scenario: Halfway through the opening
- **WHEN** the window fade-in is half run (`s = 0.75`)
- **THEN** the scale is 0.9403929

### Requirement: Tree first zoom level
The zoom level search SHALL compare the distance to the level just reached with the distance to the previous one and step back a level only when it is greater; below the first threshold the previous level is not a number, the compare is unordered and the first level (−8) SHALL answer.

#### Scenario: Below the first threshold
- **WHEN** the level is asked for 0.25, 0.26 and 0.29
- **THEN** each answers −8

### Requirement: Tree domain wheel child
The tree SHALL carry the domain wheel as a child at `(10, height − 290)`, 280 pixels square, with the opened domain as its choice, the focus share as its eased value (48 with no record) and the bar child's fade as its show. A click on one of its icons SHALL go through the wheel's confirm (known domain, different from the choice) and a confirmed domain SHALL start the tree's domain switch; a pick or drop SHALL start the wheel's 300 ms ease.

#### Scenario: Known and unknown domains
- **WHEN** a tree opened in Mind with a 700-pixel height is clicked at the Energy icon (150, 626) with no known domains, then at the Mind icon with all known, then at the Energy icon again
- **THEN** the first two clicks start no switch and the third chooses Energy and starts the 256 ms fade-out
