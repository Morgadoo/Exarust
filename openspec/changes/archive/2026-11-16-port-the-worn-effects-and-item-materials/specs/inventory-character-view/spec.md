## ADDED Requirements

### Requirement: Displaced value reads the class word
The character view's displaced value of a weapon drop SHALL read the body's weapon class word from what its held items made of it, guessing it from the main hand's grip only for a body without one.

#### Scenario: A light onto a two-handed body
- **WHEN** a light is weighed for a body whose class word is 0x20 holding a main and an off item
- **THEN** the main hand's item is weighed, not the off hand's
