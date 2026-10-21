# Design

## Shape

`perceive(observer, observed, observed_contacts, own, index, raw,
trust_of, effects, now)` updates `own[index]`, the observer's contact for
the observed character, in the order the original does: the threat is
cleared, an invisible character is skipped, the range test keeps or drops
the tracked bit (and reports the alert floor of a lost target), then
closeness, noticeability, the effective attitude and tolerance and the
view weight are written; a dead or asleep character stops there; the
group blend and the threat estimate follow. The raise of the concern
target of the observer's other contacts walks `own` by identity, so the
record being written is never borrowed twice. The pass returns a `Sight`
(seen, the alert floor, the familiarity drift, whether the mind-power check
would run) for the caller.

The role database is an input: `raw` is the attitude and tolerance toward
the observed character, `trust_of` the trust toward a third party. The
duellist passes its stored attitude and no trust, since the brain's role
attitudes are not wired to it yet.

The position score keeps one function (`score_place`); the live blow's
danger and its effect on the approach term come from one helper so that
the thrust's scale (applied past the gate too) and the never-arriving side
swing's scale are written once.

## Precision

The original converts to double for the live threshold (0.3 − 0.15·skill),
the side swing's wait below phase 0.4, the thrust's wait, the danger ramp
(4 − ms/300), the approach floor (0.1), the thrust edges past phase 0.2,
every angle wrap and the arc midpoint's bound, the closeness cap (0.99),
the hidden radii, the group blend's ×0.01 steps, the threat weights'
0.8/0.2 and 0.6/0.4 blends, the ×0.85 multiplier, the threat's ×0.1, the
familiarity factor and drift, the tolerance's ×0.01, the concern call's
attitude factor and the unconscious decay. Exarust performs the same
conversions, and keeps the original's operand order where a product or
sum is grouped differently from the written formula.

## Open questions

- Which body vector the observer's heading term reads is established (the
  heading the locomotion steers to); the unit helper that measures the
  move point's distance is taken to be the squared distance (not read).
- The engine's inverse cosine used by the yaw helper is reproduced as the
  head tracking already does; its exact constants are not in this pack
  (REA on the engine's math unit).
- The meaning of the contact's word that dies with the concern, and of
  the body byte that picks 16 over 8 out of combat, is not identified
  (REA on the contact integration and the body's writers).
- The original keeps the last published arc edge when no part is the
  weapon hand; Exarust uses the heading then.

No `docs/formats/` page is concerned.
