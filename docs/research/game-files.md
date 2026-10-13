# Retail installation analysis

Source: Steam build of Exanima (app 362490) as installed on 2026-10-13. All
numbers come from `exa-tool` and hex inspection of that install.

## Files

| file | size | role |
|---|---:|---|
| `Exanima.exe` | 3.9 MB | Win64 PE, Free Pascal 3.2.2, GUI subsystem, timestamp zeroed |
| `Exanima.rfp` | 153 B | project: resource pack, object/texture/material libraries |
| `OpenAL64.dll` | 1.8 MB | OpenAL Soft |
| `oalsoft.ini` | 48 B | `hrtf = false`, `front-stabilizer = true` |
| `steam_api64.dll` | 0.3 MB | Steamworks |
| `Resource.rpk` | 438 MiB | levels, terrains, base meshes, UI images, databases, shaders, skeleton |
| `Objlib.rpk` | 213 MiB | 4926 static props and level pieces |
| `Textures.rpk` | 3.6 GiB | 3320 world textures, up to 4096² |
| `Objects.rpk` | 35 MiB | 1253 interactive items |
| `Characters.rpk` | 34 MiB | 356 wearable meshes |
| `CharExt.rpk` | 0.3 MiB | 12 hair meshes |
| `Components.rpk` | 6.8 MiB | 1001 weapon parts |
| `ComponentTex.rpk` | 120 MiB | 3361 weapon part textures |
| `Apparel.rpk` | 1.1 GiB | 1939 apparel textures |
| `Emblems.rpk` | 10.5 MiB | 135 heraldry images |
| `Factories.rpk` | 0.2 MiB | 106 weapon recipes |
| `Sound.rpk` | 2.3 GiB | 1131 PCM WAVE files |

## `Resource.rpk` contents (243 entries)

- **Levels** (`.rfc` + optional `.rft` terrain): `exanima01`–`08`,
  `exanima05_sw`, `exanima07_sw`, `exanima67`, `exanima67_c1`, `exanimac1`;
  arenas `arena`, `arena1`–`7`, `arenae`, `arenahub`, `arenao1`–`5`;
  `menuscene`.
- **Level piece sets**: `gews_*` wall sets (34, up to 7.5 MB each),
  `gefs_*` floor sets (15).
- **Base bodies and morphs**: `humanbase`, `humanmorph`, `ancientbase`,
  `ancientmorph`, `ogrebase`, `golembase`, `golembfbase`, `golembmbase`,
  `glmbsabase`, `glmstabase`, `demona01base`, `skelbase`, `tntclsklbase`,
  `wraithhabase`, `gobblerbase`, `hrtafaelbase`, `hrtaghlabase`,
  `hrtaghlhbase`, `hrtahrvsbase`, `hrtogrbase`, `dummyhead`.
- **Terrain plants**: `trn_grassa01/02`, `trn_lplnta01/02`, `trn_splnta01/02`.
- **Terrain tables** (`.ftb`): `default`, `desert`, `grassland b`, `grslndc`,
  `seaside`, `vulcanic`, `uwa1`, `uwa1b`, `uwb6`, `uwb6a`, `uwb6b`, `uwb6g`,
  `uwb6gc`, `uwswra1`.
- **Character textures**: skin diffuse/normal/specular sets (`human*`,
  `ancient*`), beards `bearda01..06 d/n`, `humanseams`, `skindecomp`,
  `spatter01`.
- **UI images**: fonts (`fontbase24r`, `titlefont`), cursors, GUI skin and
  textures, icon sheets (arsenal, coins, crystals, skills, powers),
  tutorial cards (`tut*`), title (`maintitle01`, `bmelogol01/02`), season
  rewards, divination cards, domain wheel, compass.
- **Data**: `races.rdb`, `charroles.rdb`, `locales.rdb`, `objdb.rdb`,
  `objstrings.rdb`, `actors.rpk`, `narrator.rcd`, `humanoid.rsq`, powers
  `pwr_displace/energy/force/mind.pwr`.
- **Libraries**: `common.rml`, `underworld0.rml`, `underworldb.rml`,
  `humanbase.rml` (materials); `shading.flb`, `textures.flb` (shaders);
  `manualkm.fds`, `manualcn.fds` (manual).

## Naming conventions

- Packs store lowercase names without extensions (`Resource.rpk` keeps
  extensions). Maximum 16 characters.
- Texture suffixes: `d` diffuse, `n` normal, `s` specular, `g` gloss,
  `h` height, `m1`/`m2` masks, `o` opacity, `r` reflection.
- Apparel: `<item>_<variant><part><map>`, e.g. `armet___aad`,
  `armet___aam1`, `armet___aan`.
- Weapon parts: `<type> <part> <nn>` (`sword grd 08`, `mace spike 08`,
  `haft1h ovl a04`); factories: `<weapon> <variant>` (`falchiona1`,
  `greatsword a1`, `ubm poleaxe a1`).
- Level pieces: region prefixes `ub`, `ub5`, `ub6`, `ubb`, `ubm`, `uw`,
  `uw01`, `usw` (Underworld areas), `got` (gothic), `mrb` (marble).
- Voices: `vh[f|m]NN_<kind>NN` (female/male, actor, grunts/groans).

## Format statistics

Formats by magic across all packs:

| format | count |
|---|---:|
| chunk tree (`CF AF 23 3D`) | 7 650 |
| image (`C6 3D 2D 1D`) | 8 839 |
| PCM WAVE | 1 131 |
| weapon factory | 106 |
| terrain / terrain table | 17 / 15 |
| nested pack | 9 |
| database | 5 |
| power | 4 |
| skeleton/motions | 1 |
| DDS | 2 |

Images by pixel format: BC4 ≈ 4 600, BC5 ≈ 2 300, BC1 ≈ 1 580, BC3 ≈ 115,
uncompressed ≈ 240; about 200 of those use RLE-packed storage.
