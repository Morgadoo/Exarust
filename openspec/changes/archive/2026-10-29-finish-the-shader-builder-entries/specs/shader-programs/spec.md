# Spec Delta

## ADDED Requirements

### Requirement: Global time lanes
The `Global` block SHALL write `ViewPos.w` and `TimeDlt.x` as the sub-frame offset times the frame scale, and `TimeDlt.y` as `−(1/factor)` times the frame scale. The sub-frame offset SHALL be 0 outside super-sampling and `0.5 − (i + 0.5)/N` for sub-frame `i` of `N`; the frame scale SHALL be 1 for a frame of at most 1/60 s and `16.666…/ms` otherwise. The exposure SHALL NOT enter these lanes.

#### Scenario: Super-sampled slow frame
- **WHEN** sub-frame 1 of 4 is drawn after a 25 ms frame with factor 8 and exposure 2
- **THEN** `TimeDlt` is `(0.125 · s, −0.125 · s)` with `s = 16.666…/25`

### Requirement: Shadow declaration callback
The builder's separate shadow-declaration callback SHALL declare one `Shadow<i>` per light slot, a 2-D shadow sampler for a slot whose kind has `0x300 == 0x200` and a cube shadow sampler otherwise, testing every slot's own kind and declaring no `vrSdw` input.

#### Scenario: Sun in slot 1
- **WHEN** three slots hold point, directional and point lights
- **THEN** `Shadow0` and `Shadow2` are cube samplers and `Shadow1` is a 2-D sampler

### Requirement: Shadow tap table
The `SdwSmp` table SHALL be the original's eight three-float taps at their exact single-precision values, and the sun shadow filter SHALL use each tap's first two lanes.

#### Scenario: First tap
- **WHEN** tap 0 is read
- **THEN** its bits are `0x3E8B8A8F`, `0xBF0C745B`, `0x3EB5B2CD`

### Requirement: Texture upload wait
The material loader's wait SHALL scan its map slots from slot 0 while each holds a texture, at most five, start every scanned texture still uploading, and poll until no scanned slot is uploading.

#### Scenario: Gap in the slots
- **WHEN** slots 0 and 1 hold textures, slot 2 is empty and slot 3 holds one
- **THEN** only slots 0 and 1 are scanned
