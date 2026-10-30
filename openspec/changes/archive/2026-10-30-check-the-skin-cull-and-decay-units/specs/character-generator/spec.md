# Spec Delta

## MODIFIED Requirements

### Requirement: Running skin composite
The merged skin run SHALL take the character's composited colour, surface and normal maps, generated once per distinct look (stem, tone, side, layers, beard and decomposition; the look seed's offsets only with the decomposition pass) and read from the race maps at the target's own coordinates. Until the composite is ready, the run SHALL draw the race maps plus tone.

#### Scenario: Cache key
- **WHEN** two jobs differ only in tone, side, a layer amount, a beard or the decomposition offsets
- **THEN** their keys differ, and an equal job repeats its key

## ADDED Requirements

### Requirement: Culling append
The merge SHALL leave out a garment part's triangle whose per-triangle flags meet the append's cull bits, appending only the vertices a kept triangle uses, and SHALL give a part without flags the unset word `1` once the merged mesh has a flag stream. The body's own slots SHALL be appended without cull bits.

#### Scenario: Flagged triangle
- **WHEN** a part of two triangles flagged `0x200000` and 0 is appended with cull bits `0x200000`
- **THEN** only the second triangle and its three vertices are merged, and with cull bits `0x400000` both stay

#### Scenario: Shipped garments
- **WHEN** the shipped garments' drawn groups are scanned
- **THEN** 1 091 groups of 54 models carry a cull mark, and no slot whose two cull masks are zero carries `0x400000`

### Requirement: Decomposition in the composite
A composite job SHALL carry the decomposition pass when its feral level is above zero, with the record's two skin-map offsets. The pass SHALL first draw a mask, `humanseams` red times `humandcmpw` red at the offset times the amount, smoothed twice and stored as a byte, then replace the three base programs with the race's colour or gloss at `Tex0`, the mask at `Tex1` (and `Tex5` for the normal pass) and the job's tone as `SknCol`.

#### Scenario: Mask byte
- **WHEN** both mask maps are white and the amount is 0.5
- **THEN** the mask texel is 128

#### Scenario: Living job
- **WHEN** the feral level is zero, absent or NaN
- **THEN** the job carries no decomposition and the maps are the chargen ones
