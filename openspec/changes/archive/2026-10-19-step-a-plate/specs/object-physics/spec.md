# Spec Delta

## ADDED Requirements

### Requirement: Plate travel
A pressed plate SHALL raise each travel by 0.02, at most 1, and a free plate SHALL lower each by 0.01, at least 0. A one-shot SHALL stop being pressed once a travel passes 1, and later travels that tick SHALL fall. The mean SHALL be written back to every travel. Grind speed SHALL be 0.9 times the mean's change plus 0.1 times the old speed, and SHALL stop at 0.001 or below. Otherwise volume SHALL be 50 times the speed, squared, and pitch SHALL be 20 times the speed plus 0.8, at most 1.

#### Scenario: A rise, a grind, and a one-shot
- **WHEN** a pressed plate starts at travel 0, and a one-shot plate with travels 0.99 and 0.5 is pressed
- **THEN** the first mean is 0.02 with grind volume 0.81 and pitch 1, and the one-shot is no longer pressed with both travels equal

### Requirement: Plate click
A mean above 0.7 SHALL press a plate that was up, click, and signal 1, or -1 when inverted. A mean below 0.5 SHALL release a plate that was down and signal 0, unless inverted. A travel of 0.68 that rises SHALL not press. A travel of 0.51 that falls SHALL not release.

#### Scenario: The click and the two edges
- **WHEN** a pressed travel of 0.68 steps, a pressed travel of 0.69 steps, a down plate falls from 0.51, and a down plate falls from 0.5
- **THEN** 0.68 stays up, 0.69 clicks and signals 1, 0.51 stays down, and 0.5 releases and signals 0
