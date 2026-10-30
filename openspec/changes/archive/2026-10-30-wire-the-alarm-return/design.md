# Design

Format page: `docs/formats/OTHER.md` (the alarm zones' volume section; no
format change).

**When it runs.** The original runs the alarm return only from the world's
revive pass, which only the story revive calls, before the save. The story
revive runs five seconds after the story hook saw the player dead (condition
`0xA00`) with red health of at least 0.01 (double). Exarust's flow already
models the knock-out and revive (`Flow::knock_out`, `Flow::revive`); the
revive now queues `Step::AlarmReturn` right after the prelude closes. The
golem module takes that step out of the queue between the transition and
the step application (`FlowSystems`), so the guardians and the player are
reset before the save and before the body's revive raises the player.

**The query.** Centred on the player (its model's place), radius 3000 cm,
x and z only, strictly inside, AI-bit race words, over the player's room and
the rooms behind its exits (Exarust reads each character's room from its
position, as the wake's query does).

**The homeward callback.** Refuses a statue (statue byte set) and a guardian
whose condition carries `0x800` (dying or dead: the bit the existing code
called a "returning" alarm bit; the word is the condition word, which the
sleep also zeroes). Fills a home whose x is 0 from the body. Writes tone and
stance factor 1 (the original's tone and tone product, which the knock-out
zeroes; a third word whose role is not traced is noted), condition 0, Stand
2, resets the body at its home facing the home angle (woken 32 ticks), then
sleeps it: statue bit and byte, statue rest (the pose word 2 and the rest
byte), condition 0, and the first time the latch and the sleep scales. With
its statue byte set the golem's ally sense zeroes its concern, so its target
pass leaves combat on its own, as in the original (the sleep installs the
sentry routine; Exarust's sentry is the idle routine the brain is already
in).

**The player.** When any guardian went home and the first alarm node (in
placement order, named `Alarm_`) holds the armed point within 0.1 squared,
the player is reset at the node's centre plus its third axis times its third
half extent plus 200, facing the third axis's heading plus π plus a draw
less ½ (the draw only then), with motion 0x1d requested, condition `0xA00`
and tone and stance factor 0; the revive that follows raises it knocked
out. The alarm stays armed.

**The move.** The pose reset rebuilds the body at the new frame. Exarust
moves the character root (x and z; the root keeps its height [guess: the
original's frame height is the body's standing-height word]) and turns the
model. A standing body is carried with its root by its next aim (parts,
feet anchors, targets, frame), so placing its parts too would move it twice
(a first build did, and the golems fell and kept trying to get up); a
fallen or limp body (the dead player) is placed rigidly about its root.

Open questions and how they were answered:

- When does the return run? REA callers: only the world's revive pass, only
  from the story revive (its callers: the knock-out hook's five-second
  check). Not on a timer, not on leaving the zone.
- Which bit does the homeward callback test? The condition word's `0x800`
  (the dying bit, set by the collapse and dying tick; the dead condition
  `0xA00` carries it).
- Where does the player go? The `Alarm_` node lookup over the world's
  objects' nodes; the node names are the alarm items' names (corpus).
- Does the alarm disarm? No: the return writes nothing to the armed state.

Not answered: the homeward callback's third word; what the world's revive
pass does to every brain beyond the alarm return (left out).
