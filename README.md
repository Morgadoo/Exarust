# Exarust

Exarust is an open-source reimplementation of the engine behind
[Exanima](https://store.steampowered.com/app/362490/Exanima/) by
[Bare Mettle Entertainment](https://www.baremettle.com/), written in Rust with [Bevy](https://bevy.org/) and wgpu.
Point it at a copy of Exanima you already own and it reads that
installation's packs (levels, models, textures, sounds) into its own engine.
The goal is a runtime that stays as close as possible to the original:
physically simulated characters, momentum-driven combat, layered armour,
procedural items, Story and Arena modes.

> **Status: Phase 0, foundations.** The data layer reads every retail pack
> and decodes every texture format. There is no gameplay yet. See
> [`docs/PLAN.md`](docs/PLAN.md) for the roadmap.

## What works today

- Opening an installation: `Exanima.rfp` and all 12 `.rpk` packs
  (memory-mapped, about 22 000 entries), including packs nested in other packs.
- Detecting every data format by its magic, with a documented status for
  each in [`docs/formats/`](docs/formats/README.md).
- Decoding textures: BC1/BC3/BC4/BC5, uncompressed and RLE-packed images, with mip
  chains.
- `exa-tool`, an inspector and extractor: `info`, `ls`, `stats`, `chunks`,
  `rfi`, `png`, `extract`.
- `exarust`, a Bevy window that opens the game data and shows a decoded
  texture ("first light").

## Build and run

You need Rust (rustup), GNU Make, the usual Bevy system libraries
([`docs/BUILD.md`](docs/BUILD.md)), and your own Exanima installation.

```bash
cp .env.example .env     # set EXANIMA_DIR to your Exanima folder
make info                # what's in the installation
make run                 # first light
cargo exa stats Objlib   # formats inside a pack
```

Exarust distributes no game assets and never modifies the installation.

## Layout

```
crates/
  exa_rpk        packs, project file, installation VFS          (implemented)
  exa_formats    format parsers: chunk trees, images, databases (partly implemented)
  exa_assets     Bevy bridge: GameData resource, loaders        (started)
  exa_world      levels: tile grids, sets, props, terrain       (skeleton)
  exa_render     materials, lighting, shadows, GI, post         (skeleton)
  exa_anim       skeleton, motion clips, target poses           (skeleton)
  exa_physics    rigid + articulated bodies, cloth              (skeleton)
  exa_character  bodies, morphs, apparel, stats, injuries       (skeleton)
  exa_items      item DB, factories, procedural materials       (skeleton)
  exa_combat     contact damage, armour layers                  (skeleton)
  exa_ai         perception, navigation, combat AI              (skeleton)
  exa_audio      positional PCM, reverb, sound sets             (skeleton)
  exa_script     dialogue, actors, journal                      (skeleton)
  exa_ui         menus, inventory, HUD, manual                  (skeleton)
  exa_game       modes, states, saves; composes all plugins     (skeleton)
  exa_tool       `exa-tool` CLI
  exarust        the game binary
docs/            plan, architecture, formats, research
```

## Documentation

- [Plan](docs/PLAN.md): phases, milestones, risks, next steps
- [Architecture](docs/ARCHITECTURE.md): crate layers, schedules, data flow
- [Formats](docs/formats/README.md): what is known about each retail format
- [Research](docs/INDEX.md): installation inventory, the engine's class map,
  community tools

## Acknowledgements and license

Inspired by [IW4L](https://github.com/vladtrc/iw4L), an open-source Bevy
runtime for Modern Warfare 2, whose layout and conventions this project
follows. Community reverse-engineering resources are credited in
[`docs/research/online-research.md`](docs/research/online-research.md).

Exarust's source is licensed under [Apache 2.0](LICENSE); see
[NOTICE](NOTICE). Exanima, its assets and trademarks belong to Bare Mettle
Entertainment. Exarust is not affiliated with them.
