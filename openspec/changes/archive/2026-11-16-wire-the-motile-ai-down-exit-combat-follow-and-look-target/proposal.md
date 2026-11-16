## Why

Roadmap task B4 (lane `motileai2`, phase B of docs/roadmap/phase-b-close.md),
the second pass over the NPC AI's open rows. The combat state's down exit
left its arc kind, the body's stance word and its pose word unwritten; the
combat state's follow check (the follower branch) had no live caller; the
follow mode word, the leader's planned point and the body the walk lets pass
were not kept on the route; the flight's escape never soft-stopped a moving
body and its run did not write the body's cursor and wanted directions; the
walk's head look read a look target nothing set.

## Evidence

REA (Exanima.exe, Ghidra) and the disassembly; the constants read from the
image:

- The down exit: with the down bit, the arc kind is "none" (0) when the
  condition is exactly the getting-up word and the body's clip clock is past
  0.9 (a double compare, NaN not past), else "recovering" (4); the stance
  word becomes the guard by a plain store (no set-stance helper, no clip
  start); the pose word is cleared. The combat state's estimate step (past
  the follow and the leave) clears the pose word too. The arc kind is the
  word the position score reads for the other fighter.
- The follow check: run first in the combat tick for a fighter whose
  activity is the follow, unless it steps; the leader is the mind-controlled
  body or the player. The band (400 at the follow's install and in the
  follow state's band branch) eases by `0.99·b + 7` (`0.8·b + 0.2·(100·a +
  500)` while the leader's request has bit 4), 0.7 of it while following;
  inside it a follower soft-stops; outside it the urgency is 2 with the
  leader's bit 8 or past three bands, else 1; a follower walking to its
  leader lets it pass the walk's in-the-way test and is called (its combat
  tick ends), replanning when the leader is more than 100 cm from where it
  planned; a plan is forced, to the leader's ground point 20 cm up, stop 40,
  size class 1; success makes the mode word 4 and keeps the leader's point.
- The route stop and the soft stop both clear the mode word; the walk lets
  the body it records pass (that body is set by the follow and the follow
  check).
- The flee tick: escaped (not fresh, the threat's room known and neither its
  own nor its home) with the walk's moving byte set, it soft-stops; running,
  the body's smoothed and raw cursor and its wanted direction take its
  heading.
- The target pass's attended contact is the AI's look target the walk's head
  look reads.
- The leave's copy of the body's frame origin is written only: no read of
  it on the AI object (the reads at that offset belong to the player's
  behaviour, whose word it is there).

## What changes

- The duellist's down exit publishes its arc kind for the position scores of
  the fighters around it, writes the guard as its stance word without
  starting it (the clip under way ends in it), and with every tick past the
  leave clears the pose word, which ends a statue's rest.
- The duellist runs the follow check for a following fighter, plans its walk
  to the leader and ends its combat tick when the leader calls; the game
  publishes the player as the leader each body tick.
- The route keeps the walk's mode word (cleared by every stop), the leader's
  planned point and the body it lets pass; the follow state reads and writes
  them there and keeps its route while the combat state fights.
- The flight soft-stops on its escape and aims its run's directions along
  the heading.
- The walk's head looks at the attended contact.

## Out of scope

- A flight without a route fighting that tick (the original runs the combat
  state then); the casting event-hook stop; the dormant holder move; the
  round-robin dialogue refresh (the contact's event word is not kept); the
  player's own room gather; the scripted action table.
- GPU scenes and determinism runs (the card is on hold): listed in the
  report.
