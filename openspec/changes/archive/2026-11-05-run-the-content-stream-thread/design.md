# Design

## The original

The unit's set-up makes ten rings and the thread: jobs (16), completions
(16), streamed characters' results (128), attachments (16), character
requests and results (32 each), object requests and results (32 each), and
installs its hooks. The thread loops until its terminate bit: while the
level's list is unfinished, its next entry; otherwise one job, one object
request and one character request, each when queued; only when none ran,
the nearest streamed character (its stage node gets the "building" bit
`0x100000` first); when that did not run either, it waits on its event.
Every post pushes under its ring's lock and signals the event.

Requested objects: the builder peeks the request, looks the record up by
id (not found: the missing counter), reads its model section (none: the
counter), and builds an `@` model with the item factory, any other from
the object library by name when the request has `0x10000000`. The node
takes the request's id, flags and scene. Under the result ring's lock the
node is pushed, then the request popped. The one poster is the item icon
cache, which always sets `0x10000000` and whose draw spins on the fetch
(a zero sleep and a texture upload pass a try).

Requested characters: built (the corpse builder for condition `0xA00`),
`0x2000` marks the node `0x1000`, `0x1000` skips the AI start and the
saved AI pose, the size words scaled; `0x400` sends the character to the
attachment ring, any other to the result ring; `0x100` frees the request
after. Posters: the story load (`0x1000`, the world scene), the character
creator, the game screen, the arena, the traders, the summoned demon, the
game main's named character (`0x500`).

## Exarust

Bevy's world lives on the main thread, so the stream thread builds what a
model takes on the CPU (`ModelJob::build`) and the main thread adds the
assets (`ModelLoader::finish_model`) and spawns. The thread is a task on
the async compute pool that runs passes until one is idle with no signal
since it began; a post starts it when it is not running. This is the
event's wait without parking a pool thread (the texture loader decodes on
the same pool).

The record lookup stays the poster's: the object database is the main
thread's catalogue, so a post names the record's model entry. A factory
weapon is still assembled on the main thread (its atlas and stats live in
the model cache). The loaded player's body job is prepared on the main
thread when it is posted (the morph tables and the unmorphed body are in
the cache) and built on the thread.

Keys: the icon scene and the world scene are Exarust's constants (the
original passes scene addresses).
