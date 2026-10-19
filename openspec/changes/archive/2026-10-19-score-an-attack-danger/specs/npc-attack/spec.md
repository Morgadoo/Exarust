# Spec Delta

## ADDED Requirements

### Requirement: A side swing scores danger
A side swing on its arc end at phase 0.5 SHALL score danger bits 0x40000000 at skill 0 and 0x40800000 at skill 1. Phase 0.35 SHALL score bits 0x3ff28f5c. Phase 0 and a sweep that is off SHALL score 0. A held weapon SHALL keep bits 0x40000000 past the limit. Outside a fight the strike SHALL be 0 and that danger SHALL stay.

#### Scenario: Edge at phase 0.5 and 0.35
- **WHEN** a side swing on its arc end is scored at phase 0.5 with skill 0 and skill 1, at phase 0.35, at phase 0, not sweeping, past the limit with a held weapon, and outside a fight
- **THEN** the danger bits are 0x40000000, 0x40800000 and 0x3ff28f5c, phase 0 and the sweep that is off score 0, the held weapon keeps bits 0x40000000, and outside a fight the strike is 0 and the danger bits stay 0x40000000

### Requirement: A thrust and a feral fighter score the place
A thrust at 100 cm SHALL score danger bits 0x3f000000 inside its arc and 0x40800000 outside it. At 200 cm danger SHALL be 0. Feral 1 SHALL halve the close term and score no danger. Dull SHALL keep the close term and score no danger.

#### Scenario: Thrust arc, feral and dull
- **WHEN** a thrust is scored at 100 cm inside its arc, at 100 cm outside it, and at 200 cm, and a side swing is scored at feral 1 and dull
- **THEN** the thrust danger bits are 0x3f000000 and 0x40800000, 200 cm scores 0, feral 1 halves the close term and scores no danger, and dull keeps the close term and scores no danger
