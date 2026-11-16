## ADDED Requirements

### Requirement: Recorded differences kept aside
A function whose port is known to differ from the original on purpose SHALL keep its case file out of the replayed directory and SHALL be recorded with the count of differing cases, so the replay passes only when every replayed function is identical.

#### Scenario: The sun view matrix
- **WHEN** the sun view matrix's cases recorded with the processor's reciprocal are replayed against the port's exact reciprocal
- **THEN** they differ, and the file is kept aside with its count recorded
