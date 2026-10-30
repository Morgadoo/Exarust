## ADDED Requirements

### Requirement: Particle quad node frame
The system SHALL, for a template with flag 0x40, express the screen and stretched quads' camera axes in the emitter node's frame (each axis dotted with the node's three axes) and, with flag 0x1000, add −0.9 times (in double precision) each axis' component along the node's up axis back onto it and renormalise it by the approximate reciprocal square root; a quad mode other than screen, stretched or diamond SHALL draw nothing.

#### Scenario: A node turned a quarter turn about its z axis
- **WHEN** a template with flag 0x40 draws under a node turned 90° about z and the camera's up is (0, 1, 0)
- **THEN** the quad's up axis is (1, 0, 0)

### Requirement: Particle first-frame bit
The system SHALL clear an emitter's first-frame light bit after every frame, independently of the at-rest bit that clears when the intensity reaches the template's base.

#### Scenario: Two dark frames
- **WHEN** an emitter's light stays below its base for two frames without brightening
- **THEN** only the first frame asks for a full light update

### Requirement: Legacy fire key path
The system SHALL scale keys 1 to n of a fire key table on the legacy path by encoding each channel to sRGB (linear up to 0.0031308), moving it 0.35 toward its luma (0.299, 0.587, 0.114), scaling it by 0.75 and storing a saturated byte rounded half to even.

#### Scenario: An orange key
- **WHEN** the key (128, 64, 0, 32) is loaded on the legacy path
- **THEN** it becomes (127, 103, 36, 32)

### Requirement: Arc pass clock
The system SHALL give each supersample pass of an arc's draw the clock ((pass + frame + 0.5)·0.001)/divisor + start − 0.0005, the counters added as integers, the 0.5 in single precision and the rest in double.

#### Scenario: The second pass of frame two
- **WHEN** pass 1 of frame 2 draws from a start of 0.5 with divisor 1
- **THEN** its clock is 0.503

### Requirement: Device arc fields
The system SHALL read a level device's arc from its object record's active bit, duration, length, branches and flag byte, and SHALL give the arc a fixed end point the record's length along the node's third axis only when the flag byte has bit 0x02.

#### Scenario: A flagged device
- **WHEN** a device at (1, 2, 3) with third axis (0, 0, −1) has length 240 and flag 0x02
- **THEN** its arc ends at (1, 2, −237)

### Requirement: Arc discharge clear
The system SHALL clear an electrified item's arc effect on a discharge when the item's record is gone or the charge step reaches the limit, and SHALL spawn the spark burst at the queued point in every case.

#### Scenario: A missing record
- **WHEN** a discharge hits an item whose record is gone
- **THEN** the charge is unchanged, the effect clears and the burst appears
