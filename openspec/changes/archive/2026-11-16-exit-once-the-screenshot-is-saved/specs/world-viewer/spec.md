## ADDED Requirements

### Requirement: Screenshot run ends after the save
A `--screenshot` run SHALL end once the shot has been captured and written, and SHALL end with a warning when no shot has arrived 600 frames after it was asked for.

#### Scenario: A fast offscreen run
- **WHEN** `exarust play arena --offscreen f.bin --frames 300 --screenshot s.png` runs at several hundred frames a second
- **THEN** s.png is written before the run exits
