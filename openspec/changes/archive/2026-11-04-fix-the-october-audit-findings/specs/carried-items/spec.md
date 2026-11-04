# Spec Delta

## ADDED Requirements

### Requirement: Kit condition writes
A cleaning or repair kit's dirt and wear bytes SHALL reach the item records the panels and the worn figure read, and the wear byte the description card's sentence. A kit's own copy of an item SHALL take its template's record and card under its own id, and its later writes and container list SHALL be its own.

#### Scenario: A copy keeps its own record
- **WHEN** a kit cleans a template to dirt 0x20 and a copy of it is repaired to wear 0x10 and given a container child
- **THEN** the copy reads dirt 0x20, wear 0x10 and one child, and the template keeps its wear and no child
