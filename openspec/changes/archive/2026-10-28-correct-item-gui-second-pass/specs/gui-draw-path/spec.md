# Spec Delta

## ADDED Requirements

### Requirement: Item panel icons
The inventory screen SHALL draw each visible pack entry whose icon is in the panel's atlas through the `HltCol` program at its cell inside the pack panel's client area, clipped to that area, and SHALL draw no icon layer for an empty pack.

#### Scenario: One entry
- **WHEN** the pack holds one entry with a cached icon at cell (10, 20) and the inventory opens
- **THEN** the frame holds one `HltCol` layer clipped to the pack's client area with its quad 10 pixels in from the clip's left
