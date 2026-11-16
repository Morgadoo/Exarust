## ADDED Requirements

### Requirement: Recorded cases replayed bit for bit
The oracle SHALL replay every case of a recorded case file against the port registered under the file's function name, under the engine's floating-point mode, and SHALL report the function as identical only when every case gives the recorded output bytes.

#### Scenario: A port that differs
- **WHEN** a case file records that the sign of -3 is 1 and the port answers -1
- **THEN** the replay reports that case as the first that differs, with the port's bits 0xBF800000

#### Scenario: A port that matches
- **WHEN** every case of a file gives the recorded bytes
- **THEN** the replay reports the function identical over the file's case count

### Requirement: NaN payloads counted apart
An output that is a NaN on both sides with other sign or payload bits SHALL be counted as a NaN-only difference and SHALL NOT fail the function.

#### Scenario: Two NaNs
- **WHEN** the original's output is the NaN 0x7FC12345 and the port's the NaN 0xFFC00000
- **THEN** the case counts as a NaN-only difference

### Requirement: Names without addresses
The registry SHALL name each original function by a name only; the case files, the recorder and the map from a name to the original's code SHALL stay outside the repository, and the replay test SHALL be ignored unless a case directory is given.

#### Scenario: No case directory
- **WHEN** the workspace tests run without a case directory
- **THEN** the replay test is ignored and every other test runs
