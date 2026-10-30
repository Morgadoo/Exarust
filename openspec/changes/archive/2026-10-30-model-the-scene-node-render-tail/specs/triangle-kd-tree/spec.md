# Spec Delta

## ADDED Requirements

### Requirement: Background builder flags
The background kd-tree builder SHALL accept every queued mesh, a push clearing the reset's discard flag and the idle flag; a reset SHALL set the discard flag and empty both rings; a mesh built while the flag is set SHALL NOT be handed back; a deleted mesh SHALL be skipped; the idle flag SHALL be set when the thread finds the incoming ring empty. The query install SHALL write seven callbacks with the tree as context onto the mesh and copy them onto every instance.

#### Scenario: A reset during a build
- **WHEN** the queues are reset while a mesh is being built
- **THEN** that mesh is not handed back, and the next queued mesh is
