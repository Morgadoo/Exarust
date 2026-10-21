# Design

## Context

The script lane's parser keeps each section parsed (`Dialogue`, script
bytes); the roles lane's kept the raw bytes after the entry head and found
sections on demand. Both walk the same order for bits `0x1`, `0x2`, `0x4`,
`0x8`, `0x10` and `0x40`.

## Decisions

1. Keep the parsed form: the dialogue engine needs the graphs, and a
   parsed entry has every section the finder can return.
2. Read the corpse script at the reader's position after section `0x8`
   and rewind, so sections `0x10` and `0x40` are read from the same place,
   as the original's two accessors do.
3. The finder returns the entry; callers read `dialogue` or `script(bit)`.
4. A section running past its entry fails the role (the script lane's
   rule); the roles lane returned no section instead. No shipped record
   does either.
