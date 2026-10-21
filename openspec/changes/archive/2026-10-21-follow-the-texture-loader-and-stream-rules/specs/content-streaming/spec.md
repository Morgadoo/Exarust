# Spec Delta

## MODIFIED Requirements

### Requirement: Texture loader passes
The loader SHALL decode one request per pass and SHALL wait while uploads are pending unless the request it decoded was promoted; pausing SHALL only stop the wake-ups. A request that cannot be decoded SHALL upload the grey placeholder. The loader SHALL keep one in-progress mark per load, a finished decode SHALL clear only its own load's mark, and a texture that is decoded and waiting for its upload SHALL not count as a priority item. A load released while it is decoding SHALL have its upload dropped. A running frame SHALL pump the ready uploads before it starts the next decodes and again after, so the yield never leaves the loader idle.

#### Scenario: Yielding
- **WHEN** a decoded request waits for its upload
- **THEN** the loader waits before its next decode, paused or not
- **AND** after a promoted request it goes on

#### Scenario: A newer load of the same texture
- **WHEN** a load of a texture is decoding and the same texture is queued again and taken
- **THEN** the first decode finishing leaves the newer load's mark in place

#### Scenario: Release while decoding
- **WHEN** a load is released while its decode runs
- **THEN** its upload is dropped when the decode finishes

### Requirement: Content stream results
Built database objects and requested characters SHALL wait in rings of 32 until the caller fetches them by the key it asked with; the rest of each ring SHALL keep its order. A fetch SHALL always search the result ring first; when the key is not there, it SHALL return the null node while object requests are queued and the scene's nil node otherwise. The main-thread pump SHALL run the completion callbacks, then hand over the streamed characters to insert into the scene (a ring of 128), then the characters requested for an existing entity (flag 0x400, a ring of 16).

#### Scenario: Fetch and pump
- **WHEN** a requested character without flag 0x400 and a streamed character are built and the pump runs
- **THEN** the pump hands over the streamed character only, and the requested one is fetched by its owner and scene

#### Scenario: A ready object while requests are queued
- **WHEN** an object is built and another object request is still queued
- **THEN** fetching the built object returns it, not the default

#### Scenario: The default
- **WHEN** a key is not in the ring and an object request is queued, then none is
- **THEN** the fetch gives the null node, then the scene's nil node

## ADDED Requirements

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

### Requirement: Streaming in the running game
The game SHALL decode textures on Bevy's task pool, at most four in flight, collect finished decodes oldest first and upload them in the frame. A streamed texture SHALL replace its placeholder under the same handle, and the materials and cube shadow bind groups that held the placeholder SHALL be rebound. An unattended screenshot SHALL wait, up to a cap, for the loader to be idle.

#### Scenario: A streamed texture arrives
- **WHEN** a decoded image replaces its 1×1 placeholder
- **THEN** the materials that used the placeholder show the real image on the next frame
