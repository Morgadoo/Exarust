# Spec Delta

## ADDED Requirements

### Requirement: Header camera pair
Both writers SHALL store the view's camera target (centimetres) and zoom word in the header after the locale, from the host's camera when it provides one and the origin and zoom 1 otherwise. Both readers SHALL read the pair and drop it: a read session SHALL keep the origin and zoom 1.

#### Scenario: Write only
- **WHEN** a story session with camera (123.5, −45, 678.5) and zoom 1.75 is saved and read back
- **THEN** the header holds those values and the read session has the origin and zoom 1

### Requirement: Save disable in the live saves
While the last level change has saving disabled, a new-slot save and a flow save step SHALL write nothing and SHALL not apply death. A checkpoint step SHALL skip only the save: when no checkpoint exists the existing save SHALL still be copied over it.

#### Scenario: Stale checkpoint
- **WHEN** saving is disabled, the slot's save holds old bytes and no checkpoint exists, and a checkpoint step runs
- **THEN** the save is unchanged and the checkpoint holds the same old bytes

### Requirement: Checkpoint step order
A checkpoint step SHALL, only when the slot has no checkpoint, write the save without the death check and then copy it over the checkpoint; an existing checkpoint SHALL be left unchanged.

#### Scenario: Existing checkpoint
- **WHEN** a checkpoint already exists and a checkpoint step runs
- **THEN** its bytes do not change

### Requirement: Retail save restore
A story or arena save without an Exarust payload SHALL restore the reader's globals: the clock, the seed, the start background, the journal chunk or the single "New Entry" page, the discovery, global experience, memory and transition chunks, and the level from the header's locale or `exanima05` when the transition state is 2; an arena save SHALL restore the clock, copper, day and the tables it carries.

#### Scenario: Forced locale
- **WHEN** a retail story section names `exanima02` and its transition state's first word is 2
- **THEN** the restored session's level is `exanima05`

### Requirement: Converted arena record on load
The live arena load SHALL apply the old arena record repair to the player's record when the save was converted from an older data version, before the identity reset.

#### Scenario: Converted load
- **WHEN** a converted save's player record wears 1, 0x80000002, 3 and carries 0x80000001, 6
- **THEN** the restored record wears 1, 3 and carries 6
