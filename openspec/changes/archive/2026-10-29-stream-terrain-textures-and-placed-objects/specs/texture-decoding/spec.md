## ADDED Requirements

### Requirement: Terrain texture path
The terrain's material textures SHALL be taken from a live record when one exists and otherwise queued for the texture loader behind the 1×1 placeholder with the streaming status alone, never decoded by the load step; a name the game does not ship SHALL give no texture. A terrain tile SHALL be spawned only once its materials' textures have arrived.

#### Scenario: A queued terrain texture
- **WHEN** a terrain material's diffuse map is asked for while no record holds it
- **THEN** a load is queued behind the placeholder and the tile waits until its upload replaces it

### Requirement: Waiting texture finished at once
A caller that loads the level in one call SHALL finish a record still holding its placeholder at once, with the word the record was created with and without taking another reference.

#### Scenario: Finished on the spot
- **WHEN** a record holding its placeholder is finished at once
- **THEN** the real image is under the same handle, the queued load is dropped and the record keeps one reference

### Requirement: Fast-load flag
With the texture manager's fast-load flag set, a load the material path queues SHALL start as the loader's priority item, so its decode does not wait for the upload pass; the terrain path SHALL ignore the flag. The flag SHALL be off, as the shipped game never sets it.

#### Scenario: The flag set
- **WHEN** the flag is set and a material texture and a terrain texture are queued
- **THEN** the material's load is the priority item and the terrain's is not
