## ADDED Requirements

### Requirement: Motion library fallback
Without `humanoid.rsq` the motion library SHALL be one unnamed action and one `Default` moveset without a parent or clips, with no bones; Exarust SHALL take the same library when the file does not parse.

#### Scenario: No file
- **WHEN** the library is loaded without `humanoid.rsq`
- **THEN** no character finds a clip

### Requirement: Child sort by absolute x
The loader's child sort SHALL fill each slot with the first child, from that slot on, whose absolute x is strictly below the best so far (starting at 3.4e38), the best index starting at 0, so that a slot whose remaining children are all NaN swaps with the first child.

#### Scenario: A NaN child
- **WHEN** children with x NaN and 1.0 are sorted
- **THEN** they end in the order NaN, 1.0

### Requirement: CPU vertex skinning
The CPU vertex skin SHALL blend the joints' 3×4 matrices by weight, starting with the first joint and stopping at the first joint number below 1, and SHALL transform the point once, each component summed as (x·a + y·b) + (z·c + t).

#### Scenario: A stopped blend
- **WHEN** a vertex at (1, 1, 1) has joints 1, 0, 2 with weights 1, 0, 1 and joint 1 translates by (1, 0, 0)
- **THEN** it skins to (2, 1, 1)
