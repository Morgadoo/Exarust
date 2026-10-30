# Design

## The sense the classifier calls

The original's contact classifier copies the observer's contact for the
observed character into a scratch record, calls the sense pointer the attach
stored (with the brain as its owner) or, when none was stored and the brain is
no statue, the observed character's base-perception virtual, then admits the
record and classifies it. The attach stores the standard routine for race words
with the AI bit and class byte `0x12`, the ally routine for the other AI race
words, and nothing for race words without the AI bit. The base-perception
virtual is the same routine in every character-behaviour class table, the one
Exarust ports as `combatant::perceive`.

So `personality::Sense` has three cases: `Base` (default, `perceive`),
`Standard` (`standard_sense`) and `Ally` (`ally_sense`). The role attitude the
round-42 report worried about stays where it is: only the base perception
reads the role database, and the humans keep it. The Golem Suit's sense writes
no attitude except the player's (1), so its other contacts keep the attitude
they were admitted with (0).

## The statue byte

In the standard routine the lying branch is taken when the observed body is
dead or asleep or when the observer's statue byte (the byte the attach sets for
a statue race word) is set. Exarust passed the observer's sleep there; the
routine now takes `statue` as `ally_sense` does.

## Classify before the drift

The integration routine runs, in order: the aim point and freshness, the worst
hostility cleared, the soothe damping's 0.001 step, the nearby list and (for
the brain whose turn it is) the target pass, then the per-contact drift and the
aggregates. The target pass therefore classifies contacts that carry the
previous tick's drift, and the drift reads the level, the attended contact and
the stress the pass has just written. Exarust's `run_pass` now calls
`contacts::aim`, the target pass, and `contacts::drift` after storing the
report's level, attention and stress. `contacts::integrate` stays as the two
halves back to back for the tests.

## The out-of-fight exit

The AI tick leaves after the footprint refresh when the body's condition has
bit `0x800`, before the integration and its target pass, so such a brain senses
nothing. `perceive_all` now skips such an observer.

## Open questions

- The one-brain-a-tick schedule of the target pass is not reproduced (answer:
  a later change; the round-robin variables are in the integration routine).
- The `0x80000` body flag that switches the hostility rule is not identified
  (answer: REA on its setters).

No docs/formats page is involved.
