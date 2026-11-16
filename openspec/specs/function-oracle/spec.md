# function-oracle Specification

## Purpose
Automatic checks that a port matches the original: recorded calls of the original's functions (inputs and outputs as raw bytes) replayed against the Rust ports, bit for bit.

## Requirements

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

### Requirement: Recorded differences kept aside
A function whose port is known to differ from the original on purpose SHALL keep its case file out of the replayed directory and SHALL be recorded with the count of differing cases, so the replay passes only when every replayed function is identical.

#### Scenario: The sun view matrix
- **WHEN** the sun view matrix's cases recorded with the processor's reciprocal are replayed against the port's exact reciprocal
- **THEN** they differ, and the file is kept aside with its count recorded

### Requirement: Stubbed matrix routines record their arguments
A spec SHALL be able to stub a routine that builds or reads a matrix through the x87 unit, answering its result from a buffer and recording its arguments into an out buffer, so the caller's own arithmetic is checked bit for bit.

#### Scenario: The head look
- **WHEN** the head look runs with its Euler routines stubbed
- **THEN** the kept yaw and the angles it builds the matrix from replay identical over 5000 cases
