## ADDED Requirements

### Requirement: Cut-away cascade
A cut-away mark SHALL carry to the mesh's parent under the parent bit for every marked kind, and to its mesh children under the children bit only from the overhead kind. Clearing a marked mesh SHALL carry to the parent and the mesh children from every kind; a mesh that was not marked SHALL be left alone, children included.

#### Scenario: Children follow only an overhead mark
- **WHEN** a sphere-kind mesh with the parent and children bits is hit by the camera's segment, and later an overhead mesh with the children bit is hit and then missed
- **THEN** the first marks itself and its parent but not its mesh child, the overhead mark carries to its mesh child, and its clear clears the child
