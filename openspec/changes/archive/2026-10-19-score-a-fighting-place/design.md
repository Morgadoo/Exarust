# Design

## Score

`score_place` takes the distance and a `Spot`. The close term uses the other fighter's reach plus 150 cm. The strike term, only in a fight, uses the observer's reach and step and the other fighter's size. The duellist calls `still_closing` and walks only while the candidate is past the plateau. `TCombatant` moves from 0% to 7%. The positioning row moves from 0% to 40%. The attack-arc danger is not scored. The AI section stays 1%. The headline stays 14.4% and 5.7%.

## Open questions

- The incoming-attack danger is not this score.
- A blow does not knock the fighter out.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
