# Spec Delta

## ADDED Requirements

### Requirement: Tree lines
A node SHALL sit at ten times its place times the zoom, plus the pan, and round to the nearest even integer unless dragged. A factor SHALL scale that point about the origin. Both marks at zero SHALL draw nothing. Otherwise each end leaves its node along the sine and cosine of its turn times a full circle, by 0.514 or 0.295 minus 0.003 times the zoom, times the zoom rounded to an even 256. The smaller reach applies when the power high mark is set. A zero source mark SHALL dim the line.

#### Scenario: A snapped node and a dim line
- **WHEN** a node at 2, -1 is placed at zoom 1 with no pan and snapped, a node at 0.25 is placed the same way and again while dragged, a factor of one half scales 20, -10 about the origin, and a link from a large node at the origin to a small node 400 to the right is drawn at zoom 1 with both turns at zero, the source mark zero and the dest mark one, and again with both marks zero
- **THEN** the snapped node is at 20, -10, the quarter node snaps to 2, 0 and stays at 2.5 while dragged, the scaled point is at 10, -5, the line starts at the large reach above the origin and ends at the small reach above 400 in 19 points and is dim, and the unmarked link draws nothing
