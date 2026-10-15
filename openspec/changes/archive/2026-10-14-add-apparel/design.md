# Design

## Decisions

- Apparel parts are authored in their body part's space, so they are spawned
  as children of that part with an identity transform.
- Run-time `&` materials are resolved from their name alone; the grayscale
  diffuse waits for procedural tinting (masks `m1`/`m2`, item material
  colours), which belongs to the item system.

## Risks / Trade-offs

- Body skin can show through tight apparel until covered parts are hidden.
