# `exa_items`

Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Scope

- Object database (`objdb.rdb`, `objstrings.rdb`), item instances and inventory/containers.
- Weapon factories (`Factories.rpk`) assembling `Components.rpk` parts; mass/balance derived from geometry.
- Procedural material grades (steel, iron, copper alloy, leather, wood...) and their texture generation.
