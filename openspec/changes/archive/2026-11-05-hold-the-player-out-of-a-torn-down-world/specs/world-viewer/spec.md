# Spec Delta

## ADDED Requirements

### Requirement: Offscreen debug screenshots
In an offscreen game every debug screenshot (`--screenshot`, `--searcher-shots`) SHALL save the image the window's cameras draw into. A windowed game SHALL save its window. The search's screenshots SHALL be taken one a frame, in the order they fell due.

#### Scenario: Searcher shots in a grid quadrant
- **WHEN** a grid quadrant runs `play exanima01 --searcher 0x2b --searcher-shots <dir>` and the follower searches
- **THEN** each of its picks, settles and loots writes a numbered PNG of the quadrant's frame into `<dir>`
