# `exa_items`

Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: equipping by occupancy (`carry`), an inventory pane of icons (`pane`), the description card's page and coin value, a weapon card's damage rows, and an apparel card's coverage and rows (`card`). The icon picture is still open.

## Scope

- Object database (`objdb.rdb`, `objstrings.rdb`), item instances and inventory/containers.
- Weapon factories (`Factories.rpk`) assembling `Components.rpk` parts; mass/balance derived from geometry.
- Procedural material grades (steel, iron, copper alloy, leather, wood...) and their texture generation.
