# Spec Delta

## ADDED Requirements

### Requirement: Dropped weapon meets another body
A collapsing body's dying tick SHALL put a solid dropped weapon into the solver, and one shipped body step SHALL record a contact when that weapon's sphere starts on another body's sphere. The contact names both bodies and has one hit. The weapon leaves the place one step alone would leave it.

#### Scenario: Two spheres start together
- **WHEN** a collapsing body drops a solid weapon of velocity 1.5 whose sphere starts on another body's sphere, and the shipped body step runs once
- **THEN** the contact names both bodies, has one hit and deepest bits 0x3ec39cd9, and the weapon's first particle is at bits 0xbda3ae15, 0x3f7e43be and 0xbe492d0c
