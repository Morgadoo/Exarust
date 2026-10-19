# Spec Delta

## ADDED Requirements

### Requirement: Shell ripple
A shell ripple SHALL lose 0.04 times its scale from the strength. The shown value SHALL then move toward that strength by at most 0.06 times the scale upward and 0.04 times the scale downward. Below 0.05 on both it SHALL be gone.

#### Scenario: A held shell and a faint one
- **WHEN** shown 1 and strength 1 are stepped at scale 1 and at scale 2, shown 0.09 and strength 0.09 are stepped at scale 1, and shown 0.05 and strength 0.05 are stepped at scale 1
- **THEN** scale 1 is 0.96, scale 2 is 0.92, the 0.09 pair stays at 0.05, and the 0.05 pair is gone
