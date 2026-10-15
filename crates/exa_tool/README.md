# `exa_tool`

`exa-tool`: inspect and extract data from your own installation.
`cargo exa <command>` runs the release build.

| command | does |
|---|---|
| `info` | project file and pack inventory |
| `ls <pack> [--inner <entry>]` | entries with detected format |
| `stats <pack>` | entries and bytes per format |
| `chunks <pack> <entry> [--depth n]` | chunk tree of an `.rfc` |
| `rfi <pack> <entry>` | image header, format, mip count |
| `png <pack> <entry> <out.png>` | decode an image's base level |
| `extract <pack> <dir> [--filter s] [--inner e]` | write entries with guessed extensions |
| `corpus meshes` | decode every chunk tree; per-pack table and reasons for unsupported records |
| `corpus winding` | near-horizontal level faces up or down (grid pieces, level meshes, props), open downward faces, mirrored nodes |
| `export-gltf <pack> <entry> <out.glb>` | static meshes, node hierarchy and materials with embedded PNG textures |
Extracted data stays on your machine; `/extracted/` is git-ignored.
