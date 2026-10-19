# Spec Delta

## ADDED Requirements

### Requirement: Shell section slots
A kept shell vertex SHALL store its section and the corner index minus that section's base. The first link in its half-open range whose mark is not 255 supplies the section and the corner. A later link SHALL be ignored. A vertex with none SHALL be absent. A short list or a missing base SHALL be an error. A base of -1 subtracted from 1 SHALL store 2.

#### Scenario: Two slots and a gap
- **WHEN** three vertices are slotted, a later link is ignored, a base of -1 is subtracted from 1, a short corner list is slotted, and a missing base is slotted
- **THEN** the slots are section 0 local 1, absent, and section 2 local 2, the later link is ignored, the local index is 2, the short corner list is an error, and the missing base is an error
