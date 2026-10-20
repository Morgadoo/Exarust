# `exa_items`

Item database and procedural items: weapons assembled from components by factories, armour layers, materials and quality.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: equipping by occupancy (`carry`), an inventory pane of icons (`pane`), the description card's page and coin value, a weapon card's damage rows, an apparel card's coverage and rows, the card icon's place, a shield card's weight, impact, and encumbrance, the arena gate for rank and points, a description phrase picked from the rows in range, the weapon sentence around that phrase, the fixed-id name edit, and the apparel sentence (`card`). Possessions as the character record keeps them (`possession`: two weapon sets, held marks in the carried list, swapping sets, giving, having an item through nested containers, the take-into-hand and take-out rules, loose items, golem cores, the AI's planned arms), the object-database catalogue and the swap systems (`catalogue`), copies of items (`instance`: condition from the template, price, desirability, protection, spreads and the roll) and the world-load fill of containers from random tables with grades (`randomise`). Injuries beyond damage are still open.

## Scope

- Object database (`objdb.rdb`, `objstrings.rdb`), item instances and inventory/containers.
- Weapon factories (`Factories.rpk`) assembling `Components.rpk` parts; mass/balance derived from geometry.
- Procedural material grades (steel, iron, copper alloy, leather, wood...) and their texture generation.
