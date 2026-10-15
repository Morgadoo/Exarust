## ADDED Requirements

### Requirement: Post-processing sharpen
The system SHALL sharpen the frame in the final post pass as the original does with supersampling on: an unsharp mask from four diagonal half-pixel samples, weighted by their contrast, before the grading curve.

#### Scenario: Floor detail
- **WHEN** a textured floor fills the view
- **THEN** its fine detail is as crisp as in the original
