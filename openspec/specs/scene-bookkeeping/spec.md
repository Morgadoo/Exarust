# scene-bookkeeping Specification

## Purpose
The live scene's node-removal queue and the scene editor's frame step, as the original's machine code does them.

## Requirements

### Requirement: Removal queue
The scene bookkeeping model SHALL keep a 256-entry node-removal queue of which 255 are usable, and a full queue SHALL still mark the node and the sector's removal bits.

#### Scenario: A full queue
- **WHEN** a 256th removal is queued
- **THEN** the node and the sector's removal bits are still marked

### Requirement: Frame step
The scene editor model SHALL take the frame step as a signed 64-bit stamp difference compared in double with 16.666666666666668 and store a capped step as the float 0x41855555.

#### Scenario: A long frame
- **WHEN** a frame lasts longer than the limit
- **THEN** the stored step is the capped float
