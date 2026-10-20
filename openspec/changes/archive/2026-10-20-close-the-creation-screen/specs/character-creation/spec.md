## ADDED Requirements

### Requirement: Creation close
Closing SHALL, when the child is inactive, clear the stored byte and schedule the special form in mode 10, else the regular form. Otherwise it SHALL release the first present of `0x208`, `0x220` (also calling `0x2d`) and `0x218`, then `0x238` and `0x210`; outside mode 12 it SHALL release `0x228` and `0x1d0` and reset two words. It SHALL then tick, clear the active word, write the strings and clear the widget bit.

#### Scenario: Inactive and active
- **WHEN** the child is inactive and the mode is 10
- **THEN** the stored byte clears and the special form is scheduled
- **WHEN** the child is active, `0x208` is absent, `0x220` and `0x218` are present, `0x210` and `0x228` are present and the mode is 12
- **THEN** only `0x220`, `0x210` and the `0x2d` call run
- **AND** the widget reset does not run
- **WHEN** the mode is not 12
- **THEN** the widget reset and both later releases run
