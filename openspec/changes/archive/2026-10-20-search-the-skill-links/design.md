# Design

## Behaviour

The axis flag is 1 when the direction's y magnitude is at least its x magnitude. With the flag clear, every y offset is multiplied by 4 before measuring.

Phase one resolves the slot under the point through the node hit. When that hit is positive, the node's point comes from its layout, and two slots are tested: the slot below and the slot above the hit slot, wrapped into 0..4. Each candidate code reuses the hit's high byte and sets the low slot+1; an unreachable code is skipped. An accepted slot's satellite is `node + round(unit × 18.3 × cos) × turn` horizontally and `node − round(unit × 18.3 × sin) × turn` vertically at `slot × 0.2` of a turn. The normalized offset (after the y scaling) must dot the direction above 0.1, and the largest dot wins. A winner is written as integers and returned.

Phase two runs when phase one has no winner. The best metric starts at one million. Every node's five slots are tried with their stored high code; a squared offset under 1 is skipped, a dot under 0.3 is skipped, and `length / dot²` must be smaller than the best. The winner is written and returned; without one the input point and code 0 are returned.

## Model

`pick_skill_satellite(input)` takes the point, direction, hit code, node point, node list, reach predicate, unit and turn, and returns the code and point.

## Open questions

The hit, reach and layout bodies are not in.

## Docs

This method is 1363 bytes. `TSkillUI` moves from 25% to about 32%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
