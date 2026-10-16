# Other formats

## Databases (`.rdb`)

Status: **framing** (`exa_formats::rdb`).

```
u32 magic       races 0xDBCB0D00, charroles 0xDBCB0D03, locales 0xDBAE0D01, objdb 0xDBAC0D09
u32 table_len   multiple of 16
records[table_len/16]: u32 id, u32 flags, u32 offset (from end of table), u32 size
payloads
```

`races.rdb` (18 records of 624 bytes), `charroles.rdb` (dialogue and AI
roles, including a C-like script language), `locales.rdb` (the level list:
ids 1 Exanima, 2 Exanima01 … 13 Exanima67, 17 Arak_Ar),
`objdb.rdb` (object database, 0x5900/16 = 1424 records), `objstrings.rdb`
(string library, magic `0xAFBF0C02`). The community spec reports a trailing
garbage record in each table (the reader skips entries pointing outside the
file). `objdb` item records and race records are decoded; other payloads are
not.

**Races** (`exa_formats::races`): 18 records of 624 bytes, then an empty
entry. Layout:

| offset | field |
|---|---|
| `+0x00` | name[32] |
| `+0x20` | body model stem[24]: `Skel` → `skelbase.rfc`, `GlmBsA` → `glmbsabase.rfc` |
| `+0x38` | shared moveset[16] (`Default`) |
| `+0x48` | the race's own moveset[16] (`Golem`, `Herath`, `Ogre`, `Demon A01`, …) |
| `+0x58`, `+0x5C` | floats, not decoded |
| `+0x60` | support-pin factor: scales the pull of the two thorax pins and the pelvis pin toward the stance (human 1.25) |
| `+0x64`–`+0x70` | floats, not decoded |
| `+0x74` | balance bonus (1.0 in every record): the balance score gains 0.05 × it, and the recovery-step reach grows by (1 + it); the original sets 1.5 or 2.0 instead for some creature types |
| `+0x78` | stance factor: the share of muscle tone a standing body uses (tone target = tone × it) and the divisor of the standing joint-muscle damping, k = 40 / it (human 0.5; skeleton, fael and ghouls 0.75; golems and the harvester 1.0) |
| `+0x7C` | arm relaxation: scales how far the arms' muscle anchors may relax toward the current pose (human 1.0, skeleton 0.5) |
| `+0x80` | limb damping ratio r: a living body's velocity retention is scaled by u², u = (1 − balance)(1 − r) + r (1.0 for most races, so no extra damping; golems 0.95, the golem suit 0.97) |
| `+0x84`–`+0x90` | floats, not decoded |
| `+0x94` | gait table: 4 rows (walk, jog, sprint, backpedal) × 5 floats, see below |
| `+0xE4` | health factor (damage model, below) |
| from `+0xE8` | sound names at `+0x144` (`StpSft`, `SkelHit`, …), protection at `+0x11C` and coverage at `+0x21C` (natural armour, below); the rest not decoded |

Gait rows, per second as stored. The original's loader divides columns 0
and 3 by 83 and column 1 by 83² to get per-tick values.

| column | meaning |
|---|---|
| 0 | thrust: the push along the heading (human walk 88.9, jog 350, sprint 472.2, backpedal 77.8) |
| 1 | push scale: how the push enters the body's carry (550 in every human row; 1600 in some creatures' walk) |
| 2 | 0 in every record and never read: the loader fills this slot with a built-in value per row (0.05, 0.2, 0.25, 0.05) whose use is not traced |
| 3 | clip rate (human 27, 16.6, 13.5, 28) [inf: feeds the gait's clip clock; not traced] |
| 4 | speed along the heading under which the gait's phase push is boosted (0 everywhere but the harvester's walk, 20) |

How the factors were established: reverse engineering of `Exanima.exe`
0.9.5. The character builder looks the race up by its id (race byte + 1) in
the `races.rdb` id database, which is loaded raw, so every read of a race
field uses the file's own record and offsets. Each factor's meaning comes
from the routine that reads it (the support pins, the balance score and
recovery steps, the stance factor of the body tick, the arms' anchor
relaxation, the limbs' retention). When the id is missing, the original
uses a built-in record instead: every factor 1.0 and an all-zero gait
table, so a character on it cannot walk (`RaceBody::FALLBACK`). The values
per race are printed by `exa-tool races`; a corpus test checks every
record.

A character record's race byte is the race's id minus one. Placed
characters across the levels, by race byte:

- 0 Human 103, 1 Skeleton 147, 2 Ancient 23, 4 Sir (Herath ogre) 1,
  5 Fael 6, 6 Ghoul 56, 7 Ghoul H 41, 8 Gobbler 10, 9 Manipulator 19,
  10 Harvester 3.
- 16 Golem Guard 46, 17 Golem Boss 1, 18 Golem Suit 1, 19 Golem BAM 10,
  20 Golem BAF 10.

Bodies of the human frame (human, skeleton, ancient) name their parts like
the human body (`01Pelvis`). The others name them like apparel
(`0100Pelvis`, pelvis at the origin). All use the bone indices of
`humanoid.rsq` and its clips. Root positions are scaled by the body's leg
length against `humanbase` (golem 1.42×, ogre 1.39×).

## Object database items and apparel painting

`objdb.rdb` holds 1424 table entries; the last points outside the file and is
skipped. An item record payload is a chain of `u32 tag, u32 size` sections,
where the size counts the 8-byte header. A record's table flags are the OR
of the tags of the sections it holds (1422 of 1423 records; the exception
is "Map"), so `0x10101000` is name, ground model and body, and `0x10001000`
name and body only.

| tag | contents |
|---|---|
| `0x10000000` | display name[16], a word (0, 1 or 2, else noise; [guess] editor state), the parent id: the editor folder for global items (`+Apparel/+Misc`, `+Clothes/+Legs`, …), the source item for level-local copies |
| `0x00100000` | ground or icon model name[16] (`Objects.rpk` or `Objlib.rpk`, or `@` and a factory), then a flags word: bit 0 dims the model's voxel lights and glow meshes by the placement's condition byte (salves `0x51`/`0x1`, power crystals `0x1031`, Diviner's Deck, Repair Sphere, PedestalHand; see `RFC.md`, placed objects), …; absent in "Braies_A" |
| `0x1000` | item body: the worn model name[16] (`Characters.rpk`), the occupancy table at `0x10`, the slot table at `0xA8`, coverage and protection at `0x150`/`0x1A0`, the appearance block at `0x1B0..0x214` |

**Worn and ground model.** A character is dressed in the model the body
names, never the `0x00100000` one: REA on the apparel model loader shows it
loading and caching the body's leading name from `Characters.rpk`, and the
icon builder gives no icon when the `0x00100000` section is missing. Across
`objdb.rdb` and every level's own database the two names agree for 1617
apparel records (363 global, 1254 local). They differ for item 8
"Braies_A", which has no ground model, and for "Legg Cloth Lay A", "Trouser
Qui A" and their 39 local copies, which are worn as `Trouser_A` and shown
on the ground as `Trouser_At` (an `Objects.rpk` entry). 366 of the 388
objdb records with a body are worn as a `Characters.rpk` model (319
models), against 363 by the ground model. Exarust reads both
(`Item.worn_model`, `Item.model`): apparel is dressed, painted and looked
up by its worn model, weapons by their ground model.

Appearance block (100 bytes): quality, damage and dirt bytes (the painter uses
q = x, damage = x/(2−x), dirt = (x²+x)/2 with x = byte/255), one unknown
byte, then eight 12-byte layers: +1 mask number (`<stem>m<n>`, 0 = unused),
+2 material set (used when the material is 0), +4 material, +8 palette colour.

**Occupancy table** at body `0x10`: a `u64` for each of bones 1..20 (bone b
at `0x10 + 8(b − 1)`). Byte L is a bit mask of the bone's sub-parts the
item occupies on layer L, 0 innermost. Established by REA: the apparel
loader takes an item's layer as the highest non-zero byte over the table
and keeps worn items sorted by it; putting an item on takes off every worn
item whose table overlaps the new one's on any bone; the underwear rule
tests bit `0x02` of the pelvis's layer-0 byte, the groin. Only that bit is
tied to behaviour; the other sub-part bits are not decoded. Trousers, hose
and braies set it (pelvis `0x03` on layer 0); mail legs sit on layer 1,
shirts and gambesons on 2, tunics and coats on 4, plate on 6–7. The table's
last entry (bone 20) is the first word of the slot table at `0xA8`.
Exarust reads it (`Item.occupancy`, `Item::covers_groin`) for the
underwear rule; wear conflicts wait for equipping.

Item body offset `0xA8` holds apparel's per-slot layer table: 8 bytes for
each of 19 body slots (bone indices). Low bits give the layer level. Bit
`0x80` hides the body part of that slot (helm: head; trousers: thighs and
shins; boots: feet; gambeson: torso and arms). Bit `0x40` hides the slot's
partial body variant. Exarust hides covered body parts' meshes when
dressing (from the first byte of each slot). Fit variants and culling of
apparel triangles by their cull bits are not done yet.

### Underwear

The human body is nude: `humanbase.rfc` has no underwear part and the skin
textures are painted bare. Underwear is an item, objdb record 8
"Braies_A", and the engine refers to it only by that id.

- **The item.** Worn as `braies_a` (one node on the pelvis, texture set
  `braies__aa`, which has masks 1 and 2 only). No ground model, so it has
  no icon and cannot be dragged off the figure ([inf] from the icon code;
  not tried in the game). Occupancy: pelvis `0b11` and both thighs bit 0,
  on layer 0. Its slot table is all zero: it hides no body part.
  Coverage: pelvis 60 %, thighs 100 %, with zero padding and resistances
  and the cloth hit sound. Appearance: quality 160, damage 64, dirt 0;
  masks 1 and 2 rough cloth (material 1) in colour 1, palette family 0
  shade 1, `#c7c7c7`: pale grey linen. "Braies_P" (425) is the same model
  shown on the ground as `braies_a` and painted in mauve silk (colour 250);
  no level, local item, container, drop table or actor uses it.
- **The rule.** If a character record's race byte is 0 (human; [inf] that
  the tested byte is the race byte) and none of its worn items covers the
  groin, item 8 is appended to its worn list. It runs when a record is
  created, when the character screen opens, after an item is taken off the
  figure, looted or stripped, and in the arena's outfit generators. It does
  not run at spawn: a placed character is dressed exactly as stored.
- **Displaced braies vanish.** Putting on anything that overlaps them
  (trousers, hose) takes them off, and every handler of displaced items
  drops item 8 instead of storing it, so braies are never carried. Taking
  the trousers off again brings them back through the rule.
- **Start outfits.** A fresh record wears `[8]`, also in the play-start
  fallback with no player file. Story background 0 ("unknown") is cut to
  braies only, plus a start letter in the pack; its creator kit offers a
  shirt, trousers and shoes. The proctor and villager backgrounds use their
  `actors.rpk` templates, which wear trousers; 0.9.5 has no template for the
  knight. No kit, template or level lists braies for the player.
- **Corpus.** Of the 103 placed humans, 4 wear item 8 and nothing else
  (Sticky and two dead men in exanima01, Fergus in exanima02) and 99 wear
  trousers: all 103 are covered by their stored items.
- **Evidence.** REA on `Exanima.exe` 0.9.5 (the rule and its callers, the
  record defaults, the creator's background presets, kit tables and mode
  filter, the equip conflict pass and the displaced-item handlers, the
  apparel loader), the corpus above, and a retail save whose fresh story
  player (background 0) wears item 8 only, as do its Sticky and two
  unnamed humans.

Exarust (`exa_character`) applies the rule to the humans it makes at run
time: the player in play mode and the character view's human body. The
underwear goes on beneath the given outfit, painted with item 8's own
appearance, when nothing in the outfit covers the groin. Placed characters
are dressed as stored. Not done until equipping exists: the rule on
unequip, braies vanishing when displaced, wear conflicts between garments
given on the command line.

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

**Colour space.** REA on the original's paint jobs and texture loading shows
the following.
- The finishes read the shade map, the masks, the item normal and every
  library texture as stored. All are loaded as data and none is decoded from
  sRGB.
- The palette colour reaches the programs as computed (HSV to RGB, not
  linearised).
- The colour target is sRGB RGB8 (sRGB RGBA8 for items with alpha). The job
  switches sRGB encoding off before drawing, so each texel stores the
  program's output unchanged.
- The scene samples the colour target through sRGB decoding and multiplies it
  by the painted material's diffuse colour, which is 1.
- The specular target is linear RGB8 and the normal target linear RG8.
- The GPU builds the mips of all three targets after drawing (the original
  skips them when the GL driver reports a Radeon).

A finish's output is therefore an sRGB value, not a linear albedo. Pale grey
rough linen (palette colour 1, the braies' quality and damage) over a mid-grey
shade map and mid-grey library textures paints texel 96 and is lit as 0.117.
Exarust keeps this chain. It uploads the colour map as sRGB with the texels
as painted and averages its mip levels in linear light ([inf] that the
original's driver builds sRGB mips in linear light, as it filters them). The
surface and normal maps are uploaded as linear data.

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

## Weapon factories (`.fty`)

Status: **verified**. All 106 parse (v0 46, v1 34, v2 26). The generator
reproduces the original's choices; the reference generator matched all 143
factory weapons of a real save, part for part and material for material.
`exa_formats::factory`, `exa-tool corpus factories`.

- **Header**: `u32 magic` `0xAFCE0F0v`, 8 reserved bytes, name[16] (empty),
  atlas width and height, `i32` grips (0) × 8 bytes, `i32` material areas
  (at most 8 read) × 56 bytes: name[16], 8 candidates
  {material, bits, fixed colour, mode}, count at +0x30.
- **Parts**: `i32 n`, then per part `u32 flags` (bit 0 texture-only overlay,
  `0x10` random twist), name[16], position, rotation (radians), scale
  (cm), atlas rectangle, a texture parameter, 3 deforms (16 bytes in v0,
  28 in v1/v2: header type|axis<<8|flags, amount, pivot, range, blend, stat
  weight, 0), {chance, attach-to (1-based), role, attach mode}, then
  `i32` variants of 40 bytes (44 in v2): model[16], mask areas[4],
  slash/crush/pierce/thrust bonuses, weight (0 → 100 before v2), tags,
  exclude mask, flags, and in v2 a child Y offset (mm). A trailer lists the
  tag names.
- **Generation** (32-bit LCG `s·0x08088405 + 1`; seeds A and B; quality
  q = byte/255): a twist per part from B; per part and deform
  value = (2B−1)·blend + (1−blend)·(2A−1); per part a presence roll and a
  weighted pick among variants whose tags agree with earlier choices. The
  first part offering a tag owns it. Then per area a weighted material
  roll (weights 100/50/20/10 by class, scaled by distance from q to the
  candidate's level) with a finish and a colour by material class. Items
  with flag `0x10000000` in their weapon section keep their stored choices
  and materials.
- **Assembly**: components (`Components.rpk`, one mesh each, +Y along the
  weapon, cm) are placed in recipe order on their nearest chosen ancestor.
  The ancestor's scale and band deforms move the attach point, and twists
  accumulate. Textures are `<model>d`, `n`, `m1`..`m4`, painted per mask
  with the material of the area the variant maps it to.
- **Items** name a factory by a model name starting with `@`. Their weapon
  section (`0x4000`, 96 bytes) holds flags, seeds, quality, damage, dirt,
  4 material slots, stats, and the stored choices at +0x50.

Exarust assembles factory weapons for held items, placed objects and
`--weapon @name`. Not yet: taper, bend and radial deforms, atlas UV
cropping.

## Weapon stats

Status: **verified** (REA on the builder, the factory stat pass and the
condition step). A reference implementation reproduced bytes `0x30..0x3F`
of all 143 factory weapons in a real save, byte for byte. Exarust:
`exa_formats::weapon_stats`.

**Weapon section bytes** (`0x4000` body):

| off | meaning |
|---|---|
| `0x30` | weight, 0..100 |
| `0x31` | balance, 0..100 |
| `0x32` | impact adjustment (127 neutral) |
| `0x33` | thrust |
| `0x34` | striking side's type code |
| `0x35`, `0x36` | start and end of the striking part, ×0.005 of the length |
| `0x37`, `0x38`, `0x39` | slash, crush, pierce |
| `0x3A..0x3F` | the same block for the flipped side (e.g. a falchion's back) |
| `0x40` | adjustment, 0x80 neutral |
| `0x41` | hit sound class |

The section's first word holds the flags:
- kind (bits 0–3): 1 bladed, 2 hafted head, 3 stick, 4 flail;
- hands (bits 4–5): 0 unarmed, 1 one-handed, 2 two-handed, 3 polearm;
- haft material (bits 16–17): 1 wood, 2 metal, 3 other.

**Factory weapons** start from their template item's bytes.
- Each chosen variant adds its slash, crush, pierce and thrust (variant
  +0x1A, i16 each, low byte, wrapping). It also sets the type code from
  its flags (>> 4 & 0xF). Role-6 parts fill the flipped block instead.
- Each deform with value x = v·stat weight (deform +0x14) scales bytes by
  role. F(x) = 1 + x·amount, or 1/(1 − x·amount) when that is ≤ 0. G(x)
  is the same without the amount.

| role | deform | effect |
|---|---|---|
| blade | type 1, Y | weight ×F, balance ÷F |
| blade | type 1, X | weight ×F, balance ÷F, slash ×F, thrust ÷F |
| blade | type 3, Y | balance and 1/slash ×(F/2 + ½), thrust ×F |
| blade | type 4 (bend) | thrust by G (depends on flags) |
| haft | type 1 or 2, Y | f = 0.4G + 0.6; weight ×(f/2 + ½), balance ÷f |
| haft | type 1, XZ | weight ×(0.1G + 0.9) |
| guard | type 1, Y | thrust ×F |
| head | type 1 | the same as a haft, with f = F (halfway when per axis) |
| head | type 3, Y | same, with f = 0.5G(−x/2) + 0.5 |
| head | type 5, all or XZ | pierce ×(0.6G(x/2) + 0.4), crush ÷ the same |
| head | type 5, Y | same as a haft, with f = 0.4G(x/2) + 0.6 |

- Every step rounds. Weight and balance are capped at 100. Variant flag
  `0x800` removes the flipped side.
- Factory items then scale by condition c = (1 − 0.4d)(0.8 + 0.5q), with q
  and d the quality and damage bytes /255:
  - slash, pierce and thrust ×c;
  - crush ×(0.7c + 0.3);
  - each capped at 160.
- The head's striking extent (`0x35`/`0x36`) comes from assembled
  geometry; Exarust keeps the template's values for now.
- Materials and dirt do not enter the stats. There is no physical
  material table.

**Record** (`exa_combat::damage::WeaponRecord`):
- weight w = b30/100, balance b = b31/100;
- impact = (1 − b)·0.6 + 0.4w + 0.1 + (b32 − 127)/100;
- slash, crush, pierce and thrust = 0.5·(byte/100)^0.7;
- the length is the farthest component point along +Y, in cm.

## Armour

Apparel item body (`0x1000`) fields:
- `0x150 + 4·(bone − 1)`: coverage percent (u32) per bone;
- `0x1A0`: padding ×0.005;
- `0x1A1..0x1A3`: slash, crush and pierce ×1/140;
- `0x1A5`: encumbrance ×0.0025;
- `0x1A6`: hit sound class (0 flesh, 1 cloth, 2 leather, 3 mail, 4 plate,
  6 golem).

Instances scale padding and resistances by their condition c (the
appearance block's quality and damage). Races add a natural inner layer:
`races.rdb` coverage at `0x21C`, protection at `0x11C`. Skeletons, golems,
demons and the Fael head have one.

Each body part keeps two layers, filled in equipment order. A new item
folds the old outer layer into the inner one:
- each layer's effective coverage is γ(c) = c, or c + 1.75(1 − c)(c − 0.5)
  above ½;
- padding adds as γ-weighted;
- resistances combine as s0′ + (1 − s0′)·γ1·s1, with s0′ = γ0·s0;
- the inner layer's coverage becomes 1.

The 19×8 slot table at `0xA8` is not read for armour.

## Damage

Implemented in `exa_combat::damage`.

**Inputs**
- Speed v of the struck point (cm/tick): capped at 30; below 10 it is
  softened to 10(2 − v/10)(v/10)²; overhead swings ×1.5. u = 0.05v.
- Swing impact I = (impact + 0.3)·1.1·u^(1 − impact/2), times the zone
  factor along the weapon:
  - past its tip there is no hit;
  - below the striking part (the haft) the edge share fades;
  - bladed weapons: impact peaks near the middle and falls towards the
    tip, where the draw-cut share grows.
- Stabs: energy T from the tip (full within 7 cm), weight and the lunge.

**Armour**
- One roll r per blow. A layer counts fully when r < coverage, partly
  (1.75(c − ½)) above half coverage, otherwise not.
- Padding A sums. Resistances combine multiplicatively; pierce resistance
  is weighted by (½ + r/2).

**Through the armour**
- Impact: J = max(I − 0.2A, 0)/(A² + A + 1).
- Crush: C·1.58·J^(2 − C), then reduced by crush resistance.
- Slash: a mix of chop and draw by the draw share, each reduced by slash
  resistance.
- Pierce: P·1.58·J^(1 + P/2), less 0.75 of pierce resistance.
- Penetration P = 0.5·max(J − 0.8, 0) + slash + crush + pierce.

**Outcome**
- Part multipliers (damage/stun): head 1.6/1.4, thorax 1.2/1.2, lower legs
  0.6, feet 0.4, forearms 0.5/0.75, hands 0.3/0.6, others 1.
- Red = 0.75·P/(P + 1)·m. Yellow = 0.75·(T/2 + (2(P − P/(P + 1)) + J)·m).
- Health starts at 0.25 × the race's factor (`races.rdb` `0xE4`: human 1,
  ogre 6, golem 2, …).
- Applying a blow: yellow −= 0.1·Y; red −= 0.2·R; yellow is rescaled by
  how much red is left. Yellow under 0.01 is death.
- Daze = √(0.75P + 1.25J)·stun·0.5·0.25/max health.
- Blows with J > 0.5 shove the victim by 10J cm/tick.

Not yet: the bite (weapons sticking in), blocking and parrying, shields,
encumbrance's effect on swings, and the state that knocks a character
unconscious.

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
