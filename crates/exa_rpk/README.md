# `exa_rpk`

Packs (`.rpk`), the project file (`Exanima.rfp`) and `GameData`, a read-only
view of an installation. No Bevy dependency.

**Phase:** 0. Status: implemented, verified on every retail pack.
Format spec: [`docs/formats/RPK.md`](../../docs/formats/RPK.md).

- `MappedArchive`: memory-mapped pack with a parsed table, case-insensitive lookup.
- `Archive`: the same over a borrowed slice, for packs nested in other packs (`.rml`, `.flb`, `.fds`, `actors.rpk`).
- `Project`: the pack lists from `Exanima.rfp`.
- `GameData`: opens every present pack by `PackKind`.
