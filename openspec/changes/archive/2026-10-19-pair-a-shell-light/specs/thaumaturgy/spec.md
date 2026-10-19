# Spec Delta

## ADDED Requirements

### Requirement: Shell light pair
A shell's second light SHALL store two values. A resting state SHALL store 0.1 and 0.75. While opening, a shown value under 0.25 SHALL store one tenth of its times-4 product with 2 minus that product, and 1 minus that product times 0.25. At 0.25 or more it SHALL store 0.1 and 0.75. While closing, the first value SHALL be one fifth of 2 minus the absolute gap from 1.5 to the shown value times 3.5, and the second SHALL be the shown value times 0.75. A hold SHALL keep the pair it was given.

#### Scenario: Rest, open, hold, and close
- **WHEN** a resting pair is taken, an opening pair is taken at 0, at the float just under 0.25 and at 0.25, a hold keeps a pair, and a close is taken at 0 and at 1
- **THEN** the resting pair is 0.1 and 0.75, the open at 0 is 0 and 1, the float just under 0.25 is just under 0.1 and 0.75, the open at 0.25 is 0.1 and 0.75, the hold keeps its pair, the close at 0 is 0.1 and 0, and the close at 1 is 0 and 0.75

### Requirement: Shell index upload
A shell index upload SHALL ask for a header of 20 bytes plus 8 a channel, and a payload of the triangle count times the width. The target SHALL be 0x8893, the usage 0x88E4, and the access 0x88B9. A finish SHALL unmap only when the buffer name is above 0.

#### Scenario: Three triangles and a name
- **WHEN** an upload is asked for one channel, the triangle count of two skinned spans, and a width of 4, and a finish is asked for a name of 0 and a name of 1
- **THEN** the header is 28 bytes, the payload is 12 bytes, the target is 0x8893, the usage is 0x88E4, the access is 0x88B9, the name of 0 stays, and the name of 1 unmaps
