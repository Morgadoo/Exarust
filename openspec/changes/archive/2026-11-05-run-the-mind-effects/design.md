# Design

Rules page: [`docs/THAUMATURGY.md`](../../../../docs/THAUMATURGY.md),
Mind in the running game.

## Who owns a body

The original keeps a status word on every body and two global pointers:
the player's body and the body it left. Exarust keeps the status word in
the body's `MindBody` (on a body with a power caster, the caster's
`CastBody` is the word the casts and ticks read), the player as the body
with `PlayerBody` under the root with `Player`, and the body left in
`MindControl::held`.

- `0x10_0000` (handover): the caster of a running Control Mind or Spirit
  Form. The combatant's take-hit reads it.
- `0x60_0000` (in control): the body the player was handed by Control
  Mind; its bar's Use ends the control (bit `0x20_0000`).
- `0x22_0000`: the spirit body.

## Control Mind

The cast places the binding (state 1, the cost as the countdown, the
scene clock as the stamp). Each tick it takes 0.002 of focus while the
countdown is above 0; a collapsing target ends it with its focus given
back; past 2000 ms it becomes state 0 and hands over: the caster gets the
handover bit and its player behaviour is held with its step cleared, the
target gets the control bits and the possession marker (owner the target,
caster the caster), its AI behaviour is set aside and it gets a new player
behaviour whose look vectors are the caster's. The camera's target moves
on the next frame (a queued call). The end unlinks the marker, gives the
focus back and opens the transition screen with no audio fade and the
600 ms fade; at black the camera comes back and, queued, the behaviours
swap back: the target's status becomes the caster's less the control bits
(handover bit included), the caster loses the handover bit.

Exarust keeps the AI components (duellist, brain, duel powers) in
`HeldBehaviour` on the possessed body and the player behaviour in
`HeldPlayer` on the held body. References to the target stay valid: the
original's broadcast that swaps behaviour pointers in every brain has no
counterpart because Exarust's references are entities.

## Spirit Form

The cast (focus 0.05 in double) adds the power, sets the handover bit,
starts the spirit screen and its music and builds the spirit body: a new
record of race byte 14 sharing the caster's powers, placed 60 cm along the
caster's facing on its ground height, given the caster's focus, reserve
and capacity change and the list (rebound to it), a player behaviour
whose spawn-snap count is 4, status `0x22_0000` and the player. The
caster's behaviour is held with its step cleared. Each tick the power
takes its drain from the spirit's focus; below 0.001 or when told to end
it clears the handover bit, fades the spirit screen out and opens the
transition screen; at black the body takes the values and the list back,
its behaviour runs again, the camera returns and the spirit leaves the
scene.

Exarust moves the power caster and the list instead of sharing them; each
body keeps its own status word.

## The hit's break

The combatant base's take-hit, unless the struck body is down or its AI
is held, calls the break when the body's status has the handover bit: the
first entry of its list that is a Spirit Form whose body it is ends on the
spirit; else the first Control Mind ends. With the list moved to the
spirit, Exarust looks the spirit up by its `SpiritOf` link.

## The perception's overrides

Before the brains' tick each body's list is written as the effects the
perception reads (power id, caster, target, state); a hold the
perception breaks (state 2) ends its power after the tick.

## Open questions

- The pick's line of sight (a ray from eye to eye through the scene): no
  scene query in the running game; taken as clear.
- The one word the hand-over copies from the AI behaviour into the new player behaviour (and back)
  (Exarust's player behaviour has no counterpart named yet).
- The wraith's physical frame (answered by the body work, not here).
