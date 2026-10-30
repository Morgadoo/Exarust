# Spec Delta

## ADDED Requirements

### Requirement: Blend walk branches
The blend walk SHALL send a rule's whole weight to its first child when the input is ordered and at or below the low bound, to its second child when ordered and at or above the high bound, and otherwise (in range, or a NaN) split it, walking the second child with w·t before the first with (1 − t)·w.

#### Scenario: NaN input
- **WHEN** a rule with bounds 0 and 2 reads a NaN input
- **THEN** both children receive NaN weights

### Requirement: Blend walk start gate
A painted layer's walk, and the remainder's, SHALL start only when its weight widened to double is above the double 0.001, and no weight inside a walk SHALL be gated. The remainder SHALL start at 1 and lose each painted layer's weight in turn, skipped layers included.

#### Scenario: Small layer
- **WHEN** a layer weighs 0.0009 and another walk weighs the f32 0.001
- **THEN** the first is not walked but still leaves the remainder 0.9991, and the second is walked and its halves of 0.0005 reach the materials
