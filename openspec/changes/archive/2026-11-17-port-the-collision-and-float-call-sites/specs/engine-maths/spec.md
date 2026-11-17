## ADDED Requirements

### Requirement: Engine turns at the posing call sites
The foot pitch SHALL turn the foot by the engine's X turn (its fast sine and cosine) composed by the engine's 4×3 product, and the limp's physical turns SHALL convert matrices with the engine's matrix-to-quaternion routine.

#### Scenario: A pitched foot
- **WHEN** the sampler pitches a foot by its smoothed angle
- **THEN** the foot's frame is the engine's X turn then its local frame then the shin's, by the 4×3 product

### Requirement: Ring directions by the engine's rotation
The particle ring's sources and an emitter's spawn ring SHALL take their direction as +X turned about +Y by the engine's axis-angle routine.

#### Scenario: A ring source
- **WHEN** a ring source is placed at angle θ
- **THEN** its outward axis is the engine's rotation of +X about +Y by θ

### Requirement: Tree frame colour by the engine unpack
The thaum tree's frame colour SHALL unpack the accent's three colour bytes as each byte times the stored 1/255, then scale them by 0.4.

#### Scenario: An accent byte of 51
- **WHEN** the accent's red byte is 51
- **THEN** the frame colour's red is (51 × the stored 1/255) × 0.4, bit for bit
