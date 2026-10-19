# Design

## Vertex

`shell_vertex` normalizes the bone sum with the approximate reciprocal square root, and a zero sum stays zero. The vertex is the position plus 2 times that direction. The stored direction is the same unit.

## Hit

`shell_hit` refuses an eighth hit. At or under 0.3, the strength is (2 minus the hit times 3.333) times that scale times 6. Above 0.3 it is 6. The f32 value 0.3 is already above the 0.3 limit, so it stores 6.

## Open questions

- Copying the skin's index list, and the stored size of 800, are not in.

## Docs

`docs/formats/OTHER.md` gains the vertex and the hit. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the shell mesh at 20%.
