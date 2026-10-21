# `exa_rpk`

Packs (`.rpk`), the project file (`Exanima.rfp`) and `GameData`, a read-only
view of an installation. No Bevy dependency.

**Phase:** 0. Status: implemented, verified on every retail pack.
Format spec: [`docs/formats/RPK.md`](../../docs/formats/RPK.md).

- `MappedArchive`: memory-mapped pack with a parsed table, case-insensitive lookup.
- `Archive`: the same over a borrowed slice, for packs nested in other packs (`.rml`, `.flb`, `.fds`, `actors.rpk`).
- `MemArchive`: a mutable in-memory pack: sorted add by the lower-cased 16-byte name, payload replacement that closes the old hole and moves later offsets, deletion, the in-memory read-entry rule, and a writer (byte for byte when unedited).
- `Project`: the pack lists from `Exanima.rfp`, the path resolver (a drive path is absolute, anything else joins the project folder) and the user folder (`Userdata`, else `%APPDATA%/<name>`).
- `GameData`: opens the project's resource pack and listed object/texture libraries (a missing listed pack is fatal), then every other present pack by `PackKind`; exposes the project order of the object and texture libraries and the resolved user folder.
