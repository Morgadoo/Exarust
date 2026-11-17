## MODIFIED Requirements

### Requirement: Footwork step gate
In combat mode the footwork SHALL lay no step unless the body may act: none of the bits 0x82C1 in the condition word, a muscle tone of at least 0.7, a support above 0.7 compared in double precision, and no trailing-foot request. It SHALL lay none while a swing is under way or the held movement word is zero. Energy is not read.

#### Scenario: The gate
- **WHEN** W is pressed while the condition word is walking, or the tone is 0.69, or the support is 0.5, or a trailing foot is pending, or a swing is under way
- **THEN** no step starts
