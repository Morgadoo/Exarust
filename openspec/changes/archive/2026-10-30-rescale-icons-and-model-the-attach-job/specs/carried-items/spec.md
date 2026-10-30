# Spec Delta

## ADDED Requirements

### Requirement: Model attach job
Attaching a weapon's model SHALL light its part when the item flags carry 0x2000000, mark its meshes to cast shadows when the object record answers the 0x100 test, clear the body's weapon part when the class word has no 0x30 bits, and leave the take-in job. Attaching a shield SHALL clear the part on the same test, never light or mark shadows, and leave the node finaliser.

#### Scenario: Weapon and shield
- **WHEN** a weapon with flags 0x2000000 is attached to a body of class 0x10, and a shield to a body of class 0
- **THEN** the weapon lights, keeps the part and leaves the take-in; the shield clears the part and leaves the node finaliser

### Requirement: Take-in wait
The take-in job SHALL stay queued until the attached node carries 0x2000000 and its part 0x10000. A scheduled take-out SHALL exist only for a hand whose model slot is filled.

#### Scenario: Part not marked
- **WHEN** the node carries 0x2000000 but its part only 0xFFFF
- **THEN** the take-in job is not ready and stays queued

#### Scenario: Empty hand
- **WHEN** a take-out is scheduled for a hand with no model
- **THEN** none is made
