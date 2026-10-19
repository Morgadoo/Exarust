# Spec Delta

## ADDED Requirements

### Requirement: Door partner
A single shut door SHALL stay shut at exactly 0.05 rad short of closed, and SHALL leave once it is farther and not latched, while still being held. A paired door whose partner is not within 0.1 rad SHALL not be held, and SHALL leave when its own angle is short of closed. A near partner SHALL keep it shut and held. Exactly 0.1 rad short SHALL not count as near, and past closed SHALL. A paired door SHALL add the pair bits to its layer.

#### Scenario: One leaf open, the other not held
- **WHEN** a single door is exactly 0.05 rad short and then 0.06 rad short and unlatched, a paired door 0.01 rad short has a partner that is not near, a paired door has a near partner, an angle is exactly 0.1 rad short and another is past closed, and a paired layer starts at 0
- **THEN** the single door stays shut on the line and leaves past it while still held, the open partner's leaf leaves the shut state and is not held, the near partner's leaf stays shut and held, exactly 0.1 rad short is not near, past closed is near, and the layer is the pair bits
