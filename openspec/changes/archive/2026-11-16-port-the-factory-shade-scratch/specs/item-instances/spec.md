## ADDED Requirements

### Requirement: Factory paint regions
A factory weapon's paint job SHALL list a region for every recipe part with variants, in recipe order, whose choice byte is not zero, a choice past the part's variants taking its first variant. Texture-only overlay parts (part flag 1) SHALL have a region though they are never placed, and a region SHALL be kept only when its first mask loads.

#### Scenario: A blade's fuller
- **WHEN** a shipped sword recipe chooses its texture-only fuller
- **THEN** the fuller has a paint region and no placed mesh

### Requirement: Factory shade scratch
Before its colour passes a factory paint job SHALL draw every region's shade into a one-channel scratch of the atlas's size, cleared to 1, over the texels the region's quad covers: with s the region's shade and m its first mask, the scratch becomes R + scratch·A for R = s·m·w.x and A = mix(s·(1 − m·w.x), 1 − m, w.y), stored as a byte, where w is (1, 0) for a part flagged 2, (1, 1) for an overlay part and (0, 0) otherwise.

#### Scenario: An overlay over a plain region
- **WHEN** a plain region of shade 0.5 covers the atlas and an overlay of full shade covers its left half with its mask set on the first texel
- **THEN** the scratch is 255 at the first texel and 128 elsewhere

### Requirement: Factory regions read the scratch
Every pass of a factory region SHALL read its shade from the scratch, nearest and clamped at the edges, at the region's texel centre mapped by the region's rectangle (its height scaled by the blade programs' texture scale), and the steel normal pass SHALL turn the item normal by the signs of the rectangle's size.

#### Scenario: A mirrored steel region
- **WHEN** a steel layer at full quality and no damage paints an item normal pointing along x in a region mirrored along x
- **THEN** its normal's x byte is 0, not 255

### Requirement: Factory alpha job
A factory job whose areas roll Gemstone SHALL, after every colour pass, multiply the colour target's alpha (1 after the colour passes) by each Gemstone layer's alpha program output a over its mask m, as a·m + 1 − m clamped to 0..1, stored as a byte; a material without a shipped alpha program SHALL leave the alpha unchanged. The weapon's material SHALL then refract through that alpha, reading the scene's sharp field at strength 1; every other weapon's atlas SHALL be opaque.

#### Scenario: A gem texel
- **WHEN** a Gemstone layer at shade 128, quality 0.6, damage 0.3 and dirt 0.4 with the test library is drawn with a full mask over an opaque texel
- **THEN** the program gives 0x3e1b910e and the texel's alpha is 39
