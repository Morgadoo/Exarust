# Spec Delta

## ADDED Requirements

### Requirement: List scissor switches
A list draw SHALL switch the scissor test on for a visible entry whose flag word has bit 0x1000_0000 and off for a visible entry without it, only when the bit differs from the last visible entry's, starting from off for the whole list; entries sharing no bit with the visibility mask SHALL change nothing.

#### Scenario: Entry walk
- **WHEN** entries with words 1, 0x1000_0001, 0x1000_0001, 0x1000_0002, 2, 1 and 0x1000_0001 are walked under mask 1
- **THEN** the switches are none, on, none, none, none, off and on

### Requirement: Particle pass masks and stages
The particle pass SHALL draw under the visibility mask 0x6000_0000 for a single view and 0x2000_0000 for a super-sampled pass, restoring the old mask after; it SHALL draw the first list of every particle sector depth-only, then the second list of every sector depth-only with each group's texture bound once, then per sector both lists in colour with depth writes off.

#### Scenario: Two sectors
- **WHEN** the pass covers two particle sectors
- **THEN** the draws are first-list depth for sectors 0 and 1, textured depth for sectors 0 and 1, then colour for sector 0's two lists and sector 1's two lists

### Requirement: Super-sampled resolve weight
Each super-sampled pass SHALL draw min(count, 4) sub-views and give its resolve the weight 1 / (4·pass + 1) in single precision.

#### Scenario: Eight sub-views
- **WHEN** the 8x level's schedule is built
- **THEN** both passes draw four sub-views and their resolve weights are 1 and 0.2
