# Design

## The muscles-off sampler stays without a caller

The original's body tick, after its movement decision, keeps a local that
says the body locomoted this tick. With muscles off the movement decision
is skipped and the local takes two character words instead (a byte that
is only ever cleared, and a counter that is only ever decremented while
positive). With the local clear and the muscles off, the tick jumps past
the clock advance, the end test and the sampler call, to the carry. A dead
body returned much earlier. No path sets either word, so no body runs the
muscles-off sampler. Scheduling `exa_game::limp::sample_limp` would add
behaviour the original does not have: a knocked-out corpse snapping to its
last clip key every tick.

## Daze pose order

The original computes the daze pose in its daze block, right after the
Rebalance start, whose capture is taken inside the start routine.
Exarust's starts take their capture at the sampler stage, so the daze pose
is computed there, after the capture and before the hold is applied. The
draws come from a local engine generator (the original's is global; the
body tick's other draws in Exarust are also per-system streams).

## The wake's place

The waking sits between the behaviour update and the posture block, so it
joins `CharacterStage::Behaviour`. The muscles-off body's wake (no drive in
Exarust) is still skipped; the original would only write the condition and
flags of a body nothing samples.
