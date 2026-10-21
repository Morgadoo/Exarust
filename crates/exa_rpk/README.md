# `exa_rpk`

Packs (`.rpk`), the project file (`Exanima.rfp`) and `GameData`, a read-only
view of an installation. No Bevy dependency.

**Phase:** 0. Status: implemented, verified on every retail pack.
Format spec: [`docs/formats/RPK.md`](../../docs/formats/RPK.md).

- `MappedArchive`: memory-mapped pack with a parsed table, case-insensitive lookup.
- `Archive`: the same over a borrowed slice, for packs nested in other packs (`.rml`, `.flb`, `.fds`, `actors.rpk`).
- `MemArchive`: a mutable in-memory pack (the original's in-memory block library): add by the lower-cased 16-byte name, payload replacement that closes the old hole and moves later offsets, deletion and the read-entry rule; entries stay in file order. `write` is Exarust's addition (the original never writes the library back).
- `Project`: the pack lists from `Exanima.rfp`, the path resolver (a drive path is absolute, anything else joins the project folder) and the user folder (`Userdata`, else `%APPDATA%/<name>`).
- `GameData`: opens the `Resource` pack, then the object and texture libraries the project names that are known packs (a known pack whose file is missing is an error; unknown names are skipped), then every other present pack by `PackKind`; exposes the project order of the object and texture libraries and the resolved user folder.
