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
roles, including a C-like script language), `locales.rdb` (the level list:
ids 1 Exanima, 2 Exanima01 … 13 Exanima67, 17 Arak_Ar),
`objdb.rdb` (object database, 0x5900/16 = 1424 records), `objstrings.rdb`
(string library, magic `0xAFBF0C02`). The community spec reports a trailing
garbage record in each table (the reader skips entries pointing outside the
file). `objdb` item records are decoded (see below); other payloads are not.

## Object database items and apparel painting

`objdb.rdb` holds 1424 table entries; the last points outside the file and is
skipped. An item record payload is a chain of `u32 tag, u32 size` sections,
where the size counts the 8-byte header:

| tag | contents |
|---|---|
| `0x10000000` | display name[16], two words |
| `0x00100000` | model entry name[16] (in `Characters.rpk` for apparel), … |
| `0x1000` | item body; bytes `0x1B0..0x214` are the appearance block |

Appearance block (100 bytes): quality, damage and dirt bytes (the painter uses
q = x, damage = x/(2−x), dirt = (x²+x)/2 with x = byte/255), one unknown
byte, then eight 12-byte layers: +1 mask number (`<stem>m<n>`, 0 = unused),
+2 material set (used when the material is 0), +4 material, +8 palette colour.
276 of the 387 item records name a model in `Characters.rpk` directly.

Item body offset `0xA8` holds apparel's per-slot layer table: 8 bytes for
each of 19 body slots (bone indices). Low bits give the layer level. Bit
`0x80` hides the body part of that slot (helm: head; trousers: thighs and
shins; boots: feet; gambeson: torso and arms). Bit `0x40` hides the slot's
partial body variant. Exarust hides covered body parts' meshes when
dressing (from the first byte of each slot). Fit variants and culling of
apparel triangles by their cull bits are not done yet.

The game never draws an apparel item's own textures. It paints a colour
texture per item from the shade map `<stem>d` (cavities dark), stacking each
layer's finish over its mask on black. Run-time materials `&<10 chars><digit>`
name the stem: the ten characters after the `&` (`&doublet_aa1` →
`doublet_aa`, `&gambsonaba1` → `gambsonaba`). The finishes by material
number:

| material | finish | library textures |
|---|---|---|
| 1 Cloth Rough, 2 Cloth Fine, 3 Silk, 5–6 Padded | cloth | weave (`ClothRghA03`, `ClothFinA01`, `ClthP01/02`), `LeatherDmgA02`, `ClothDmgA01` (worn), `DrtApr01` |
| 4 Velvet | velvet | `ClothFinA01` with a pale sheen |
| 7–8 Leather | leather | `LeatherA02/A01`, `LeatherDmgA02`, `DrtApr01` |
| 9 Suede, 10 Rawhide | suede | `SuedeA01` |
| 11 Mail | mail | `MailStlA01` d/o, rust, scuffs |
| 12–13, 15 Steel, Plate, Gilded | plate | `MtlStlBas01`, `MtlImpure01`, rust `MtlRustA03/A04`, `ScuffsGnrA01` |
| 14 Painted | painted steel | as plate plus `MtlDirtA01` |

Material 0 takes the first material of its set (sets 1/3/5/6/8 → 1, 2/9 → 5,
4/10 → 7, 7/12/13 → 13, 11 → 11, 14 → 20, 15 → 23). Library textures are laid
one texel per painted texel (weave, worn weave, impurities); damage, dirt,
rust and scuff maps are laid at 1024 texels across.

Palette: index = family × 8 + shade. Each of the 32 families has hue,
saturation, base value and value step. With k = 7 − shade:
v = min(k·step·0.13 + base, 1) and s = (1 − (0.09k)²)·sat, scaled by (v + 0.6)²
when v < 0.4. Then HSV → RGB. Family 0 is the greys (0.91 down to black).

Exarust paints on the CPU (`exa_assets::paint`; `exa-tool paint <model>
<stem> out.png`). The original's surface pass writes specular occlusion, gloss
and metalness, with roughness = 1 − gloss. Exarust paints gloss and metalness
into Bevy's metallic-roughness texture. Plate, painted steel and mail follow
the original; fabrics and hides are approximated without their gloss textures.
The normal pass adds the material's weave or grain normal to the item's own
normal, scaled by wear, plus the damage normal where cloth frays or hide
scuffs. Exarust paints it for fabrics and hides; metals keep the item normal.
Not yet painted: wood, bronze, gemstone and resin finishes
(drawn plain), pattern opacity textures, and per-instance appearances
(randomised items, wear on characters).

## Saves (`.rsg`, `.rcp`)

Not shipped; created in `%APPDATA%\Exanima`. Magic `0xDA7AEA02`, then a
section table padded to `0x2000` with `#playerdata`, `#worldchrdb`,
`#worldobjdb`, and a full modified copy of every visited level's `.rfc`.
Character records (`0x00CDAC06`, the same as actors) hold body, race,
equipment, health, personality, skills, roles and relations. Exarust uses
its own save format; this is documented for reference only.

## Terrain (`.rft` and `.ftb`)

Moved to [TERRAIN.md](TERRAIN.md) (verified).

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
