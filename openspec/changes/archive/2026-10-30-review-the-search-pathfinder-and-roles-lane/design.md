# Design

The line helpers keep Exarust's float grid deltas for the axis choice: the
original compares world deltas, and the grid scale is positive, so the
choice only differs where rounding moves a tie.

The off-floor search orders by the ground spread alone. The original adds
the square of a register left by its caller, the same for every cell, and
pushes a met cell only when its spread is within the bound so far; with
the constant left out both tests reduce to the spread.

Exarust keeps two relation stores: the AI's `Relations` component
(familiarity, last attitude, meeting time) and the script cast's relation
records (relation roles, topic memory, `LastEnc`). The original has one.
The encounter bridge renews the meeting time in both, before the scripts
run, as the original renews it before running them.

The worn take's grip test is a stand-in: Exarust has no gripped object,
so the item in the second hand slot, the slot the original clears, is
taken as the gripped one [guess].
