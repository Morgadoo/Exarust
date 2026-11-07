# Spec Delta

## ADDED Requirements

### Requirement: Plain parts of a character
A body model node that carries its own body chunk and has no skeleton bone SHALL join the character's body as a plain rigid body: its stored spheres and links (with their strength and smoothing, soft welds where the link flag says so), links that are never muscles (the init's tone of 0.01), and its registered mobility divided by the character init's limb mass factor `n / ((20·s³ + Σ) · d)` (Σ the skeleton parts' volumes over 10⁴ cm³).

#### Scenario: The demon's tail hangs
- **WHEN** a demon is built
- **THEN** its seven tail parts and two wings are plain parts, and its eighteen skeleton parts are limbs
