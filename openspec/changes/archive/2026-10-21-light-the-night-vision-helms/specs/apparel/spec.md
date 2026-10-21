# Spec Delta

## ADDED Requirements

### Requirement: Sees in the dark
An item body whose special kind byte (at 0x214 of the body) is 1 SHALL be read as seeing in the dark, and a body too short to hold the byte SHALL read kind 0. In the installed object database exactly the three night-vision helms SHALL see in the dark.

#### Scenario: The byte and the corpus
- **WHEN** a body carries kind 1 at 0x214, a body ends before it, and the installed `objdb.rdb` is scanned
- **THEN** the first reads 1, the second 0, and the items of kind 1 are "UCA Helm A", "UGA Helm FB" and "UW Helm NV"
