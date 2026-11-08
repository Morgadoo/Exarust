## Why

PLAN Phase 6 (thaumaturgy), TPwrForceKinesis_S. The kinesis's line test
(the largest partial routine, 786 B) stood in the physics world's whole
fixed hit for the original's walk over the scene's nodes, so the props'
stand-ins blocked the line from the head and the doors' locks were not
read; the reach answer the hand asks while the pick waits (313 B) took any
movable prop wherever it was, without the original's sight test; and the
held body's user stamp was written once at the pick while the original's
tick writes it every tick.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The line test copies its two points, takes the segment's length and unit
  direction, asks the room manager's occluder query and then its lid query
  when the scene has rooms, and returns blocked on either. It then walks
  the scene's sectors on the caster's layer mask and each sector's nodes:
  a node passes when its class has all of 0x89, it shares the layer mask,
  it lacks the no-pick bit, and it has no object or its object is a door
  (the class test against the door's VMT) whose locked byte is clear. The
  segment is tested against the node's bounds, moved into the node's frame
  and handed to the mesh's ray callback with the length; any hit returns
  blocked. Props, characters and locked doors are passed over.
- The reach callback (installed by the kinesis control beside the pick
  callback) runs the screen pick at the mouse; on a hit it asks only the
  occluder query on the pick's ray up to the hit, raises the hit 5 cm,
  shortens the line from the caster's head to it by 5 cm
  (`(len − 5)/len`) and returns the line test's answer.
- The tick reads the head, steers, then writes every collected object's
  user (the caster) and its time (now + 2000) each tick, whether or not
  the cursor hit; the end clears the no-pick bits and gives the
  environment back and does not touch the stamps.
- The pick's child walk (a nested procedure on the pick's frame) collects
  the held object and all its descendants for the held environment, the
  no-pick bit and the tick's stamps; the reach counts the root's direct
  children.

## What Changes

- exa_game `thaum::kinesis_live`: `line_clear` (shells, then nodes, a line
  of no length blocked) and `line_skips` (every body's stand-ins passed
  over but an unlocked door's, from `DoorLocks` over the hinged and sliding
  door bodies); the steering's line from the head uses them instead of the
  cursor hit's closure.
- `KinesisSight` and `see_kinesis_reach` (Update, before the hand): the
  reach answer from the cursor's ray on the frame's physics replica, the
  occluder query and the line from the head (`sight_end`); the hand's
  `press_out_of_reach` takes it for the waiting kinesis.
- The steering tick stamps the held body with its caster for another 2 s
  every tick.

## Impact

While the player's kinesis waits, a crate behind a wall or a closed
unlocked door is out of reach; one seen past a table or another crate is
in reach. Held props steer past other props and characters standing in the
line from the head, and a locked door does not keep the aim, as in the
original. A body hurt more than 2 s into a hold still blames the caster.
Not covered: the screen pick's far plane (Exarust looks 1000 m), the
nodes' class and layer words (Exarust's fixed colliders stand for the
level's 0x89 nodes), sliding doors built as level geometry (they block
whether locked or not).
