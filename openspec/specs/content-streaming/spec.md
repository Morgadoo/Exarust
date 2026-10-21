# content-streaming Specification

## Purpose
How content reaches the main thread as the original streams it: the texture
loader's request ring, promotion and upload passes, and the stream thread's
scheduler for placed level objects, generic jobs, database objects and
characters, with its result rings, main-thread pump and waits.

## Requirements

### Requirement: Texture loader queue
Texture loads SHALL be queued in request order in a ring that keeps one slot free and grows by 128 entries when full; a request for a name already queued SHALL be refused. Promoting a request SHALL move it to the front, fill its place with the last entry and mark it, and SHALL also find a request decoded and waiting for its upload.

#### Scenario: Promotion
- **WHEN** a, b, c and d are queued and c is promoted
- **THEN** the queue reads c, a, b, d

### Requirement: Texture loader passes
The loader SHALL decode one request per pass and SHALL wait while uploads are pending unless the request it decoded was promoted; pausing SHALL only stop the wake-ups. A request that cannot be decoded SHALL upload the grey placeholder.

#### Scenario: Yielding
- **WHEN** a decoded request waits for its upload
- **THEN** the loader waits before its next decode, paused or not
- **AND** after a promoted request it goes on

### Requirement: Texture upload pass
An upload pass SHALL upload only the entries ready when it started, in order, up to its budget, and run the completion callback after each. A priority load SHALL promote its request, decode it alone and upload it, and report a request that is neither queued nor waiting as not found.

#### Scenario: A budget
- **WHEN** three uploads are ready and a pass has a budget of 1
- **THEN** the first is uploaded and the other two wait for the next pass

### Requirement: Content stream pass
One pass of the stream thread SHALL build only the next placed level object while the level's list is unfinished. Otherwise it SHALL run one generic job (queueing its completion for the main thread), one requested database object and one requested character, each when one is queued, in the same pass; a request that builds nothing SHALL be dropped.

#### Scenario: One pass for the requests
- **WHEN** a job, an object request and a character request are queued after a level's two objects
- **THEN** two passes build the level objects and the third runs the job, the object and the character

### Requirement: Streamed characters
Only when a pass ran no job, object or character SHALL it build the pending streamed character nearest the focus point by squared distance, the first of equal distances winning; else the thread SHALL sleep.

#### Scenario: Nearest first
- **WHEN** streamed characters at 5, 1, 3 and −1 m from the focus are pending
- **THEN** they are built in the order 1, −1, 3, 5

### Requirement: Texture loader marks
The loader SHALL keep one in-progress mark per load, a finished decode SHALL clear only its own load's mark, and a texture decoded and waiting for its upload SHALL not count as a priority item. A load released while it is decoding SHALL have its upload dropped.

#### Scenario: A newer load of the same texture
- **WHEN** a load of a texture is decoding and the same texture is queued again and taken
- **THEN** the first decode finishing leaves the newer load's mark in place

#### Scenario: Release while decoding
- **WHEN** a load is released while its decode runs
- **THEN** its upload is dropped when the decode finishes

### Requirement: Content stream results
Built database objects and requested characters SHALL wait in rings of 32 until the caller fetches them by the key it asked with; the rest of each ring SHALL keep its order. The main-thread pump SHALL run the completion callbacks, then hand over the streamed characters to insert into the scene (a ring of 128), then the characters requested for an existing entity (flag 0x400, a ring of 16).

#### Scenario: Fetch and pump
- **WHEN** a requested character without flag 0x400 and a streamed character are built and the pump runs
- **THEN** the pump hands over the streamed character only, and the requested one is fetched by its owner and scene

### Requirement: Object fetch defaults
A fetch SHALL always search the result ring first; when the key is not there, it SHALL return the null node while object requests are queued and the scene's nil node otherwise.

#### Scenario: A ready object while requests are queued
- **WHEN** an object is built and another object request is still queued
- **THEN** fetching the built object returns it, not the default

#### Scenario: The default
- **WHEN** a key is not in the ring and an object request is queued, then none is
- **THEN** the fetch gives the null node, then the scene's nil node

### Requirement: Content stream waits
Waiting for the level SHALL return its built objects and the number missing and clear the list. Waiting for characters SHALL drain the attachments each round and stop once the request ring was empty at the round's start.

#### Scenario: A level with a missing object
- **WHEN** a level of three objects whose second builds nothing is waited for
- **THEN** two objects and one missing are returned, and a second wait returns none

### Requirement: Player focus
The level's arrival variable SHALL give the operative kind that names the player: one kind for an arrival and another for a session start or load, and none otherwise. A placed actor SHALL be the player when its operative record's kind equals that kind. A marker record SHALL be the player when its first word equals the current arrival variable and its second word is all ones. No other object SHALL be the player, whatever its own kind.

#### Scenario: An actor by kind
- **WHEN** a level begins on an arrival and holds actors of operative kind 15 and 18
- **THEN** the actor of kind 15 is the player

#### Scenario: A marker by arrival
- **WHEN** a marker record's first word is the arrival variable and its second is all ones
- **THEN** it is the player

### Requirement: Missing level slots
A failed fetch for a level object SHALL raise the missing count and SHALL not advance the slot.

#### Scenario: One slot fails
- **WHEN** the second of three level objects builds nothing
- **THEN** two objects and one missing are returned

### Requirement: Frame order of the loader
A frame SHALL collect the finished decodes, pump the ready uploads, start the next decodes and pump the uploads again, so the loader's yield never leaves it idle.

#### Scenario: Uploads waiting at the frame's start
- **WHEN** uploads are ready and decodes are queued at the start of a frame
- **THEN** the uploads are pumped before the next decodes start

### Requirement: Streaming in the running game
The game SHALL decode textures on Bevy's task pool, at most four in flight, collect finished decodes oldest first and upload them in the frame. A streamed texture SHALL replace its placeholder under the same handle, and the materials and cube shadow bind groups that held the placeholder SHALL be rebound. An unattended screenshot SHALL wait, up to a cap, for the loader to be idle.

#### Scenario: A streamed texture arrives
- **WHEN** a decoded image replaces its 1×1 placeholder
- **THEN** the materials that used the placeholder show the real image on the next frame
