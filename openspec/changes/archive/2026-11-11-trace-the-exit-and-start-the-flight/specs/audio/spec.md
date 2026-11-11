## ADDED Requirements

### Requirement: Pitched voice resampling
A voice whose rate (its buffer's rate times its pitch) and the output's rate reduce to a ratio whose products pass 32 bits SHALL be resampled like any other, without walking more input frames than the ratio asks for.

#### Scenario: A 100157:44100 ratio
- **WHEN** Constant Displacement's phasing sound plays at pitch 1.068 into a 44.1 kHz output (the ratio 100157:44100)
- **THEN** the audio thread keeps its period and the output stream drops at once at exit
