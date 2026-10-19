# `exa_items`

Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: equipping by occupancy (`carry`), an inventory pane of icons (`pane`), the description card's page and coin value, a weapon card's damage rows, an apparel card's coverage and rows, the card icon's place, a shield card's weight, impact, and encumbrance, the arena gate for rank and points, a description phrase picked from the rows in range, the weapon sentence around that phrase, and the fixed-id name edit (`card`). The apparel sentence is still open.

## Scope

- Object database (`objdb.rdb`, `objstrings.rdb`), item instances and inventory/containers.
- Weapon factories (`Factories.rpk`) assembling `Components.rpk` parts; mass/balance derived from geometry.
- Procedural material grades (steel, iron, copper alloy, leather, wood...) and their texture generation.
