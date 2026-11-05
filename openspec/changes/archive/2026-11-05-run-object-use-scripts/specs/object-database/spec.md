# Spec Delta

## ADDED Requirements

### Requirement: Operative use script
An operative section's body SHALL keep, after its fixed words, a use script: its length at `0x38`, read signed, and its text from `0x3C`; a length under 1 or past the body's end SHALL give none. In `objdb.rdb` of the installed build exactly one record holds one: the PedestalHand (`0x587`, class 2 kind 3, sound `DeviceA01`), `WitnessRole(Operative, $E1D1);`.

#### Scenario: The pedestal hand's body
- **WHEN** an operative body of class 2, kind 3 with sound `DeviceA01` and a 27-byte text at `0x3C` is read
- **THEN** the item's use script is that text; with the length 0 or −1 it has none
