## ADDED Requirements

### Requirement: Edge bars
After the four corners, the same style SHALL draw edge bars. With no title, unit 5 on a form of 100 by 80 SHALL draw a top bar from 33, 0 to 67, 10. A title of width 40 SHALL split that bar at gap 17 and add two caps. A style that is not 0x10, and bit 0x400000, SHALL draw no edges. The drawing after those bars is not in.

#### Scenario: A top bar runs from 33 to 67
- **WHEN** unit 5 dispatches a form of width 100 and height 80 with style bits 0x10 and no title
- **THEN** the top bar runs from 33, 0 to 67, 10, and a title of width 40 splits the top at gap 17
