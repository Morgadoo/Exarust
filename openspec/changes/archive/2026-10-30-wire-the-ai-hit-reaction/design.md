# Design

The original reacts inside the damage routine, synchronously; the AI reads
the result at its next tick. Exarust writes a message at the same three
points of `react_to_hits` and reacts in a PostUpdate system after the
combat's frame systems, before any AI tick of the next frame, so what the
AI tick sees is the same. Within one frame the notices are read in the
order the contacts were dealt.

The behaviour base's count (`CombatScore`) is dealt in place, where it was;
the wound credit moved up to the call's place, before the blow feedback that
stands for the original's player camera.

The base's down/dead gate reads the struck body's condition at the call:
on flesh, after the damage call, which in the original starts a killing
wound's collapse. Exarust starts the collapse after the call, so the notice
counts a killing wound as down. The AI's concern rise is not gated: the
override runs its contact loop whatever the base did.

The mark the base leaves is the striker's swing word bit that the player's
late swing check also sets; the lesson is the striker's practice lesson. A
duellist's practice is in `exa_ai`; the player's is `exa_game`'s, reached
through a `Taught` message. An attacker without a swing word takes the
lesson without a mark.

The threat estimate's inputs: the struck brain's nearby list, each body's
chest point standing in for its aim point (the duellist's own stand-in;
the original's aim point's exact place is an open question in the combat
research notes), a dead body skipped by its condition, and the attacker's
root position, facing and reach (its longest armed held weapon's, else the
unarmed reach). The estimate is only computed below the concern cap, as
the original orders it, so a hit costs one contact scan and at most one
pass over the nearby list.

Open questions: the body's aim point (a capture of the original's field
while a fighter stands and lunges); the mind bit the base reads (the mind
powers' trace of which power sets it).
