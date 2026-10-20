# Spec Delta

## ADDED Requirements

### Requirement: Every terrain material layered
The system SHALL stack every palette material whose share of a tile passes 1/510, up to 16, in palette order: the first opaque, each later one covering what is below by its over opacity sharpened by its blend mask, as the original stacks all the materials a tile uses.

#### Scenario: Dirt over the black fill
- **WHEN** an exanima01 dirt room whose tile uses more than four materials is shown
- **THEN** its floor shows dirt everywhere, with no black patches where the flat black fill lies under it

#### Scenario: Shipped tiles
- **WHEN** every shipped terrain is built
- **THEN** no tile drops a material it uses
