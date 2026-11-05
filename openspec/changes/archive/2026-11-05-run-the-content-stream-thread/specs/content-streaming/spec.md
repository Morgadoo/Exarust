# Spec Delta

## ADDED Requirements

### Requirement: Requests stay queued while built
The stream thread SHALL peek a requested object or character, build it with no ring locked, push its result and only then pop the request; a fetch made during the build SHALL answer pending. A requested object that builds nothing SHALL raise the requested-object missing count.

#### Scenario: A fetch mid-build
- **WHEN** an object request is being built and its key is fetched
- **THEN** the fetch is pending, and after the build it returns the object

### Requirement: Stream thread
The stream's passes SHALL run off the main thread, one at a time, building the requests in the order they were posted; a post SHALL wake the thread when it is idle, and a terminated stream SHALL build nothing more.

#### Scenario: Posting order
- **WHEN** object 1, character 5 and object 2 are posted
- **THEN** they are built in that order and each is fetched by its key

### Requirement: Requested object builder
A requested object SHALL build only when its record names a model and the request has bit 0x10000000; a model the object libraries do not hold SHALL build nothing.

#### Scenario: A shipped model
- **WHEN** the carried torch's record is requested with 0x10000000
- **THEN** the thread builds it and a second fetch finds nothing

### Requirement: Streamed item icons
An item icon whose model is not loaded SHALL post its object request (its id, its render variant with 0x10000000, the icon scene) and wait for the fetch before its scene is made.

#### Scenario: A pack item
- **WHEN** the inventory shows an item whose model is not loaded
- **THEN** its icon is drawn from the model the stream built

### Requirement: The loaded player through the stream
A story load SHALL post the loaded player's request (0x1000, the world scene) with its body, morphed by the record's physique; the level's player spawn SHALL fetch it, waiting while it is built, and SHALL build the body itself after a wait cap.

#### Scenario: A story save's load
- **WHEN** a story slot is loaded
- **THEN** the player's body is built on the stream thread and the spawn adds it without building it
