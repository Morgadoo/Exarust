## ADDED Requirements

### Requirement: Grid build comparison
A grid scene's leading `exe=<path>` word SHALL run that binary for its quadrant instead of the host's own, and its leading `env:<KEY>=<value>` words SHALL set that child's environment; the remaining words are the scene's arguments.

#### Scenario: Two builds side by side
- **WHEN** `exarust grid "exe=/tmp/old play exanima01" "play exanima01"` runs
- **THEN** quadrant 1 runs /tmp/old and quadrant 2 the host's build, both on exanima01
