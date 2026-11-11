## MODIFIED Requirements

### Requirement: Geometry node activation parts
Geometry activation SHALL reset the bound to zero, derive the shadow step and bias from the option word, clear the node word's 0x30000 and set the option word's 0x10, then per part set the part flags from its material, set the node's 0x10000 for a casting part unless the node has 0x80000, clear the option's 0x10 for a material with a class, and raise the bound per channel to the material's shared colour (specular × level + diffuse) with ordered compares.

#### Scenario: Two parts
- **WHEN** a node activates with a casting class-3 part of bound (0.2, −1, 0.1) and a class-less part of bound (0.1, 0, 0.3)
- **THEN** its bound is (0.2, 0, 0.3), it has 0x10000, and its option word lacks 0x10

#### Scenario: A part from its record
- **WHEN** a part's record has diffuse (0.1, 0.2, 0.3), specular (1, 0, 0) at level 0.5 and a normal layer
- **THEN** its bound is (0.6, 0.2, 0.3) and its mode word has 8
