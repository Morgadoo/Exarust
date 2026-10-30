# Spec Delta

## ADDED Requirements

### Requirement: Skin joint capture
The skin bind SHALL capture body parts depth first in child order, the first node of a bone number setting its frame; a node of a bone already captured SHALL end its branch, and a bone-0 node or a non-part entity SHALL pass through to its children.

#### Scenario: Repeated bone
- **WHEN** a node of bone 3 sits beside an earlier bone-3 node and holds bones 6 and 9 below it
- **THEN** bones 6 and 9 are not captured
