## MODIFIED Requirements

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

### Requirement: Texture loader marks
The loader SHALL keep one in-progress mark per load, a finished decode SHALL clear only its own load's mark, and a texture decoded and handed to the upload list SHALL not count as a priority item unless the loader still holds it. A load released while it is decoding SHALL have its upload dropped.

#### Scenario: A newer load of the same texture
- **WHEN** a load of a texture is decoding and the same texture is queued again and taken
- **THEN** the first decode finishing leaves the newer load's mark in place

#### Scenario: Release while decoding
- **WHEN** a load is released while its decode runs
- **THEN** its upload is dropped when the decode finishes

### Requirement: Texture upload pass
An upload pass SHALL upload only the entries ready when it started, in order, up to its budget, and run the completion callback after each. A priority load SHALL promote its request, decode it alone and upload it, and report a request that is neither queued nor held by the loader as not found, uploading nothing.

#### Scenario: A budget
- **WHEN** three uploads are ready and a pass has a budget of 1
- **THEN** the first is uploaded and the other two wait for the next pass

#### Scenario: The held record
- **WHEN** a's upload is queued and the loader holds b, decoded behind it
- **THEN** a priority load of b uploads a then b, and one of a is not found

### Requirement: Player focus
The arrival variable SHALL give the operative kind that names the player: one kind for an arrival, another for a start or load, none otherwise. A placed actor SHALL be the player when its operative kind equals that kind. A transition marker SHALL be the player when its entrance equals the arrival variable and its previous level equals the level the session came from (all ones for none). No other object SHALL be the player.

#### Scenario: An actor by kind
- **WHEN** a level begins on an arrival and holds actors of operative kind 15 and 18
- **THEN** the actor of kind 15 is the player

#### Scenario: A marker by arrival
- **WHEN** a marker's words are the arrival variable and the previous level's id
- **THEN** it is the player, and with any other previous level it is not

## ADDED Requirements

### Requirement: Transition markers
An object's volume section SHALL be read as three 16-bit extents, a 16-bit word, then the flags, the entrance and the previous level id; a body under 20 bytes SHALL give no volume. A volume whose flags' low byte is 0x40 SHALL be a transition marker, and a level's markers SHALL be listed with their placed positions.

#### Scenario: The first level's marker
- **WHEN** `exanima01` is read from the installation
- **THEN** it has one marker, at entrance 0 from locale 3, and the levels hold 54

### Requirement: Placed object flags
A placed object's object flags and its placement's static bit SHALL be kept; they mark only the object's root node, and the object's bodies SHALL still register (or stay with their mechanism).

#### Scenario: A placed object marked static
- **WHEN** a placed object's object flags carry 0x80000
- **THEN** it reports static and its bodies are registered as before

### Requirement: Streamed character handover
The pump's entry for a streamed character SHALL name the stage node the level placed and the built character; a requested or attached character SHALL have no stage node.

#### Scenario: Stage and built nodes
- **WHEN** a streamed character staged at node 3 builds node 99
- **THEN** the pumped entry names 3 as its stage and 99 as its node
