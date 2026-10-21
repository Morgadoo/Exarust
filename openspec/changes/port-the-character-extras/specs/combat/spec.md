## ADDED Requirements

### Requirement: Main-side start sound
A free main-side attack start SHALL play its sound, unless the body's quiet byte is set, with the pitch (0.2·u + 0.8) / √scale from the first uniform draw and the gain 0.4·v + 0.6 from the second, each computed in double precision and stored as single.

#### Scenario: Draws of one half and one quarter
- **WHEN** the draws are 0.5 then 0.25 and the body's √scale is 2
- **THEN** the pitch is 0.45 and the gain 0.7
