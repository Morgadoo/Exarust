# user-settings Specification

## Purpose
The user's settings as the original keeps them: the text settings file with its typed
settings, rules, hotkey macros and line rewriters, the binary settings store of nested
chunks, the byte stream they are written with, and how the launcher uses them.

## Requirements

### Requirement: Settings file lines
The system SHALL read a settings file line by line (lines end at a line feed), skipping leading bytes up to space, ending the name at the first `=` (or, with none, the first space), matching names ignoring ASCII case, and ending the value one byte before the line feed less trailing spaces; the last line without a line feed SHALL run to the end of the file. An empty value or unknown name SHALL set nothing, and later lines SHALL win.

#### Scenario: CR LF and bare LF
- **WHEN** a file holds `GUISize = 12` followed by CR LF, or followed by a bare LF
- **THEN** the first reads 12 and the second reads 1

#### Scenario: Separators and case
- **WHEN** a file holds `fullscreen=1`, `Supersample 4` and `ab\x01TextureDetail=7` as its last line
- **THEN** Fullscreen is true, Supersample is 4 and TextureDetail is 7

### Requirement: Value conversion
The system SHALL convert integers as an optional `-` and leading digits cut to 32 bits, booleans as true when the text starts with `1` or is `true` or `yes` in any case, fractions with single-precision whole digits and double-precision decimals, and colours as hexadecimal shifted left 8 bits when six bytes or fewer.

#### Scenario: Kinds
- **WHEN** the values are `4294967301`, `YES`, `-0.5` and `ff8000`
- **THEN** they convert to 5, true, −0.5 and `0xff800000`

### Requirement: Graphics and window rules
The system SHALL start from the original's defaults (shading, shadow quality, shadow detail and texture quality 4, texture detail 3, vertical sync on, a 1600 × 1080 window, frame cap −1) and SHALL apply the handlers' rules: shading quality at least 3, shadow detail at least 3, texture quality and detail at least 2, supersample at most 4, a frame cap of 0 below a limit of 10 and otherwise 1000 / trunc(1000 / limit), and a positive GUI size of 1 on a display 800 pixels tall and 0 on any other.

#### Scenario: Clamps and frame cap
- **WHEN** the file sets ShadingQuality 1, TextureQuality 1, Supersample 9 and FPSLimit 60
- **THEN** shading quality is 3, texture quality 2, supersample 4 and the frame cap 62

#### Scenario: GUI size
- **WHEN** GUISize is 2 on a display 1080 pixels tall, 2 on one 800 tall, or −1
- **THEN** the GUI size is 0, 1, or −1

### Requirement: Hotkey macros
The system SHALL parse a `KeyDef` value `<shift|ctrl|alt>+<key>: cmd; …` into a modifier bit (0x20, 0x40, 0x80), a key byte moved down by 0x20 from `[` up, and commands split at `;` with their spaces trimmed and a trailing single byte dropped; only the first six SHALL be bound.

#### Scenario: A macro
- **WHEN** the value is `ctrl+k: spawn sword; heal  ;x`
- **THEN** the hotkey is ctrl with key `K` and the commands `spawn sword` and `heal`

### Requirement: Writing one setting
The system SHALL write a setting as `Name = value`, replacing the first occurrence of the name up to the next CR, or appending CR LF and the line when there is none. The game's one-setting save SHALL treat an occurrence at the very start as missing and SHALL start its replacement one byte before the name.

#### Scenario: Settings screen
- **WHEN** GUISize is set to 5 in `\r\nA = 1\r\nGUISize = 2\r\nB = 3`
- **THEN** the text becomes `\r\nA = 1\r\nGUISize = 5\r\nB = 3`

#### Scenario: The game's save
- **WHEN** the game saves GUISize 5 into the same text, or FirstRun 0 into an empty file
- **THEN** the text becomes `\r\nA = 1\rGUISize = 5\r\nB = 3`, and the empty file `\r\nFirstRun = 0`

### Requirement: Number formats
The system SHALL write integers as digits with a leading `-` and fractions with truncated digits and a last digit rounded to even that carries into the earlier digits and the whole part.

#### Scenario: Carry
- **WHEN** 0.999 and −1.999 are written with two decimals
- **THEN** they read `1.00` and `-2.00`

### Requirement: Settings store
The system SHALL keep a store of nested chunks (u32 tag, u32 size counting the header) addressed by tag paths, SHALL resize an existing leaf in place with every enclosing size following, SHALL append missing chunks at the end of the deepest existing one with their bytes added to it and its enclosing chunks, SHALL drop a loaded store whose first word is 0, and SHALL not write an empty store.

#### Scenario: A new path
- **WHEN** four bytes are stored at the bindings path in an empty store
- **THEN** the store is the outer chunk of size 20 holding the leaf of size 12 and the bytes

#### Scenario: A resized leaf
- **WHEN** a leaf at depth three grows from 4 to 12 bytes and then shrinks to 2
- **THEN** the enclosing sizes read 36 and 28, then 26 and 18, and a later sibling keeps its payload

### Requirement: Byte stream
The system SHALL set a stream's size to the cursor plus the bytes written on every fixed-size write, SHALL grow the block in 512-byte steps, and SHALL store Latin-1 text as its UTF-8 length and bytes.

#### Scenario: A write in the middle
- **WHEN** 604 bytes are written and a u16 is then written at offset 2
- **THEN** the capacity is 1024 and the size 4

### Requirement: Settings at launch
The launcher SHALL read the settings file from Exarust's own user folder (creating it empty when missing), SHALL apply the window size, full screen, vertical sync, supersampling and frame-rate readout the file names unless the command line sets them, SHALL write `FirstRun = 0` on a first run, SHALL use the store's key bindings, and SHALL skip the files with `--no-settings`.

#### Scenario: A window from the file
- **WHEN** the file sets WindowWidth 1280, WindowHeight 720 and VerticalSync 0 and no size is given on the command line
- **THEN** the window is 1280 × 720 without vertical sync
