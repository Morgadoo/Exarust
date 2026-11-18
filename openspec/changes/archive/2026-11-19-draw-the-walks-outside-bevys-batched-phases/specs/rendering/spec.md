## MODIFIED Requirements

### Requirement: Scene pass order
The renderer's model of the original's scene pass SHALL order a frame as: the clear, the alpha-tested and opaque classes and three hook lists with depth writes, then with depth writes off the decal class and the blended lists under a 2·10⁻⁵ projection depth bias, a hook list, the cut-away faded pass's depth-only half and its colour half, the environment's weather effects and the closing hooks.

#### Scenario: Depth writes
- **WHEN** the pass order is walked
- **THEN** depth writes are on from the alpha-tested class to the third hook list and for the faded pass's depth half only

### Requirement: Portal scissor on the super-sampled view
On a super-sampled scene view each alpha-tested, opaque, decal or blended surface whose mesh carries the scissor bit SHALL be drawn only inside its sector's doorway rectangle, the rectangle in window pixels doubled to the sub-frame samples and held inside the viewport, and the surfaces after it without the bit SHALL be drawn over the whole view.

#### Scenario: A cobweb in the next room
- **WHEN** a cobweb of an adjacent room is seen through a doorway whose rectangle covers half of it
- **THEN** only the half inside the rectangle is drawn

## ADDED Requirements

### Requirement: Scissored opaque meshes leave the batched phases
A visible opaque or alpha-tested mesh of the scene view that carries the scissor bit SHALL be drawn with its own pipeline on the transparent phase, in the alpha-tested or opaque scissored list block ahead of every other transparent draw, and SHALL return to its own phase once it no longer carries the bit.

#### Scenario: Leaving the doorway
- **WHEN** a wall of the next room carries the scissor bit and then loses it
- **THEN** it is drawn clipped on the transparent phase, then again unclipped in Bevy's opaque phase

### Requirement: Transparent phase list blocks
The transparent phase SHALL draw its surfaces in the walk's list blocks: the scissored alpha-tested meshes, the scissored opaque meshes, the grass hook, the decal list, the blended list, then the late block (the faded pass's depth half, its colour half) and everything with no list; the hooks and the late block SHALL not be clipped to a doorway rectangle.

#### Scenario: Grass and a decal
- **WHEN** a plant chunk and a decal are both in view
- **THEN** the plant chunk is drawn first and neither is clipped

### Requirement: Alpha test in the colour pass
An alpha-tested record SHALL drop the fragments whose diffuse-map alpha is below one half (the skin program's diffuse alpha times the map's, the hair program's alpha without a specular map), and a terrain plant those below 0.1; a record without a diffuse map, or not alpha-tested, SHALL not discard.

#### Scenario: A cut-out leaf
- **WHEN** an alpha-tested leaf's texel has alpha 0.3
- **THEN** that fragment is not drawn

### Requirement: Faded pass depth first
Every part of a fading cut-away mesh SHALL lay its depth, with colour writes off, before any fading part's colour is drawn, and the colour SHALL be drawn after every blended surface, blended at one less the fade with depth writes off and the record's alpha test kept.

#### Scenario: Two fading roofs
- **WHEN** two fading roofs overlap on screen
- **THEN** only the nearer roof's colour is blended over the scene where they overlap
