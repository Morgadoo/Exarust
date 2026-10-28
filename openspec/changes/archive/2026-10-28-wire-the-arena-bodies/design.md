# Design

## Generator

`SceneCommand::{Spawn, Despawn, Place}` messages are applied in `Update` by
one system. It returns at once on frames without a command and keeps the
object database, race table and role book in a `Local` after the first
command, so a level that never asks for a body pays nothing. A body is a root
(position, `CharacterBody` capsule, room tracking) and a body model child
carrying every component the level's placed characters carry plus a
`SceneFighter` marker (team, company side, id, condition words, first action
time, behaviour reset). Despawn resolves a marked model to its root, removes
the capsule from the physics world (when the body still has one) and
despawns the subtree.

## Looks

The original writes a random NPC record and dresses it with the outfit
generator, whose item-fact pass is not built. Generated fighters, company
entrants and specialists take a practice-opponent actor template picked from
level and tier [guess]; the shopkeeper takes the exact `arenashpkp` template.

## Process boundary

A level change in process is blocked (physics colliders without an owner map,
load-time GI, rooms, nav and cloth). The hub writes the career and the match
setup to a versioned, total hand-off file before restarting on the arena; the
match writes the settled career and the return placement before restarting on
the hub. Each process reads the file once at `PreStartup` and removes it.

## Hub mode

The play view switches the flow to its practice stand-in after the player
spawns. The hub entry reads the mode as the click hook does: practice or menu
count as the arena while the hub level is loaded; story mode does not.
