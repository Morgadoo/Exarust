# Spec Delta

## ADDED Requirements

### Requirement: Terrain block activation
A terrain block's activation SHALL take the geometry node's shadow step and bias from its option word, set its bound to 1 on every axis, and build its kd-tree unless its external-draw bit is set.

#### Scenario: An external block
- **WHEN** a terrain block whose external-draw bit is set activates
- **THEN** its bound is (1, 1, 1) and no kd-tree is built
