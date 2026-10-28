# Spec Delta

## ADDED Requirements

### Requirement: Program cache keys
The program cache SHALL keep one bucket per material and shader word pair and, inside it, one entry per object and environment word pair. A build that fails SHALL be stored and not retried. Interning an existing pair SHALL only raise its reference count; a new pair SHALL take the first empty bucket met while scanning, or be appended.

#### Scenario: Same material, other shader word
- **WHEN** a material word is interned under two different shader words
- **THEN** it gets two buckets, and each builds its own programs

#### Scenario: Failed build
- **WHEN** a program build returns no program
- **THEN** a second request for the same words returns no program without building again

### Requirement: Shadow tap upload window
A sub-frame SHALL upload its `SdwSmp` taps starting at tap `pass · taps`, as four times the tap count in quadrant mode or the tap count otherwise, and one tap (four in quadrant mode) when the tap count is below one.

#### Scenario: Second pass
- **WHEN** two taps are used and pass 1 uploads
- **THEN** the window starts at tap 2 and holds two taps

### Requirement: Light slot gather
A draw SHALL gather at most the smaller of six and the view's light cap from its links in list order, stopping at a link with no id or a weight below 0.004 (a NaN weight passes). Unshadowed lights SHALL take the first slots and shadowed ones the rest, each keeping last frame's slot where it can.

#### Scenario: View cap
- **WHEN** eight links are offered with a light cap of two
- **THEN** two lights are gathered

### Requirement: Light slot values
A light new to its slot SHALL write its kind's position, its colour, the depth scale `(0.97 + E)/(far · scale)` and its filter radius. A light that keeps its slot SHALL refresh only a shaped light's radius and, on a scale change, its depth scale. A reset SHALL clear the light bits and the count and keep the written uniforms.

#### Scenario: Reset
- **WHEN** the slots are reset
- **THEN** the light bits and the count clear and the written uniforms stay
