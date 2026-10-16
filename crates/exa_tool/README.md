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
| `paint <model> <stem> <out.png>` | paint an apparel texture set with the appearance of the first item worn as the model (`paint braies_a braies__aa b.png`) |
| `races` | every race of `races.rdb`: names, model, movesets, health, flag words, the body scalars and the body controller's factors, the stats (reach, weapon factors, AI, footsteps, voice) and the gait table with brake distances |
| `characters <level\|actors>` | every character record of a level (`exanima01`) with where it is placed, or of the arena's actors: looks, feral level, pools, combat skill, experience, traits, voice, skills with their learnt techniques, perks, Manoeuvring tier, Deflection, roles, saved state, powers, and the stats each record spawns with (scale, most energy, reach, pins, head, health, rule bits) |
| `bodies <pack> <entry>` | the rigid bodies of a model's nodes (root or child, class, mobility, rest points, point masses, spheres, links with slider travel, sound slots) and its cloth markers, in cm (`bodies Objects "lever a01"`) |
| `mechanisms [level]` | the door and operative records of `objdb.rdb`, or of a level's own database (`mechanisms exanima01`), with the body class each gives its model |
| `corpus bodies` | per model pack: models with bodies, undecodable bodies, models with links and hung from the world, slider bodies, cloth markers |
| `corpus meshes` | decode every chunk tree; per-pack table and reasons for unsupported records |
| `corpus levels` | grids, props and placed characters of every level; placed humans covered at the groin and their worn items with a `Characters.rpk` model |
| `corpus images` | every image by format word and storage; decodes the uncompressed ones and checks that RGB8 normal maps hold z in blue |
| `corpus winding` | near-horizontal level faces up or down (grid pieces, level meshes, props), open downward faces, mirrored nodes |
| `nav <level> [--png out.png]` | the level's navigation grid built as the game builds it at load: size, inputs, columns by span count, floor, passable and open-sky cells per layer; the picture shows both layers ([`docs/NAVIGATION.md`](../../docs/NAVIGATION.md)) |
| `export-gltf <pack> <entry> <out.glb>` | static meshes, node hierarchy and materials with embedded PNG textures |
Extracted data stays on your machine; `/extracted/` is git-ignored.
