# Spec Delta

## ADDED Requirements

### Requirement: Light activation colours
Activation SHALL set an intensity below the engine's epsilon to 0 and keep a NaN; the direct colour SHALL be the linear colour times the intensity and its luminance the engine's Rec. 601 luma; the GI colour SHALL be the direct colour times the multiplier unless the node word has 0x10000, and a second colour slot the direct colour unless the node word has 0x20000.

#### Scenario: A tiny intensity
- **WHEN** a light activates with intensity 1e-8
- **THEN** its intensity and every derived colour are 0

#### Scenario: The GI gate
- **WHEN** a light whose node word has 0x10000 activates
- **THEN** its GI colour is zero and its direct colour is not

### Requirement: Light activation reach
Activation SHALL give the light node the linear colour times (√intensity · multiplier) · 3; a ranged light SHALL take its radius as the reach and 0.8 · (1/R) as the falloff coefficient (1/R in single precision, the product in double), an unranged one 3.4e38 and 0.

#### Scenario: A ranged light of radius 2 m
- **WHEN** a ranged light of intensity 4 and multiplier 1 activates
- **THEN** its node colour scale is 6, its reach 2 m and its falloff 0.4 per metre

### Requirement: Light alias copy
The alias sync SHALL copy onto each alias the light's derived direct, second and GI colours, luminance, falloff, softness, reach and node-word bit 0x100000, and SHALL set the alias word's bits 16–23 from the light's node word keeping the alias's other bits; it SHALL NOT copy the record's intensity, radius, spot or scale.

#### Scenario: The flag byte
- **WHEN** a light with node word 0x00123456 syncs an alias whose word is 0xAB0000CD
- **THEN** the alias word becomes 0xAB1200CD

### Requirement: Content detach and attach
Detach SHALL clear the scene bit over a subtree, take a light out of its light list with its alias count dropped to zero (the alias nodes left as they are), an auxiliary node out of its list and a geometry node out of the geometry list, leaving meshes listed; attach SHALL append a node array to the parent's children and give each node the parent and its owner without unlinking it from an earlier parent.

#### Scenario: Detaching a light with an alias
- **WHEN** a light with one alias in the same sector is detached
- **THEN** it leaves the light list, its aliases are dropped, and the alias node and the sector's change bits are unchanged

### Requirement: Geometry node activation parts
Geometry activation SHALL reset the bound to zero, derive the shadow step and bias from the option word, clear the node word's 0x30000 and set the option word's 0x10, then per part set the part flags from its material, set the node's 0x10000 for a casting part unless the node has 0x80000, clear the option's 0x10 for a material with a class, and raise the bound per axis to the material's with ordered compares.

#### Scenario: Two parts
- **WHEN** a node activates with a casting class-3 part of bound (0.2, −1, 0.1) and a class-less part of bound (0.1, 0, 0.3)
- **THEN** its bound is (0.2, 0, 0.3), it has 0x10000, and its option word lacks 0x10

### Requirement: Geometry node activation bits
After the parts, activation SHALL drop the node word's 0x1000000 under a non-geometry parent when a 0x70000000 bit is set, set the alpha threshold to 1 with 0x8000000 else 0.5, and set 0x20000 on a node without 0x80000 whose squared bound, summed (x² + y²) + z², exceeds the engine's epsilon.

#### Scenario: A bound at the epsilon
- **WHEN** a node's bound is (0.000345, 0, 0)
- **THEN** it does not gain 0x20000, while (0.000346, 0, 0) does

### Requirement: Master hand-off order
A master's hand-off SHALL give every listed instance the new master; the new master's own slot SHALL become the old master, or, for a separate hand-off, the list's last entry with the last slot dropped (no instances when fewer than two were listed).

#### Scenario: A separate hand-off
- **WHEN** a master listing [3, 1, 2, 4] hands off separately to 1
- **THEN** node 1 lists [3, 4, 2]
