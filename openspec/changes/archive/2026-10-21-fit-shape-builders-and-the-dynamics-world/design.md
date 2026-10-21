# Design

Extends `docs/formats/RFC.md` (Rigid body: registration of hung trees) and
the `exa_physics` README. No format changes.

## Loops that run four times

The builders' vertex loop and the body setup's share loop both start their
counter at −1, increment it first and loop while it is under 3, so the body
runs for 0, 1, 2 and 3. Read from the pseudo-code alone this looks like
three passes; the listing settles it. The shares are therefore 4/mᵢ times
4 over their four-term sum (the quotient taken first), and every rest point
is scaled and placed.

## The grid's move

FPC's `Move` takes the source first: the positive branch moves the table
from its start to `offset` cells further and empties the first `offset`
cells, so cell j takes what j − offset held. The strip clears follow the
listing, including its start column `w + dx − 1` for a leftward shift and
its row counts (rows beyond the table are skipped in Exarust where the
original writes past it).

## Cells

A sector's cell is its node's x and z times the float 1/600, rounded to
even by the default conversion, plus half the grid. A body's cell in the
placement query uses the floor helper (it sets the rounding mode down for
one 32-bit conversion) on its bound sphere's centre, plus the sector's
cell. The nine offsets start with the cell itself.

## Placement query

A candidate is tested when its bound sphere is not apart from the body's
start bound (the sign of the gap), it is of another root, its layers do not
keep the two apart (no shared layer bit above the lowest, or no exclusive
channel), and it is enabled. The model had the layer test inverted. The
lower sector index goes first; the purges renumber that index.

## Open questions

- The world-link laws come from `body-links.md` §6.4, not from this unit's
  listing; the slack stored squared by the registration routine is not
  modelled for a plain slack link (answer: read the link iteration's
  listing with REA in the body-kernel lane).
- The two shape tests of the placement query and the sector step that links
  bodies into the grid belong to the sector lane (answer: REA on those
  routines there).
- What state bits 0x2 and 0x4 mean is not established; they are named by
  their helpers.
