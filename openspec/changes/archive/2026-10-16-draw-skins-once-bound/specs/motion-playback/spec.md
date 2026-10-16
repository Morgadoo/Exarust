## ADDED Requirements

### Requirement: Skins bound before drawing
The system SHALL draw a skinned body or apparel mesh only once it is bound to its character's body parts, so that no frame draws a mesh with skin attributes without its skin.

#### Scenario: Character view start
- **WHEN** the character view starts and its body is dressed over several frames
- **THEN** no frame fails GPU validation and the body appears skinned once bound

#### Scenario: Covered body parts
- **WHEN** apparel covers a body part before the part's mesh is bound
- **THEN** that body part stays hidden after binding
