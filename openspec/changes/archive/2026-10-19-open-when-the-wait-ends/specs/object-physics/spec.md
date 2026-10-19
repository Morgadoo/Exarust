# Spec Delta

## ADDED Requirements

### Requirement: Open after the wait
When a signalled wait ends it SHALL open the door. The open SHALL clear the latch, the shut flag and the pull count. A hinge that is not motorised SHALL go idle and be shoved 3 cm along local Z. A motorised hinge SHALL set its fraction to 0 and drive open for 100 ticks, with the same shove. A sliding door that is not motorised SHALL jump to fraction 0.6 for 80 ticks and wake. A motorised sliding door SHALL drive open for 300 ticks, keep its fraction, and wake.

#### Scenario: The 120th tick opens
- **WHEN** a latched plain hinge, a latched motorised hinge at fraction 0.4, a latched plain slider, and a latched motorised slider at fraction 0.4 each finish a signalled wait
- **THEN** the plain hinge is idle, unlatched and shoved 3 cm, the motorised hinge is at fraction 0 for 100 ticks and shoved 3 cm, the plain slider is at 0.6 for 80 ticks and awake, and the motorised slider stays at 0.4 for 300 ticks and awake
