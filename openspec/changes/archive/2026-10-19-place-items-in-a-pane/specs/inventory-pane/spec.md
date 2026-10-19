# Spec Delta

## Purpose
The inventory pane: where an icon sits, when a drop is refused, and which icon a point hits.

## ADDED Requirements

### Requirement: Drop
A drop from outside SHALL place the icon at the cursor minus the grab, clamp it inside the pane, and add its bulk. A drop from the same pane SHALL only move that icon. An item larger than the pane's limit SHALL be refused.

#### Scenario: Place and move
- **WHEN** an icon of bulk 2 is dropped from outside near the far corner of a 100 by 80 pane, then dropped again from the same pane
- **THEN** it is clamped inside, the bulk stays 2, and the second drop only changes its position

### Requirement: Capacity
A drop from outside SHALL be refused when the bulk would pass the capacity. A drop from the linked pane SHALL not be refused for capacity. A container SHALL NOT be dropped onto the pane that shows it.

#### Scenario: Linked and full
- **WHEN** a bulk of 2 is dropped into a pane of capacity 1 from the linked pane, then a bulk of 1 is dropped from outside, then the pane's own container is dropped
- **THEN** the linked drop lands, the outside drop is refused, and the container is refused

### Requirement: Drag and hit
Ending a drag SHALL remove the icon. The hit SHALL be the top-most icon whose rectangle holds the point.

#### Scenario: Top icon
- **WHEN** two overlapping icons are dropped and the point lies in both, then the lower icon's drag ends
- **THEN** the hit was the upper icon, and the lower icon is gone
