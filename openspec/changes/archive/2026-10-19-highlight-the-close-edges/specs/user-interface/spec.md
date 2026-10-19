## ADDED Requirements

### Requirement: Close-edge highlight
With the still bit set, close mark -9 SHALL draw a strip from x0 plus one unit to x0 plus three units between y0 and y0 plus two units. Mark -10 SHALL draw the same strip below y0. Mark -8 SHALL draw a four-unit square at x0, y0. The tint SHALL use the tab sample and the colour 255, 204, 153, 255.

#### Scenario: The marks draw their tints
- **WHEN** the still bit is set, the close x is 10, the close y is 20, the unit is 5 and the close mark is -9
- **THEN** the strip is (16, 40), (16, 30), (26, 30), (26, 40)
- **WHEN** the mark is -10
- **THEN** the strip is (16, 20), (16, 30), (26, 30), (26, 20)
- **WHEN** the mark is -8
- **THEN** the square is (11, 50), (11, 30), (31, 30), (31, 50)

#### Scenario: Other marks draw nothing
- **WHEN** the still bit is set and the close mark is -4
- **THEN** no close-edge tint is drawn
