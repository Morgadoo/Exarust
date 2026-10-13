# Other formats

## Databases (`.rdb`)

Status: **framing** (`exa_formats::rdb`).

```
u32 magic       races 0xDBCB0D00, charroles 0xDBCB0D03, locales 0xDBAE0D01, objdb 0xDBAC0D09
u32 table_len   multiple of 16
records[table_len/16]: u32 id, u32 flags, u32 offset (from end of table), u32 size
payloads
```

`races.rdb` (19 records of 624 bytes), `charroles.rdb` (dialogue and AI
roles, including a C-like script language), `locales.rdb` (language list),
`objdb.rdb` (object database, 0x5900/16 = 1424 records), `objstrings.rdb`
(string library, magic `0xAFBF0C02`). The community spec reports a trailing
garbage record in each table. Record payloads are not decoded yet.

## Saves (`.rsg`, `.rcp`)

Not shipped; created in `%APPDATA%\Exanima`. Magic `0xDA7AEA02`, then a
section table padded to `0x2000` with `#playerdata`, `#worldchrdb`,
`#worldobjdb`, and a full modified copy of every visited level's `.rfc`.
Character records (`0x00CDAC06`, the same as actors) hold body, race,
equipment, health, personality, skills, roles and relations. Exarust uses
its own save format; this is documented for reference only.

## Terrain (`.rft` and `.ftb`)

`.rft` (`01 AD EF 3E`): `u32 0`, `name[16]` of its terrain table
(`UWA1`, `UWB6`, `GrassLand…`, `Seaside`, `vulcanic`), then grid sizes (e.g.
`f32 20.0`, `4`, `5`, `20`) and large data blocks (0.35–40 MB): heightfield,
material layer weights, cached GI/AO blocks (`TTerrainBlock`, `TGIMapBlock`,
`TStaticTerrain`).

`.ftb` (`02/03 BD EF 3E`): `u32 0`, `name[16]`, then a ~5 KB table of terrain
material layers (`TTerrainMat`, `TATrnMatLyr`), grass/plant meshes
(`trn_grass*`, `trn_lplnt*`, `trn_splnt*` in Resource).

## Powers (`.pwr`)

`CE 01 CE AF`, `00 CD 01 FE`, then a node graph (`TPwrNode`, `TPwrCnct`)
referencing icon images (`thmmndico1`). Four domains: displacement, energy,
force, mind — the thaumaturgy skill trees.

## Weapon factories

`Factories.rpk`, magic `0x00/01/02 0F CE AF` (three versions). Recipes that
assemble `Components.rpk` parts (heads, hafts, guards, pommels, langets) with
procedural materials (`TItemFactory`, `TFactoryMaterial*`, `TSteelGen`,
`TIronGen`).

## Actors

`actors.rpk` inside Resource: 21 entries, magic `06/07 AC CD 00`, then a size
and a `name[16]` (`ArenaShpKp`, `opponent01..`, `Demon A01`): spawnable
character templates with loadouts.

## Dialogue (`.rcd`)

`narrator.rcd`: speakers (`Narrator`, `Player`, `Corlian`, …) with colours,
then utterances. Format unknown beyond that.

## Material libraries (`.rml`)

Packs of material payloads (the same layout as `BA01` payloads, see
[RFC.md](RFC.md#material-list-ba00)) keyed by material name: `common.rml` (51), `underworld0.rml` (24), `underworldb.rml`,
`humanbase.rml` (`eyehuman`, `skinhuman`). Named in `Exanima.rfp`.

## Shader libraries (`.flb`)

`shading.flb` (54 programs: `ambient`, `bloomfilter`, `blurconv_x/y`,
`dispshell_f/v`, `effectbarrier_f/v`, …) and `textures.flb` (61 procedural
texture programs: `chardecomp_d/n/s`, `chargen_*`, `cloth_*`, …). Entries are
obfuscated: identical plaintext prefixes give identical ciphertext prefixes,
which suggests a position-keyed XOR or a static dictionary coder over GLSL.
Decoding them is optional (we write WGSL) but helps visual fidelity.

## Manual pages (`.fds`)

`manualkm.fds` (keyboard/mouse) and `manualcn.fds` (controller): packs of
CRLF text with markup tags `[algn=2]`, `[tcol=FFE399]…[/tcol]`. Pages:
armour, combat, encounters, health, interaction, inventory, movement,
saving, …

## Sound

`Sound.rpk`: 1131 RIFF/WAVE PCM 16-bit files. Mono 44.1 kHz (424),
48 kHz (395), 96 kHz (186), 192 kHz (39); stereo 48/44.1/96 kHz (87, mostly
music `msc_*` and ambience `ambn*`). Prefixes: `ambn*` ambience, `door*`,
`foley*`, `stp*` footsteps, `vh[fm]NN_*` voices (grunts, groans),
`thm*` thaumaturgy, `wh_*` weapon hits, `msc_*` music.
