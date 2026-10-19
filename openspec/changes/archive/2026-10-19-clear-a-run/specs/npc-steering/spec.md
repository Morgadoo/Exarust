# Spec Delta

## ADDED Requirements

### Requirement: Run and stop
Urgency of 0 or less SHALL set the run bit until the carrot fraction, as a double, is past 0.99 on the last segment. Past that the run bit SHALL clear, and a 3-D squared distance under 900 to the carrot SHALL stop. Urgency of 1 or more SHALL clear the run bit and stop under 10000 of the last point. Exactly 900 and 10000 SHALL not stop. A stop SHALL set the stopping bit, clear free walk and following, and pull a far carrot back by 8 times the speed plus 20 along the facing.

#### Scenario: The run bit clears, then a close body stops
- **WHEN** urgency 0 is not on the last segment, a fraction just under 0.99 is on the last segment, a fraction of 0.99 is 10 cm from the carrot, a fraction of 1 is exactly 30 cm from the carrot and another is 40 cm up, a standing body 100 cm above the carrot stops, urgency 1 is 99 cm from the last point and another is exactly 100 cm, and urgency -1 is not on the last segment
- **THEN** the first two set the run bit, 10 cm stops with the stopping bit and keeps the carrot, exactly 30 cm clears the run bit and does not stop, 40 cm up does not stop, the raised stop pulls the carrot 20 cm along the facing at the standing height, 99 cm stops, exactly 100 cm clears the run bit and does not stop, and urgency -1 sets the run bit
