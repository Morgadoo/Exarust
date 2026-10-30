# Spec Delta

## ADDED Requirements

### Requirement: Shipped outfits under the cull
A dressed human SHALL hide a body slot only where a garment node of that slot is merged with triangles left after the culling append, and SHALL keep its underwear and trousers drawn where the cull bits leave their triangles.

#### Scenario: Braies alone
- **WHEN** the shipped human wears only `braies_a`
- **THEN** the braies' visible mask is `0x601C0013`, their pelvis node's 416 triangles are merged without cull bits and the body keeps all eighteen slots

#### Scenario: Trousers under a shirt
- **WHEN** the shipped human wears `trouser_a`, `shirty_a` and `sandals_b`
- **THEN** the trousers' pelvis node takes cull bit `0x200000` and loses 126 of its 310 triangles, the four leg nodes take `0x400000` and lose none, and the body keeps only its feet, chest, head and hands (`0x30348`)

#### Scenario: Hidden slots are covered
- **WHEN** the shipped human wears `trouser_a`, `leathjac_a`, `boots_b`, `helm_b`, or `braies_a`, `gloves_a_lth`, `helm_b`, `boots_b`
- **THEN** every body slot left out has a merged garment node of that slot with triangles left
