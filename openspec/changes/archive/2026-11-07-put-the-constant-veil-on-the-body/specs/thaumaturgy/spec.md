## ADDED Requirements

### Requirement: Constant displacement start on the body
A Constant Displacement cast SHALL give the caster's body parts collision bit `0x4000`, a phasing hit veto with a fresh streak and the restore hook, the body wisps, and the `thmdsp_cns01` hum starting silent at pitch 1, looping, `(length · 0.8) · r` seconds in (double). The veto, the bit and the hook SHALL stay after the power ends.

#### Scenario: Cast
- **WHEN** the player casts Constant Displacement
- **THEN** its body parts carry bit 0x4000 and its body carries a fresh phasing veto

### Requirement: Constant displacement hum and end
The hum SHALL follow the body at the power's volume; the power's end SHALL let it fade and free itself and SHALL end the body wisps (strands frozen, fade-out started).

#### Scenario: End
- **WHEN** the power ends
- **THEN** the hum fades out and the body wisps trail away and leave

### Requirement: Phased blow
A blow the veto passes SHALL play `thmdsp_phs` at the body at volume 0.6 and the drawn pitch, give the striker and its descendants bits `0xC000`, hang a one-strand tendril from the striker at the contact and list the striker. The restore hook SHALL give a listed striker its bits back and end its last tendril once `stamp + 500 < now`.

#### Scenario: A blow passes through
- **WHEN** a blow on a displaced body is passed by its veto
- **THEN** the striker's parts carry bits 0xC000 and a tendril hangs from it
- **AND** 504 ms later the bits are gone and the tendril is ending

### Requirement: Dragged link reach
A link dragged out of a tree node SHALL keep its hover while `(0.55 · zoom)²` in double is strictly greater than the squared distance from the node, and otherwise (a not-a-number included) SHALL drag the link, keeping its angle in radians; its curve SHALL start at the engine's sine and cosine of that angle and its far tangent SHALL run toward the pointer; its drop SHALL be taken at the last moved point and store the angle over 2π.

#### Scenario: Leaving the node
- **WHEN** the pointer moves from 55 to 56 units from a node at fixed zoom 100
- **THEN** the drag passes from the hover to the link
