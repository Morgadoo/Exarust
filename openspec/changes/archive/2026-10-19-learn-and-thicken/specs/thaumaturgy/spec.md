# Spec Delta

## ADDED Requirements

### Requirement: Learned marks
Each node mark SHALL be the last known pair whose id, with bits 20 to 23 cleared, equals the power, or zero. The first node SHALL set bit 0x10000. A mark that already has bit 0x1000 SHALL keep bit 0x4000 and set bit 0x10000. Any other mark SHALL clear bit 0x4000, and SHALL set bit 0x10000 when the top nibble is not 7 and every link end, one-based, has bit 0x1000. A missing end SHALL not.

#### Scenario: An opened link and a held mark
- **WHEN** a lone node is marked, a known id with bits 20 to 23 set is followed by a later pair, a link to a closed node is marked, a link to an opened node is marked, a top nibble of 7 is linked to that opened node, and a mark of 0x5000 names an end that is not a node
- **THEN** the lone node is 0x10000, the later pair wins as 0x10002, the closed link stays 0, the opened link is 0x10000, the nibble of 7 stays 0, and the held mark is 0x15000

### Requirement: Thick joint
A thick joint SHALL place two points around the middle, one to each side of the chord from the first point to the third, at the given width. Width 2 through 0, 10 and 20 SHALL sit at 10, -2 and 10, 2 with a join of zero. A zero-length chord SHALL stay on the point.

#### Scenario: A straight joint and a still point
- **WHEN** a width of 2 is placed through 0, 10 and 20, and the same width is placed where all three points are 1, 2
- **THEN** the straight joint is at 10, -2 and 10, 2 with a join of zero, and the still point stays at 1, 2
