## ADDED Requirements

### Requirement: Factory emblem layers
The system SHALL paint an area's emblem as a layer after the area's own one, with the same mask, material and wear weight, the emblem's colour and the picture `emblembkg0` followed by the id's two digits; the layer SHALL paint only the colour pass, and with a painted-pattern material (1, 7, 8, 25) its mask SHALL be multiplied by the picture's red at the region coordinate times `PtrnPos.xy` plus `PtrnPos.zw`, otherwise it SHALL cover its whole mask.

#### Scenario: A cloth emblem
- **WHEN** a ClothBasic layer carries an emblem whose picture is opaque on its left half
- **THEN** the emblem colour shows on the left half only and the specular and normal maps are those of the base layer alone

#### Scenario: A material without the program
- **WHEN** a ClothFine layer carries the same emblem
- **THEN** the emblem colour covers the whole mask

### Requirement: Factory pattern position
The system SHALL start `PtrnPos` at (atlas width / height, 1, 0, 0) and, for every assembled part whose chosen variant has flag `0x1000`, multiply its scale by f = 1 − byte/254 (byte/127 when negative), move its offset by −(f − 1)/2 and then by f · offset byte/127.

#### Scenario: Half scale
- **WHEN** a 512 by 256 atlas has one placing variant with scale byte 127 and offset bytes 127 and −127
- **THEN** `PtrnPos` is (1, 0.5, 0.75, −0.25)

### Requirement: Blade texture stretch
The system SHALL take a paint region's `UVScale` from the last of its part's deforms whose header word is 6, as the deform factor of the negated value, else 1, and SHALL sample only blade layers' mask, shade and region normal at (u, v · UVScale).

#### Scenario: A stretched blade
- **WHEN** a texture-only deform of amount 0.5 has value 0.5
- **THEN** the region's `UVScale` is 0.8
