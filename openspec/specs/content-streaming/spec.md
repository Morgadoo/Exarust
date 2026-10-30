# content-streaming Specification

## Purpose
How content reaches the main thread as the original streams it: the texture
loader's request ring, promotion and upload passes, and the stream thread's
scheduler for placed level objects, generic jobs, database objects and
characters, with its result rings, main-thread pump and waits.

## Requirements

### Requirement: Texture loader queue
Texture loads SHALL be queued in request order in a ring that keeps one slot free and grows by 128 entries when full; a request for a name already queued SHALL be refused. Promoting a request SHALL move it to the front, fill its place with the last entry and mark it, and SHALL also find the request the loader holds, decoding it or waiting with it for an upload pass, but not one already in the upload list.

#### Scenario: Promotion
- **WHEN** a, b, c and d are queued and c is promoted
- **THEN** the queue reads c, a, b, d

### Requirement: Texture loader passes
The loader SHALL decode one request per pass. When it has decoded a request while earlier uploads are queued and the request was not promoted, it SHALL hold that request and start no other decode until an upload pass lets it go on; pausing SHALL only stop the wake-ups. An empty ring SHALL mark it idle, an upload pass SHALL not wake an idle loader, and a queued request SHALL clear the flag. A request that cannot be decoded SHALL upload the grey placeholder.

#### Scenario: Yielding
- **WHEN** a is decoded with no upload queued and b is decoded while a's upload waits
- **THEN** the loader goes on after a and waits after b, paused or not, until an upload pass
- **AND** after a promoted request it goes on

### Requirement: Texture upload pass
An upload pass SHALL upload only the entries ready when it started, in order, up to its budget, and run the completion callback after each. A priority load SHALL promote its request, decode it alone and upload it, and report a request that is neither queued nor held by the loader as not found, uploading nothing.

#### Scenario: A budget
- **WHEN** three uploads are ready and a pass has a budget of 1
- **THEN** the first is uploaded and the other two wait for the next pass

#### Scenario: The held record
- **WHEN** a's upload is queued and the loader holds b, decoded behind it
- **THEN** a priority load of b uploads a then b, and one of a is not found

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
The loader SHALL keep one in-progress mark per load, a finished decode SHALL clear only its own load's mark, and a texture decoded and handed to the upload list SHALL not count as a priority item unless the loader still holds it. A load released while it is decoding SHALL have its upload dropped.

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
The arrival variable SHALL give the operative kind that names the player: one kind for an arrival, another for a start or load, none otherwise. A placed object of the operative node kind SHALL be the player when its operative kind equals that kind. A placed object of the volume node kind SHALL be the player when its marker's entrance equals the arrival variable and its previous level equals the level the session came from (all ones for none). No other object SHALL be the player.

#### Scenario: An actor by kind
- **WHEN** a level begins on an arrival and holds actors of operative kind 15 and 18
- **THEN** the actor of kind 15 is the player

#### Scenario: A marker by arrival
- **WHEN** a marker's words are the arrival variable and the previous level's id
- **THEN** it is the player, and with any other previous level it is not

#### Scenario: The node kind decides first
- **WHEN** an object of operative kind 15 has a model and a door section, or a marker's record has a model
- **THEN** on an arrival neither is the player

### Requirement: Placed object node kind
The player test SHALL read the node kind the database builder gives a placed object: on a model tree the door kind when the record has a door section, else the operative kind when it has an operative section, and none for a weapon or an item body; the volume kind for a model-less record with a volume section; the operative kind for a model-less operative record's placeholder.

#### Scenario: A door before an operative
- **WHEN** a record with a model holds a door and an operative section
- **THEN** its node kind is the door kind, and without the model and with a volume section it is the volume kind

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

### Requirement: Transition markers
An object's volume section SHALL be read as three 16-bit extents, a 16-bit word, then the flags, the entrance and the previous level id; a body under 20 bytes SHALL give no volume. A volume whose flags' low byte is 0x40 SHALL be a transition marker, and a level's markers SHALL be listed with their placed positions.

#### Scenario: The first level's marker
- **WHEN** `exanima01` is read from the installation
- **THEN** it has one marker, at entrance 0 from locale 3, and the levels hold 54

### Requirement: Placed object flags
A placed object's object flags and its placement's static bit SHALL be kept; they mark only the object's root node, whose bits 0xB80000 SHALL be replaced by the object flags' same bits and which SHALL gain 0x80000 when the placement word has 0x10, and the object's bodies SHALL still register (or stay with their mechanism).

#### Scenario: A placed object marked static
- **WHEN** a placed object's object flags carry 0x80000
- **THEN** it reports static and its bodies are registered as before

#### Scenario: The root node word
- **WHEN** a root node word of 0xFFFFFFFF takes object flags 0
- **THEN** it reads 0xFF47FFFF

### Requirement: Streamed character handover
The pump's entry for a streamed character SHALL name the stage node the level placed and the built character; a requested or attached character SHALL have no stage node.

#### Scenario: Stage and built nodes
- **WHEN** a streamed character staged at node 3 builds node 99
- **THEN** the pumped entry names 3 as its stage and 99 as its node

### Requirement: Character size words
A character's two stored size words SHALL each scale the body's base scale by the word times 1/65535 in double precision, narrowed to single precision last, the second word's field written first. Only the streamed builder of a body that is not dead SHALL replace a pair whose second word is zero with 0xFFFF and 0xFFFF; the request builder and a dead body SHALL scale the words as stored.

#### Scenario: Half a word
- **WHEN** a word of 32768 scales a base of 1
- **THEN** the scale is 0.5000076

#### Scenario: An unset pair
- **WHEN** a living streamed body has the pair 0x8000, 0
- **THEN** both words become 0xFFFF and both scales equal the base
- **AND** a dead body keeps 0x8000, 0

### Requirement: Character request flags
A character request's bit 0x2000 SHALL add 0x1000 to the built character's node word, and its bit 0x1000 SHALL skip both starting the character's AI and restoring its saved AI pose (three floats and a fourth word).

#### Scenario: A marked request
- **WHEN** a request with 0x2000 builds a node whose word is 0x40
- **THEN** the node word is 0x1040

### Requirement: Saved character chunks
A streamed character's saved blob SHALL be a 32-bit length of the bytes that follow it and then tag, size and body records, scanned from byte 4 while the record lies within the stored length, keeping the last body of each of the flat pose, motion, pose tree and AI home tags; a malformed record SHALL end the scan.

#### Scenario: A repeated tag
- **WHEN** a blob holds two motion records, of one and of two bytes
- **THEN** the motion chunk is the two-byte body

### Requirement: Placed objects through the stream
A level load SHALL build its placed objects through the content stream's level pass, one entry a pass in file order within the step's budget, beginning the level with the session's arrival word and previous level (the words for none when nothing set them). An entry whose record is not in the database or whose model is not found SHALL count as missing; the placed player's position SHALL be reported with the level.

#### Scenario: A level without session words
- **WHEN** a level is loaded with no arrival word set
- **THEN** no placed object is the player and the missing count is at least the objects whose model was not found

### Requirement: Level wait report and handoff
The level wait SHALL report a non-zero missing count as "N missing database objects." under "Content management" and SHALL report nothing for zero. It SHALL hand the finished objects to the scene's grid environment when the scene has one and its room-mode word has either bit of 0x3, and SHALL append them to the caller's list otherwise; every shipped scene's room-mode word SHALL be 0x73.

#### Scenario: Thirty-seven missing
- **WHEN** a level ends with 37 missing objects
- **THEN** the report reads "37 missing database objects."

#### Scenario: A grid scene
- **WHEN** a scene with a grid environment has the room-mode word 0x73
- **THEN** the objects go to the grid environment, and with the word 0x70 they are appended
