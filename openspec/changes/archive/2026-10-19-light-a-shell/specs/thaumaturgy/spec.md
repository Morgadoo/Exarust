# Spec Delta

## ADDED Requirements

### Requirement: Shell light
A shell light SHALL follow the shown value. While opening, a shown value whose double is above 0.666 SHALL become one half, and any other SHALL become 0.75 minus the absolute gap from 0.75 to the shown value times 1.5. Any other state SHALL take half the shown value. The light SHALL then be one tenth of that. While closing, the gain SHALL become the shown value times 2.8. Any other state SHALL keep its gain.

#### Scenario: A bend, a flat half, and a close
- **WHEN** an opening light is taken at 0, at one half, at the float just under 0.666 and at the next float, a resting light at 1 is taken, and a closing light at one half is taken
- **THEN** the opening levels are 0, a tenth of 0.75, the bend, and a twentieth, the resting light is a twentieth and keeps its gain, and the closing light stores a gain of 1.4
