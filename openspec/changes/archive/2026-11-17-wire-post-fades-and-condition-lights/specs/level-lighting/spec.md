## ADDED Requirements

### Requirement: Condition re-scale of placed lights
A placed object whose record is condition-scaled SHALL scale its voxel lights under the root to their records' intensities times (1 − c/255)² (double precision, cast once), with their colours recomputed, and its glow meshes' emissive by the same share, at spawn and again whenever its condition byte changes.

#### Scenario: A crystal spent to half
- **WHEN** a condition-scaled object's condition goes from 0 to 128
- **THEN** its voxel light's intensity becomes the record's times (127/255)² and its glow's emissive the same share
