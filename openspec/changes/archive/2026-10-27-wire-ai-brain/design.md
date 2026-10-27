# Design

Rules: [`crates/exa_ai/README.md`](../../../crates/exa_ai/README.md), the
brain and Contacts and flight.

## Decisions

- Three systems in `Update`, in order: the role and player rating, the
  perception pass, the AI tick, then the duellist's combat state. The
  original perceives inside its target pass; Exarust perceives just before
  it, in the same frame, after the same clear.
- Every brain runs its pass every frame (the original runs one brain a
  tick); the reach of a tracked perception plus the flat band stands in
  for the missing line-of-sight filter. A known contact is always
  perceived so a lost track is handled.
- The brain keeps the integration's values; the combatant's contacts take
  its concern, hostility, concern target and noticed level after the pass,
  so the combat state and the next threat estimate read one list.
- The duellist fights only its own target: an entry the pass makes against
  another character does not start it (labelled a guess in the code).

## Not wired

Listed in the proposal's out-of-scope paragraph.
