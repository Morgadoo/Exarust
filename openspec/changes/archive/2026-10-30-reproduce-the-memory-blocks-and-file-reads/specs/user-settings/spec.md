## ADDED Requirements

### Requirement: Memory blocks
The system SHALL keep a memory block's logical size, allocation size and kind, SHALL resize a general block to exactly the size asked for, SHALL allocate global memory in 8192-byte steps and reallocate it only when the rounded size changes, SHALL refuse to resize resource memory except to zero, SHALL copy a borrowed view's first `min(old, new)` bytes into a new general block on its first resize, and SHALL copy a block into a destination by resizing the destination with its own kind's rules.

#### Scenario: Global steps
- **WHEN** a global block is resized to 1, 0x1fff, 0x2000 and 0x2001 bytes
- **THEN** its allocation is 0x2000, 0x2000, 0x2000 and 0x4000 bytes

#### Scenario: A borrowed view
- **WHEN** a borrowed view of four bytes is resized to eight, or to two
- **THEN** it becomes a general block holding its four bytes and four zeros, or its first two bytes

### Requirement: Whole-file reads
The system SHALL read a whole file by reference or over the search path, reporting an open failure as `Failed to open file: ` and a read failure as `Unable to read contents of file: ` followed by the name, and SHALL report nothing when no folder of the search path holds the file; the first folder that holds the file SHALL end the search even when its read fails.

#### Scenario: A miss on the search path
- **WHEN** no folder in use holds the file
- **THEN** the read fails with no log line

#### Scenario: The newest folder wins
- **WHEN** both an older and a newer folder hold the file
- **THEN** the newer folder's bytes are read

### Requirement: Settings store exit save
The system SHALL write the settings store on exit only when it holds bytes, and the exit hooks SHALL run latest registered first.

#### Scenario: An empty store
- **WHEN** an empty store is saved over an existing file
- **THEN** the file keeps its bytes

### Requirement: Hotkey macros across a re-read
The system SHALL append a re-read file's `KeyDef` macros after the macros already parsed, so only the first six macros ever parsed are bound.

#### Scenario: A seventh macro on a re-read
- **WHEN** a file with six macros is read and a file with one more is read into the same settings
- **THEN** the list holds seven macros and the seventh is unbound
