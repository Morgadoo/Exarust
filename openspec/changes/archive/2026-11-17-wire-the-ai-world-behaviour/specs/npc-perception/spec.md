## ADDED Requirements

### Requirement: Mask Mind in the sight
A body whose masked flag is set (a Mask Mind or a worn mind ward) SHALL be seen as hidden, its sight radius shrunk by the two factors of its first power whose id's low word is 0x102 (the instant power 0.9 and 1, the persistent one 0.9 and 0.8), or by 0.9 and 0.8 with no such power.

#### Scenario: A blind observer and a masked player
- **WHEN** an observer of perception 0 stands 2 m from a masked player
- **THEN** with no Mask Mind power it does not notice the player, and with factors 0.5 and 0.5 it does

### Requirement: Contact event word
A contact's event word SHALL hold the other's mid-swing attack (1), the brain's engagement (2) and the concern's first reaching 1.2 (4) and 4 (8); the per-tick concern integration SHALL raise 4 and 8 through the same setter.

#### Scenario: The concern drifts past 1.2
- **WHEN** the integration raises a contact's concern from 1.19 past 1.2
- **THEN** the contact's event word has bit 4

### Requirement: Scene re-entry clears the seen counts
A brain moved back into the scene SHALL zero every contact's count of the ticks it has been seen.

#### Scenario: A brain re-enters
- **WHEN** a brain whose contact was seen 12 ticks re-enters the scene
- **THEN** the contact's count is 0
