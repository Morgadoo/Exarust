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

Planned: `corpus` (parse every entry and report coverage) and `export-gltf`.
Extracted data stays on your machine; `/extracted/` is git-ignored.
