# Spec Delta

## ADDED Requirements

### Requirement: Wake a sleeper
A sleeper whose flags are exactly the dormant flag, and that has seen a contact, SHALL drop role 44 and be knocked out at the current clock. It SHALL be down, keep the dormant flag, gain the waking flag, clear the statue mark, set gait 0, set tone to 0 and the pin gain to 0.08, and raise its sleep countdown to 256. Any other flags, or no contact, SHALL leave it sleeping.

#### Scenario: A contact knocks the sleeper out
- **WHEN** a sleeper with only the dormant flag sees no contact, then sees one at clock 2500, and a sleeper whose flags also include the shock stagger sees a contact
- **THEN** the first stays dormant with role 44 until the contact, which drops role 44, knocks it out at 2500, leaves it down with the dormant and waking flags, clears the statue, sets gait 0, tone 0 and pin gain 0.08, and raises the countdown to 256, and the stagger sleeper stays dormant with role 44
