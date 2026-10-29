# Design

The original keeps one flags word beside the condition on every body; the
flow now uses that word (`exa_combat::StateFlags`) instead of a component
of its own, so the wake tick and the story hook read the same bits.

The lie-down is `exa_combat::condition::knock_out` plus
`Ragdoll::knock_out`; the dead-body revive is `condition::revive` plus the
body's muscles back (`Ragdoll::revive` for a limp body). Both insert a
`KnockOut`, the marker the live wake tick runs on (the original runs it on
any body whose flags hold 0x20; Exarust's sleepers have their own wake in
`exa_ai::idle`). A wake write in the same batch as a fresh knock-out
changes the pending copy, because the insert is deferred.

The wake tick runs after the condition in each character's posture stage
(the order is a guess). Its getting-up clock is read as the clip's
progress and the full wake's stance as the race's standing factor.
