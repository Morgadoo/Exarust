# User settings files

Not shipped: the original creates them in the user folder. Status:
**verified** against the 0.9.5 executable (every rule below was read from
its routines with REA/Ghidra and is reproduced in `exa_formats`); no user
file is part of the corpus, so the tests are synthetic.

| file | what | Exarust |
|---|---|---|
| `<project>.ini` | text settings, `Name = value` lines | `exa_formats::settings_ini` |
| `<project>.set` | binary settings store, nested chunks | `exa_formats::settings_store` |

`<project>` is the project file's name (`Exanima`). The user folder is the
installation's `Userdata` folder when it exists, otherwise the user's
application-data folder (`%APPDATA%`) and the project's name, which is
created when missing (`settings_ini::user_folder`).

Exarust keeps its own copies in `EXARUST_USER_DIR`, else the platform's
configuration folder and `exarust` (`~/.config/exarust/Exanima.ini`), so
the retail game's settings and first-run setup are never touched. The
launcher reads both files at start (`exarust/src/user_settings.rs`;
`--no-settings` skips them).

## Settings file

### Registry

Each part of the original registers its settings as `{name, kind,
handler}`. Kinds: 0 none (never converted), 1 text, 2 integer, 3 boolean,
4 fraction, 5 colour.

| part | settings (kind) |
|---|---|
| graphics and keys | `GUISize`, `HWMultisample`, `ShadingQuality`, `ShadowQuality`, `ShadowDetail`, `TextureQuality`, `TextureDetail`, `Supersample`, `FPSLimit` (2); `VerticalSync`, `Fullscreen`, `NoTopMost` (3); `KeyDef` (1) |
| launcher | `WindowWidth`, `WindowHeight`, `StereoMode` (2); `Touchscreen` (3) |
| hardware configuration | `PerfIndex` (4); `ShowFPS`, `Music`, `CamFollow`, `LinearRGB`, `SoftwareAudio`, `NoProcMips`, `NoDynamicMips`, `DynamicVBO`, `SwapCopy`, `ForceVBO`, `GILowPrecision` (3); `ShadowFmt`, `ScreenWidth`, `ScreenHeight`, `InputMode`, `AimSpeed` (2) |
| game | `CursorSize` (4); `CursorInt`, `CursorCom` (5); `FirstRun`, `ShowGameTips`, `CheckPoints` (3) |
| audio | `Volume` (4) |

### Reading

At start the file is read whole; when it cannot be read an empty file is
written in its place. Then, line by line (a line ends at a line feed):

1. Bytes up to space (`≤ 0x20`) at the start are skipped.
2. The name ends at the line's first `=`; with none, at its first space; a
   line with neither is skipped. Trailing spaces leave the name, and a byte
   up to space inside it restarts the name after that byte.
3. The name is looked up ignoring ASCII case; the first registered match
   wins.
4. The value starts after the separator and its spaces and ends **one byte
   before the line feed**, less trailing spaces. With CR LF that byte is
   the CR; with a bare LF it is the value's last byte (`GUISize = 12` LF
   reads 1). The last line, without a line feed, runs to the end of the
   file. An empty value sets nothing.
5. The value is converted by kind and handed to the setting's handler, in
   file order, so a later line wins.

Conversions:

- **integer**: an optional `-`, then digits to the first other byte (none
  gives 0), summed in 64 bits and cut to 32;
- **boolean**: true when the text starts with `1` or is `true` or `yes`
  (any ASCII case);
- **fraction**: an optional `-`; the whole digits are summed in single
  precision as `(c + v·10) − 48`; a `.` that is not the last byte starts
  the decimals, each digit d at place k adding d / 10ᵏ in double precision,
  rounded back to single;
- **colour**: hexadecimal digits (either case) to the first other byte; a
  value of six bytes or fewer is RGB and is shifted left 8 bits to RGBA.

### Graphics and window rules

Values before the file is read, and what each handler does:

| setting | default | rule |
|---|---|---|
| `GUISize` | 0 | stored; a positive value becomes 1 when the display is 800 pixels tall (which also sets a compact-display flag) and 0 on any other, so only 0 and negatives offset the GUI scale elsewhere ([UI.md](../UI.md), Interface scale) |
| `HWMultisample` | 0 | stored |
| `ShadingQuality` | 4 | at least 3; from 4 a render word is `0x2f3`, below it `0x273` (bit `0x80`); from 6 a further flag is set. The words' readers are not traced |
| `ShadowQuality` | 4 | stored |
| `ShadowDetail` | 4 | at least 3 |
| `TextureQuality` | 4 | at least 2 |
| `TextureDetail` | 3 | at least 2 |
| `VerticalSync` | true | stored |
| `Fullscreen` | false | stored; a full-screen window covers the display, otherwise the window is `WindowWidth` × `WindowHeight` |
| `NoTopMost` | false | stored |
| `Supersample` | 0 | at most 4 |
| `FPSLimit` | cap −1 | below 10 the cap is 0 (none); otherwise the frame time in whole milliseconds, trunc(1000 / limit), back to frames: 1000 / that (60 → 16 ms → 62; 144 → 166). A limit above 1000 divides by zero in the original; Exarust gives no cap |
| `KeyDef` | — | a hotkey macro (below) |
| `WindowWidth`, `WindowHeight` | 1600, 1080 | stored |
| `Touchscreen` | false | only a true value acts |

The defaults are the variables' initial values; the hardware configuration
may change them after the file is read (not reproduced).

### Hardware configuration settings

The hardware configuration registers these at start; their handlers store
the value (the ones below with a rule transform it first). What the renderer
switches do beyond that was not traced, so their effects are Exarust's
reading (see [UI.md](../UI.md), Settings options):

| setting | kind | default | rule |
|---|---|---|---|
| `PerfIndex` | fraction | 0 | the GPU benchmark's best rate over 60 |
| `ShowFPS`, `Music`, `CamFollow`, `LinearRGB`, `SoftwareAudio`, `NoProcMips`, `NoDynamicMips`, `DynamicVBO`, `SwapCopy`, `ForceVBO`, `GILowPrecision` | boolean | `Music` and `CamFollow` on, the rest off (`ShowFPS` and `Music` are the screen's own defaults; the others are guesses, see the code comments) | kept; `ShowFPS` feeds the readout, `GILowPrecision` is already Exarust's GI format |
| `ShadowFmt`, `ScreenWidth`, `ScreenHeight` | integer | 0 | kept; `ScreenWidth`/`ScreenHeight` are the full-screen mode's size |
| `InputMode` | integer | 0 | the controller when greater than 0 (the screen writes 0 or 51) |
| `AimSpeed` | integer | 30 (0.29) | the 1-based step: the aim speed is `(step − 1) / 99` in double precision, then single, unclamped (the screen writes `round(99·fraction) + 1`) |

The audio's `Volume` (a fraction in percent: the handler multiplies it by
0.01 in single precision, default 1.0) and the game's `ShowGameTips` and
`CheckPoints` (booleans, default on) are read beside them because the
settings screen's options use them.

### The settings screen's options

The hardware configuration's own defaults, before the file is read: volume
1.0 (0–1.5), music on, super sampling 0, shading quality 4 (at least 3),
texture detail 3 (at least 2), texture quality 4 (at least 2), shadow detail
4 (at least 3), shadow quality 4, hardware antialiasing 0, vertical sync on,
frame rate off, windowed, tips on, checkpoints on, aim speed 0.29
(displayed as 30). The slider steps and labels are in
[UI.md](../UI.md#settings-options).

Apply reads the file first (nothing happens when it cannot be read), then
writes `PerfIndex` (two decimals, trailing zeros kept), `Volume`
(percent), `Supersample`, `ShadingQuality`, `TextureDetail`,
`TextureQuality`, `ShadowDetail`, `ShadowQuality`, `HWMultisample`,
`InputMode` (`0`/`51`), `AimSpeed`, `Fullscreen`, `Music`, `ShowFPS`,
`ShowGameTips`, `Checkpoints`, `VerticalSync` and `ShowFPS` again, in that
order, through the settings screen's line rewriter, and writes the file. The
quality levels, super sampling, hardware antialiasing, vertical sync, full
screen and the second `ShowFPS` come from the screen's draft; the
performance index, volume, input mode, aim speed, music, the first
`ShowFPS`, tips and checkpoints are the live values (the Music checkbox only
mutes the session, so its line keeps the startup value). Opening the page
reads the file again into its draft.

The GPU benchmark warms up for half a second and takes three half-second
runs, the best frame count over its whole half-seconds divided by 60. The
performance index picks the quality levels listed in
[UI.md](../UI.md#settings-options); its top tier's super sampling comes from
`√(2304000 / (width·height)) × index`.

### Hotkey macros (`KeyDef`)

`<shift|ctrl|alt>+<key>: command; command; …`, one per line, up to six
bound. The modifier (before the `+`, trailing spaces trimmed, any case) is
bit `0x20` shift, `0x40` ctrl or `0x80` alt; anything else, no colon at
all, no colon after the key, or at most one byte after that colon and its
spaces adds nothing. The key is the first byte after the `+` and its
spaces; bytes from `[` up move down by 0x20 (lower case to upper). The
commands are split at `;`, lose leading and trailing spaces, and a single
byte left at the end is dropped. Every macro gets a command list; only the
first six are bound to a key, whose handler runs the commands through the
debug command registry (not reproduced).

The count of macros is the length of the settings unit's macro list,
which a re-read never clears: a second read of the file appends its macros
after the first read's, so past the sixth they get no key handler.
`Settings::reread` applies a file onto live settings and keeps the list.

### Writing one setting

A setting is written as `Name = value` (`" = "` between). Integers are
written as their digits with a leading `-` (`0` for zero; the smallest
integer, whose negation overflows, writes as `-`); booleans as `1`/`0`;
fractions with a number of decimals: truncated digits of the absolute
fraction and a last one rounded to even, carrying into the earlier digits
and, when they are all nines, into the whole part by the value's sign;
trailing zeros optionally removed down to one; a zero whole part is `0` or
`-0` (`settings_ini::format_*`).

There are two line rewriters. Both search the whole file for the first
occurrence of the name, matching case (the search is a plain substring
search, 0-based):

- **settings screen** (`put_setting`): the bytes from the name to the next
  CR (or to the end) are replaced by the line; with no occurrence, CR LF
  and the line are appended. The settings screen rewrites `PerfIndex` (2
  decimals), `Volume`, the quality levels, `InputMode`, `AimSpeed`,
  `Fullscreen`, `Music`, `ShowFPS`, `ShowGameTips`, `Checkpoints` and
  `VerticalSync` this way, then writes the file.
- **the game's own save** (`save_one_setting`): as above, but an
  occurrence at the very start of the file counts as missing (the line is
  appended), and the replacement starts **one byte before** the name, so
  the line feed before it is lost and the line joins the one above (which
  then reads a value running into the next name). The game uses it on a
  first run only: `FirstRun = 0`, and on some systems `CursorSize = 150`,
  `GUISize = 1`, `GILowPrecision = 1`. A fresh file therefore reads
  `\r\nFirstRun = 0`.

The splice behind both rewriters moves the bytes after the line with a
length that is `at` bytes too long (the original's count omits the start
offset), so the original writes past the end of its block. The bytes it
keeps are the ones a correct splice keeps, so Exarust copies exactly the
tail and documents the overrun instead of reproducing it
(`settings_ini::splice` via `text::splice`).

Evidence: the settings registry, the reader, the boolean test, the two
rewriters with their splice routine, the number formatters and the
handlers' code in the graphics-and-keys cluster, the hardware
configuration's save and the launcher's and game's registrations, read
with REA (instruction listings where the decompiler dropped an index
register or a jump target).

## Settings store

`<project>.set` is a tree of chunks: `u32 tag`, `u32 size` (counting its
own 8-byte header), then the payload or child chunks. Settings are
addressed by tag paths; the player's key bindings live at
`0x6440A401 / 0x6440CB01` (payload: [PLAYER.md](../PLAYER.md), Bindings:
version 101, controller flag, count 23, the saved table's rows). The
account-wide story experience is the four bytes at the one-tag path
`0x6431AC04`, written little-endian; an absent or empty value leaves the
number the game holds.

- **Load**: the file is read whole at start. Contents whose first word is 0
  fail the check: they are dropped and "Set file integrity check failed."
  is logged. A missing file leaves the store empty.
- **Get**: the path is walked one tag per level; each level scans its
  siblings in order, enters the first chunk with the tag, and stops at a
  sibling of size 0. The leaf's payload is its bytes after the header.
- **Set**: the deepest chunk on the path that exists is found. If it is
  the leaf, its payload is resized in place: every enclosing chunk's size
  moves by the payload's change and the bytes after it shift. Otherwise
  the missing chunks are written at the end of that chunk (or of the store
  when none exists), each intermediate one sized to hold the rest, and
  their bytes are added to that chunk and every chunk enclosing it.
- **Save**: on exit, through the program's exit hooks, when the store is
  not empty. The bindings dialog also saves it after storing the bindings.
  Registering a hook puts it at the head of a chain and a hook calls the one
  registered before it; the settings hook saves, then chains, so the store is
  written before the earlier hooks run (`settings_store::ExitHooks`,
  `SettingsStore::save_to`).

Evidence: the store's path builder, loader, getter, setter with its
ancestor walk and one-level descent, and the save and exit hook (component
`RF_StoreSettings`), and the bindings dialog's writer and reader.

## Byte stream

The original's binary writers and readers share one stream: a block (size
and capacity) and a cursor (`exa_formats::data_stream::DataStream`).

- Every fixed-size write (bytes, u8, u16, u32, f32) sets the size to the
  cursor plus the bytes written, so writing in the middle cuts the stream
  there. When that passes the capacity the block grows to the size rounded
  up to 512.
- A text line is its bytes then CR LF (written as the u16 `0x0A0D`).
- A string is its u32 length and, when not empty, its bytes; reading one
  with length 0 leaves the destination unchanged.
- Latin-1 text is stored as its UTF-8 length (u32) and UTF-8 bytes: bytes
  below `0x80` as they are, `0x80`–`0xBF` as `C2 b`, `0xC0`–`0xFF` as
  `C3 (b − 0x40)`. This write only raises the size and resizes the block to
  exactly the end. Reading back keeps bytes below `0x80`, turns `C2 b`
  into b and `C3 b` into b + 0x40, and drops other sequences.
- Raw reads copy from the cursor and advance it; past the size they are an
  error in Exarust (the original reads on).

## File plumbing

`exa_formats::file_ref`:

- **Folder table**: folders are registered with a trailing `\` and a use
  count; a path without a drive colon is taken from the base folder. A
  registration matches an entry of the same length as the path given and
  the same bytes, so a path given without its `\` is added again.
  Indexes are 1-based; 0 means no folder.
- **File reference**: a folder index and a name kept inline up to 16 bytes
  (longer names on their own). A whole-file read opens the folder's path
  and the name (the name alone without a folder); the full path is the
  long name itself when it holds a `\` after its first byte, else the
  folder (the base folder without one) and the name.
- **Entry stem**: an inline name cut at its last dot (after the first
  byte); a long name from after its last `\` to its last dot, zero-padded
  to 16, or its first 16 bytes from there. Without a dot the length wraps
  around the name and its last byte is lost.
- **Extension**: a dot after the first byte and after the last `\` keeps
  the name up to it; otherwise a dot and the extension are appended.
- **Folders made for a path**: the first window starts two bytes past a
  drive colon (or at the second byte); each folder is created up to the
  next separator, and the walk stops when a window opens on a separator.
- **Search path**: a name is read from the newest folder still in use
  where it exists; the first folder where it exists ends the search even
  when the read fails.
- **Whole-file reads** (`read_file`, `read_referenced_file`,
  `read_from_search_path`): a failed open leaves the block alone and logs
  `Failed to open file: <path>` at level 3 under the topic `File IO`, only
  while the error log is switched on; a failed read always logs `Unable to
  read contents of file: <path>`, and the read by reference then clears the
  block (the search-path read leaves it at the file's size). The search
  path logs the bare name, and when no folder holds the file it logs
  nothing. Exarust returns the reason as `FileIssue` (`message` gives the
  log line, none for `Missing`).
- **Exists**: `file_exists` is `GetFileAttributesA` — true for a file or a
  folder.
- **Resources** (`read_resource`): a named payload of the executable under
  the `RT_RCDATA` type (10), the name taken from a reference (an inline name
  up to its first zero, a long name whole). On success the block holds the
  bytes and is resource memory — only released, never reallocated — and a
  missing resource leaves the block as it was. The resource directory walk
  itself is the platform's and is passed in.
- **Memory blocks** (`exa_formats::mem_block`): the block the reads fill
  has a logical size, an allocation size and a kind. A general block grows
  to exactly the size asked for and frees at zero; global memory is
  allocated in `0x2000`-byte steps and only reallocated when the rounded
  size changes; a resource's memory only accepts a resize to zero; a
  borrowed view copies the smaller of its old and new sizes into a fresh
  general block on its first resize. `copy_into` resizes a destination
  block by its own kind's rules to the source's size and copies the
  destination's new size of bytes (`duplicate` does it into a new general
  block); `make_owned` gives a borrowed view its own
  copy. The original's two resize routines differ only in the allocator they
  call for a general block, so one `resize` serves both.
- **Format registry**: `{key[16], handler, description, use count}`; a new
  format goes in below the run of used formats at the end (never below
  index 1); lookup matches the 16 key bytes ignoring ASCII case.

Not reproduced: the platform's resource-directory walk (the resource rule
around it is `file_ref::read_resource`) and the error log's switches.
