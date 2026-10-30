# Design

Extends docs/RENDERING.md, Rooms and black fill.

## Binding

The level loaders call the grid manager's binding for each built node whose
kind word is the door kind. The binding reads the node's world position and
third row, finds the link as before (`DoorBodies::bind`), and writes the body
of the node's first child into both rooms' link entries; a node without
children binds nothing. In Exarust the model's first root stands where the
object node does (it takes the placement with an identity transform), so the
leaf is the first model node whose parent is that root. Door models are a
frame with the leaves under it, so this is the first leaf.

## Door state

The room manager's three readers (the active-room walk, the reach query, the
spill) all test the same body byte, the pair passage flag the door step
writes through its passage setter: own flag `!locked && (shut || angle >
closed − 0.4)` for a hinge, `!locked && travel ≥ 0.8` for an open slider,
`!locked` when shut; the pair flag is the AND of both leaves. A link is open
when the flag is clear, so a door 0.4 rad or more open is open and a locked
door is always open. Exarust keeps this flag as `Door::both_passable`, so the
rule is `link_open(flag) = !flag` (no body: open).

## Room states

The original's update marks every link target of each active room adjacent,
and every link target of a dropped room hidden, without a door test; the
portal pass clips adjacent rooms through every doorway, also without one.
Only the wanted walk reads doors. So a shut door leaves the room behind it
adjacent (lid drawn), not hidden.

## Frame cost

One pass over the bound leaves (about a hundred in a story level), a hash
lookup each, and a tracker write only on change.

## Open questions

- Locked gates count as open, so the room behind a locked gate turns active
  within 2 m of its door point. Read in the assembly; not compared with a
  retail capture.
- 44 of exanima01's 140 door objects bind to no link (no link behind them
  from their cell); the same rule binds them in the original, not checked
  live.
