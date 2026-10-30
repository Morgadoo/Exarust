# Design

The standard sense keeps the listing's precision: the closeness clamp in
double, the attacking weight's view term and the general weight's closeness
term in double, the softening's product in double then single, everything
else in single with the listing's operand order. The engine's clamp helper
is a MINSS with 1.0 as the second operand, so a NaN softening becomes 1,
which `f32::min(1.0)` also gives.

The original writes the selected character from the observer's own contact
array at the index the heaviest entry has in the observed character's
array. Exarust reproduces that read; past the end of the observer's list it
selects nobody (the original reads its fixed array's stale slot).

The race word is `CharacterRecord::profession()`, the dword the attach
reads through the body's record. The fellow test of the ally sense reads
the observed body's race word AI bit, which a brain spawned from a record
now carries as `Npc::race_ai`; a brain made without a record has none.

The standard sense is left unscheduled: wiring it would drop the hostility
the role side feeds through `perceive`, and three brain tests that rely on
role-driven engagement would fail. The ally sense is live for the golem
races; a statue golem's sense clears what it sees until something clears
`Npc::statue`, which only the unwired alarm-zone wake would do.
