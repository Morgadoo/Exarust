# Design

## Behaviour

The pass returns when its body argument is the cloth's own meta pointer. Otherwise it records the body and, for each particle, an accepted volume and segment hit makes one callback, stores the particle index and the side chosen by the volume branch, and, when the callback resolved the hit, corrects the particle. The correction takes the edge `e0 − d8`: when its squared length is at or under the radius squared the particle becomes e0; otherwise it becomes `e0 + edge × (|edge| − radius) / |edge|`. A particle outside the hit volume, rejected by a segment test, or unresolved is unchanged.

## Open questions

The volume and segment queries and the callback body are not in; they are inputs.

## Docs

This method is 780 bytes. `TVCCloth` is 81%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
