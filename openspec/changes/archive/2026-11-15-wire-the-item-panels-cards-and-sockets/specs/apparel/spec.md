## ADDED Requirements

### Requirement: Suede programs
The suede materials (ids 9 and 10) SHALL compile the suede family's programs (`suede_d`, `suede_s`, `suede_n`), not the leather family's.

#### Scenario: Material 9
- **WHEN** the programs of material 9 are listed
- **THEN** they are named suede and none is named leather

### Requirement: Alpha run-time material choice
A factory item's merged mesh SHALL take the sRGB-alpha run-time material `_procmatr` only when an area's material is Gemstone (23); Resin and every other material SHALL take `_procmat1`.

#### Scenario: Resin
- **WHEN** a factory weapon's areas use Resin (24)
- **THEN** its mesh names `_procmat1`
