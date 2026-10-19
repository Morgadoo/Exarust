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
entry. Lengths below are as stored; the parser returns them in metres.
Layout:

| offset | field |
|---|---|
| `+0x00` | name[32] |
| `+0x18` | first flag word (u32, inside the name field after the short name): bit 0 bleeds; `0x8` in the story, a dead character of the race may be raised again (humans, the golem suit) [inf: from the character rule bit it sets]; `0x20` sets a character rule bit read by the damage routine [?]; `0x80` red regeneration (the golem suit); `0x200` hovering feet, so the simple ground probe (the wraith). Human `0x0F`, Ogre `0x27`, Sir, harvester and demon `0x23`, Fael, both ghouls, Gobbler and Manipulator `0x03` bleed; Skeleton and Ancient 0, Wraith `0x300`, golems `0x1030`, `0x2030`, the suit `0x20B8` do not. Bits `0x2`, `0x4`, `0x10`, `0x100`, `0x1000`, `0x2000` not decoded (the golem bits `0x1000` and `0x2000` are tested together with two particular items) |
| `+0x1C` | second flag word: `0x4` red regeneration (Ogre, Sir, the harvester, the demon: `0x7`, `0x7`, `0xF`, `0x7`), `0x8` quadruped, which stabs with both hands (the harvester); `0x1` and `0x2` set character rule bits whose effects are not traced. Fael, ghouls, Gobbler and Manipulator `0x3`; the others 0 |
| `+0x20` | body model stem[24]: `Skel` → `skelbase.rfc`, `GlmBsA` → `glmbsabase.rfc` |
| `+0x38` | shared moveset[16] (`Default`) |
| `+0x48` | the race's own moveset[16] (`Golem`, `Herath`, `Ogre`, `Demon A01`, …) |
| `+0x58` | kind word: `0x100` the fists are claws (weapon class `0xC0` instead of `0x80`: Ghoul and Ghoul H `0x100`; Sir, Fael, Gobbler and the harvester `0x20100`); the left hand's fist record is chosen by the whole word (`0x100` one claw record, `0x20100` the other, anything else a human fist), the right hand's by the race, so the harvester keeps a right fist. An item check matches `0x10000` (golems, `0x10001`) with items flagged `0x2000000` and `0x20000` (Ogre and the demon `0x20001`, the clawed Herath races) with items flagged `0x4000000`. Others `0x1` |
| `+0x5C` | clip-rate base: the character's clip clock advances by it times the per-motion factors each tick (1.0; the two ghouls 1.5) |
| `+0x60` | support-pin factor: scales the pull of the two thorax pins and the pelvis pin toward the stance (human 1.25) |
| `+0x64` | character size, × (1 − 0.05·height) at spawn (human 1.0, Ogre, Sir, Fael, the demon and Golem Boss 1.5, the harvester 1.8, the other golems but the suit 1.4): held weapons are placed and scaled by it |
| `+0x68` | body scale, capped at 1.5 then × (1 − 0.05·height) at spawn (human 1.0, Fael 1.35, Ogre and Sir 1.5, the golems but the suit 1.2, the harvester and the demon 1.8, so 1.5): the voice volume, turn and step rates follow it |
| `+0x6C` | 1.0 in every record, no reader found |
| `+0x70` | mass divisor: the spawn's part-mass normalisation multiplies its divisor by it, so the parts are lighter (1.0; Sir, Fael and the demon 1.1) |
| `+0x74` | balance bonus (1.0 in every record): the balance score gains 0.05 × it, and the recovery-step reach grows by (1 + it); the original sets 1.5 or 2.0 instead for some creature types |
| `+0x78` | stance factor: the share of muscle tone a standing body uses (tone target = tone × it) and the divisor of the standing joint-muscle damping, k = 40 / it (human 0.5; skeleton, fael and ghouls 0.75; golems and the harvester 1.0) |
| `+0x7C` | arm relaxation: scales how far every part's link-muscle anchors may relax toward the current pose while the clip holds still, and the arms' relaxed stiffness, 0.4·(0.5 + 0.5·it) (human 1.0, skeleton 0.5) |
| `+0x80` | limb damping ratio r: a living body's velocity retention is scaled by u², u = (1 − balance)(1 − r) + r (1.0 for most races, so no extra damping; golems 0.95, the golem suit 0.97) |
| `+0x84` | unarmed reach, cm: the character's reach while it holds no weapon (human, skeleton, ancient, Manipulator, Wraith, the golem suit 50; Ogre, the ghouls, the demon and the golems 100; Fael 130; Sir, Gobbler and the harvester 150) |
| `+0x88` | swing-turn factor: scales each held weapon's swing-turn amplitude, how far the AI turns its body through a swing (1; Ogre, Gobbler and the golem suit 0.5; Sir, the harvester, the demon and the other golems 0) |
| `+0x8C` | AI attack bonus: the AI's combat state scales an attack-opportunity term by (1 + it) (Fael 0.2, the others 0) [inf: one reader] |
| `+0x90` | recovery factor: scales each held weapon's recovery, the frames the stance blends in over after an attack, rounded (1; golems 0.5) |
| `+0x94` | gait table: 4 rows (walk, jog, sprint, backpedal) × 5 floats, see below |
| `+0xE4` | health factor (damage model, below) |
| `+0xE8`–`+0x108` | bytes and floats read by view and camera routines, not decoded (Sir `+0xF8`/`+0xFC` −8 and 14, `+0x100`–`+0x108` 1.4) |
| `+0x110` | perception: how well the race's AI sees a character masked by a mind power; p scales its sighting range (1200 cm) by 1 − 0.9·(1 − p), and its outer range is that range scaled again by 1 − 0.8·(1 − p) (1; Ancient 0.5; Skeleton and Wraith 0). A feral character uses 1 − feral/2 |
| `+0x11C` | protection (natural armour, below) |
| `+0x12C` | held-item offset: three signed bytes × 0.1 cm added to the grip of items in the hands, x taken off in the right hand and added in the left (the demon 90, so 9 cm along x; 0 elsewhere) |
| `+0x134`, `+0x154`, `+0x164` | sound names (`Golem_Foley`; `FlsCol`, `SkelHit`, `HitGlm`; `WndFls`, `SkelHit`, `HitGlm`), readers not traced |
| `+0x144` | footstep sound set (up to 15 characters): `StpSft` (Human, Ancient, the ghouls, Gobbler, Manipulator, the harvester, Wraith), `SkelStp`, `StpHvy` (Ogre, Sir, the demon), `StpMed` (Fael), `Golem_Stp`. Characters whose record flags them as golems use `Golem_Stp` whatever their race |
| `+0x174`, `+0x184` | voice set names (`VH{S}`, `VH{S}DA`, `VOM01`, …; `{S}` becomes `m` or `f`), read by the race loader, not decoded |
| `+0x1B4` | voice volume: × the body scale at spawn (1; Ogre, Sir and the demon 3; Fael, Ghoul and the harvester 1.5; Ghoul H and Manipulator 2; golems 0.5) |
| `+0x1C4` | attack voices (i32): 0 never voices an attack; otherwise a swing voices with odds of one (or two) in a counter that grows by (this − 1) with each voice and shrinks by 1 with each silent swing [inf] (human, Ancient, Wraith 4; Gobbler, Manipulator 3; Fael, ghouls, the harvester, the demon 2; Ogre, Sir 1; skeletons and golems 0). A feral character uses 2 |
| `+0x21C` | coverage (natural armour, below); the rest of the record not decoded |

Gait rows, per second as stored. The original's loader divides columns 0
and 3 by 83 and column 1 by 83² to get per-tick values, and negates the
backpedal row's columns 0 and 3.

| column | meaning |
|---|---|
| 0 | thrust: the push along the heading (human walk 88.9, jog 350, sprint 472.2, backpedal 77.8) |
| 1 | push scale: how the push enters the body's carry (550 in every human row; 1600 in some creatures' walk) |
| 2 | never read (0 in every record but the backpedal row, which holds stray bytes `84 1E 00 00`): the loader fills this slot with a built-in value per row (0.05, 0.2, 0.25, 0.05), which the locomotion reads × 1000 as the brake distance in cm: a walking body brakes 50 cm from its goal, jogging 200, sprinting 250, backpedalling 50 (a running body never brakes for its goal). The parser returns these as `Gait::brake_distance` (0.5, 2, 2.5, 0.5 m), the fallback's rows included |
| 3 | clip rate, frames per second (human 27, 16.6, 13.5, 28): the base rate of the locomotion clock, blended from walk toward the gait by the run blend |
| 4 | speed along the heading under which the gait's phase push is boosted (0 everywhere but the harvester's walk, 20) |

A feral character (character record feral level f > 0) sprints with the
sprint row moved toward the jog row by f, every column alike, set at its
spawn (`RaceBody::sprint_gait`): a fully feral zombie sprints at the jog
row.

How the fields were established: reverse engineering of `Exanima.exe`
0.9.5. The character builder looks the race up by its id (race byte + 1) in
the `races.rdb` id database, which is loaded raw, so every read of a race
field uses the file's own record and offsets. Each field's meaning comes
from the routine that reads it: the character builder (scale capped at
1.5, size, the mass divisor, the height factor), the character init (the
clip-rate base as (83/83) × the field, the flag words to the character's
rule bits, perception, the AI bonus, voice volume and attack voices), the
weapon pass on every equip (recovery, swing turn, unarmed reach), the
moveset chooser and fist record choice (claws), the footstep set, the hand
placement, the AI's sighting ranges and attack scoring, the support pins,
the balance score and recovery steps, the stance factor of the body tick,
the arms' anchor relaxation and the limbs' retention. When the id is
missing, the original uses a built-in record instead, through the same
loader: every factor 1.0, an all-zero gait table (so a character on it
cannot walk) with the brake distances, 100 cm of unarmed reach, swing turn
and recovery 1, attack bonus 0, perception 1, no footsteps, voice volume 0
and attack voices 0 (`RaceBody::FALLBACK`, `RaceStats::FALLBACK`). The
character builder copies the first flag word to the character; the damage
routine's blood branch tests bit 0, and the race loader turns the blood
dirt mode on for the race's body-model materials whose names start with
`s` when it is set ([RENDERING.md](../RENDERING.md), Blood). The values
per race are printed by `exa-tool races`; a corpus test checks every
field of every record.

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
| `0x1000` | item body: the worn model name[16] (`Characters.rpk`), the occupancy table at `0x10`, the slot table at `0xA8`, coverage and protection at `0x150`/`0x1A0`, encumbrance at `0x1A5`, the appearance block at `0x1B0..0x214`, the special kind, power flags and articulated set at `0x214`/`0x218`/`0x220` ([Armour](#armour)) |
| `0x4000` | weapon section (208 items): flags, factory seeds and choices, stat bytes `0x30..0x42` (Weapon stats) |
| `0x8000` | shield section (22 items, 80 bytes): weight, block and bulk bytes at 4, 5, 6 (× 0.01 in double precision; Weapon record); the bulk adds to its holder's encumbrance ([Encumbrance](#encumbrance)) |
| `0x10000` | operative section (68 bytes; levers, buttons and plates, keys, signal jobs, relays, other usable objects): class, kind, flags, targets (Door and operative sections) |
| `0x40000` | door section (28 bytes): flags, linked object, sound letters (Door and operative sections) |

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
are dressed as stored. Equipping (`exa_items::carry`) takes off every
worn item whose occupancy overlaps the new one: braies are dropped, other
items return to the pack, and a human left bare at the groin wears braies
again. The inventory windows are not in.

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

## Door and operative sections

Status: **verified** for the fields below (REA on the original's door,
lever and push-surface set-up, its signal dispatch, relays and jobs, and the
object factory that picks the body class; counts from the installed
databases). Offsets are from the section body (after the 8-byte header).
Exarust reads both into `objdb::Item` (`door`, `operative`).

**Door section `0x40000`** (20-byte body):

| offset | field |
|---|---|
| 0 | u32 flags: `0x2` latches (bars itself shut when it closes), `0x4` locked, `0x8` not a door to the hand's click, `0x10` motorised (moves by itself when signalled), `0x40` fast motor, `0x80` signals inverted, `0x100` a sliding door held at its file position, `0x200` sets scene-node flag `0x80` (meaning not established) |
| 4 | u32 linked object (database tag bits as placed objects): the key that unlocks the door, compared with the held key; relays of kind 1 signal every door linked to them; some doors name the lever that moves them |
| 8..11 | sound letters: lock (`LkO`, `LkT`), slam (`slm`), hinge creak loop (`hng`), unlatch (`opn`); the sound is `door`, `a` + letter, `_` and the suffix (`doorf_slm` → `doorf_slm01…06` in `Sound.rpk`) |
| 12..19 | zero in every record |

Flags seen across the databases: 0 (plain), `0x2` (secret door), `0x12`
(city gate), `0x52`, `0x108`, `0x200` and `0x204` (prison gates,
portcullises), `0x214`, `0x216`, `0x254`, `0x256`, `0x2d4`, `0x5c` and `0xdc`
(pit doors: locked, not clickable, motorised and fast; `0xdc` also
inverted). In `exanima01` the linked objects are 6 keys, 4 levers and 7
kind-1 relays; one door names an id its database lacks.

**Operative section `0x10000`** (60-byte body):

| offset | field |
|---|---|
| 0 | u8 class: 1 key, 3 lever, 5 push surface (buttons, pressure plates), 6 job, 7 relay; 0, 2, 4 and 8 are other usable objects not decoded here |
| 1 | u8 kind: a lever's 6 fades a light under its node instead of signalling; a push surface's 2 inverts its signals; a job's `0xC` lights a fire, `0xD` turns a light on or off; a relay's 0 and 3 combine their inputs' bits into a code, 1 signals every object linked to it, 2 signals both targets, 5 counts a sequence |
| 2 | u16 flags (a lever's starting value and forced detent, a push surface's one-shot and second-target bits, a relay's inversion and quiet bits) |
| 4 | u16 value: a fire's size, a relay's state, the bit an input sets in a relay |
| 6 | u16 a lever's auto-return time in 100 ms (0: it stays) |
| 8 | u32 the object signalled |
| 12 | u32 the second target, or a relay's code |
| 16.. | a name (`StnSwtch01`, `DeviceCmbA01`) and further words in some records of the other classes, a script line in one; not decoded |

**Body class.** When the original loads an object's model, a record with an
operative section makes the model's root rigid body a lever (class 3) or a
push surface (class 5), and none of the others; a record with a door section
and no operative section makes it a door. No shipped record has both
(`Item::mechanism`).

Corpus: `objdb.rdb` 47 door and 49 operative sections (15 keys of kind 0;
47 doors, 6 levers and 5 push surfaces as bodies). `exanima01`: 43 doors and
72 operative records: 40 levers (39 of kind 1, one of kind 12), 12 push
surfaces (11 of kind 1, one inverted), 6 fire jobs (sizes 200, 176 and one 0), 8
relays (kinds 0, 1, 3, 5) and 6 keys. The other story levels have 2 to 8
doors each. `exa-tool mechanisms [level]` prints them.

## Saves (`.rsg`, `.rcp`)

Not shipped; created in `%APPDATA%\Exanima`. Magic `0xDA7AEA02`, then a
section table padded to `0x2000` with `#playerdata`, `#worldchrdb`,
`#worldobjdb`, and a full modified copy of every visited level's `.rfc`.
Character records (`0x00CDACvv`, the same as actors and the levels' own;
layout in [RFC.md](RFC.md#starts-characters-and-placed-objects)) hold body,
race, equipment, health, personality, skills, roles and relations. Exarust uses
its own save format; this is documented for reference only.

## Terrain (`.rft` and `.ftb`)

Moved to [TERRAIN.md](TERRAIN.md) (verified).

## Powers (`.pwr`)

Status: **the node layout, the text chunk, the link curve, and the node
pictures are verified**. Thick-line quads are not drawn. Exarust:
`exa_formats::power_tree`.

- **Header**: first word `0xAFCE01CE`. The node count is the signed word at
  byte 52. A negative count is an error. The bytes between the magic and
  the count are skipped.
- **Nodes**: power id, two further words, x and y as floats, then a signed
  link count. A count below 1 stores no links. Otherwise that many 16-byte
  links follow. After every node is read, each node but the first subtracts
  the first node's x and y.
- **Text chunk**: first word `0xABCD0000`, then five words, then a 16-byte
  head whose last word is the string count. One more offset than that count
  follows, then a byte length and that many bytes. Each string is a length
  word at its offset and that many bytes. A length below 1 is empty. No
  chunk leaves the texts empty. A chunk that ends early is an error.
- **On screen**: a node sits at ten times its place times the zoom, plus the
  pan, and rounds to the nearest even integer unless the view is dragged. A
  factor then scales that point about the origin. A link whose two marks are
  zero is not drawn. Otherwise each end leaves its node along the sine and
  cosine of its turn (the link's first two words, fractions of a circle)
  times a full circle. The reach is 0.514, or 0.295 when the power's high
  mark is set, minus 0.003 times the zoom, times the zoom rounded to an even
  256. The samples are the cubic between those ends. A zero source mark dims
  the line. The fast sine polynomial and the thick-line quads are not
  reproduced. A turn of zero is straight up.
- **Pictures**: an icon with no columns is not drawn. Otherwise the cell is
  the power's low byte, one less, across the sheet, and the square is
  centred on the node. A second byte of 1 uses a bias of 1; any other uses
  the other bias. A frame uses the slot one less on a four-by-two sheet, at
  four fifths of the size. A wide picture drops the fraction of the view; a
  narrow one uses half. A glow is 32 spokes on a ring of 0.84 of the width
  plus the outer size, starting straight up.
- Four domains: displacement, energy, force, mind. Learning a node and the
  thick-line quads are not in.

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

The low 16 bits also pick the holder's moveset (REA on the routine run
after every equip change; [PLAYER.md](../PLAYER.md), Moveset from held
items): by the hands bits, and among two-handers "Greatsword" for exactly
`0x521`, "2H Light" when bit 0 is set, else "Two Handed". The installed
object database's 208 weapon items give "One Handed" 136 times, "Polearm"
32, "Two Handed" 19, "2H Light" 18 and "Greatsword" 3; none lacks hands
bits. The torch, "Torch A01", is a one-handed stick (`0x01000013`). The
fists' built-in records have no hands bits (flags 0, or `0x40` for the
two clawed ones).

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

The record a held weapon fights with is built from these bytes: see
Weapon record below.

## Weapon record

Status: **verified** (REA on the record builder, the holding pass, the
length and sweet-spot walks, the mass spread and table, the equip
refresh, the unarmed set-up, the shield record and its attach, the step
reach; corpus below). Exarust: `exa_formats::weapon_record`
(`exa_combat::damage::WeaponRecord`).

Every body that can strike carries a record of 28 elements W0..W27: a held
weapon, a hand, or a shield (a short record of its own). Lengths are in
cm. Below, w and b are the weight and balance bytes /100, g the holder's
armour encumbrance, s its character size, u the share of the held
weapons' weight a special worn item takes away (0 for everyone else).

**Build**, from the section (the flipped block for `0x34..0x39` when the
weapon is flipped):

| element | value |
|---|---|
| W0 flags | the section's flags & `0x0603FFFF` |
| W1 type code | byte `0x34` |
| W3 | the start below, 0.7 when under 0.15, at most 0.99 (readers not found) |
| W4, W5 striking part | bytes `0x35`, `0x36` × 0.005, of the last part's length |
| W6, W7 | w, b |
| W10 impact | (1 − b)·0.6 + 0.4w + 0.1 (0.4w + 0.1 above balance 100) + (byte `0x32` − 127)/100 |
| W11..W14 | slash, crush, pierce, thrust: 0.5·(x/100)^0.7, 0 for a 0 byte (double precision) |
| W16 handling | 0.8 − ((1 − u/2)·w + 1 − b)/8; the body turns at it × B·(1 − 0.25·progress)·P while attacking ([PLAYER.md](../PLAYER.md), Combat mode) |
| W21 length | armed: the box top (centre.y + half extent.y) of the weapon's node plus that of every rigid node down its first-child chain; unarmed: across the hand, \|centre.x\| + half extent.x |
| W22 sweet spot | armed: whole parts while the next one down the chain is rigid, then (W4 + 0.6·(W5 − W4)) of the part it stopped at; unarmed: W21 |
| W23 (unarmed) | the end byte as a number (80 for the fists); unused for reach |
| W24 second hand | (byte `0x40` − 128)·0.00156 + 1: where the second hand holds a haft (a two-hander's −30·W24 cm, a polearm's −40·W24) |
| hit sound | byte `0x41`: `WH_SwordA`, `WH_MaceMA`, `WH_AxeA`, `WH_ClubMW`, `WH_MaceMA`, `WH_ClubMW`, `WH_WoodA`, `WH_ClawsL`, `BdyCol` |

A node is rigid when its mesh body is followed by a collision body chunk
(`0x3DD0B001`, [RFC.md](RFC.md)). An assembled factory weapon is one part
in the original, so its length is its assembled top. Corpus: the box of
every one of the 103 plain weapon models of `Objects.rpk` tops at its
highest vertex within 0.05 cm; "Torch A01" is 58.3 cm (its shadow and
flame nodes are not rigid), "Flail A" 84.4 cm and "Ogre Flail" 172.0 cm
(their links are rigid).

**Holding pass**, at the build and on every equip change:

| element | value |
|---|---|
| W8, W9 held weight and balance | w, b; with the half-sword hold (2 − w)·w and 0.7·b (what sets it is not found); W8 × (1 − u) |
| h | (1 − W9)·0.6 + 0.4·W8 + 0.1 (0.4·W8 + 0.1 above balance 1) |
| W17 recoil | g/2 + h: a blocked striker recoils by v·W17; the recovery after a released swing follows it, and the attack cooldown is round(300·W17) + 300 ms of the primary striker (400 ms with none) |
| k | ((1 − g) + W9)·g/2 + W9·(1 − g) |
| W18 swing shaping | 1 − k (the swing clock slows away from its middle by it) |
| W19 blend | round((W8 + 1 − k)·10), an integer: attacks end into the stance over 2·W19 frames |
| W20 arm stiffness | 0.7 − 0.3·W17; two-handers (hands bits 2) out of the half-sword hold × (0.7 − 0.2·W9); greatswords (`0x521`) × 1.5; unarmed 0.5/s |
| W15 swing speed | (1 − W8)·0.15 + 1.05; × 1.2 for the human fists (no hand-class bits) |
| W27 AI swing turn | min(0.4·W18 + 0.3·W8 + 0.4, 1.5); unarmed × 1.5 for claws, × 0.5 for fists |
| W25, W26 swing volume and pitch | up to 124 cm 1.5·√((250 − L)·0.004) and (250 − L)/180, else 1 and 0.7; W26 × (1 + d/2), d = W9 − W8 halved when negative; unarmed W25 × 0.7·√scale, W26 × 0.6/scale |
| W23 reach | armed: 60·s + W22 |

Worked example, "- Sword A" (90 cm): W15 1.125, W17 0.42, W18 0.2, W19 7,
W20 0.574, W22 57.6, W23 117.6, W25 1.2, W26 1.022, W27 0.63. With g 0.5:
W17 0.67, W18 0.275, W19 8.

**Mass** (simulation units): M = heavy·w + (1 − w)·light by hands and size

| size | one-handed | two-handed | polearm |
|---|---|---|---|
| normal | 0.16 – 0.40 | 0.20 – 0.50 | 0.20 – 0.50 |
| ogre (`0x2000000`) | 0.32 – 0.80 | 0.50 – 1.00 | 0.50 – 1.00 |
| demon (`0x4000000`) | 0.50 – 1.20 | 0.75 – 1.50 | 0.75 – 1.50 |

spread over the weapon's rigid parts half evenly and half by their own
masses; the weapon's own part then weighs s times more.

**Equip refresh**: each held record is held again with the character's g
and s; the character's reach is the largest W23; W19 = round(W19 × race
`0x90`) (0.5 for the five golems, else 1) and W27 × race `0x88` (0 for
Sir, the harvester, the demon and four golems; 0.5 for the ogre, the
gobbler and the golem suit; else 1); the AI's thrust preference is
round(W14/(W11 + W12 + W13)·5) + 1 capped at 9, 10 without edges, 0
holding nothing. Unarmed, the reach is race `0x84` (human, skeleton,
ancient, manipulator, wraith and golem suit 50 cm; ogre, ghouls, demon and
the other golems 100; Fael 130; Sir, gobbler, harvester 150) and the
hands weigh twice their part's mass (four times with heavy
claws). Race index 6 and 7 (the ghouls) swing with W20 = 1; race index 5
(the Fael) turns half as far (W27 × 0.5).

**Fists and claws.** Unarmed, both hands are held, each with a built-in
section: the right hand's by race index (8 heavy claws; 6, 7 claws; 4, 5
heavy claws), the left hand's by the race's word at `0x58` (`0x100` claws,
`0x20100` heavy claws), else the human fists. So the Harvester (index 10,
word `0x20100`) strikes with a right fist and a left heavy claw.

| section | flags | balance | impact | type | start = end | slash, pierce | `0x40` | sound |
|---|---|---|---|---|---|---|---|---|
| fists | 0 | 100 | 160 | 1 | 80 (0.4) | 0, 0 | 0 | 8 `BdyCol` |
| claws (ghouls) | `0x40` | 100 | 180 | 1 | 60 (0.3) | 60, 40 | 0 | 7 |
| heavy claws | `0x40` | 100 | 200 | 1 | 150 (0.75) | 60, 40 | 0 | 7 |

A human fist is 17.66 cm (the hand node's box), with impact 0.43, W15
1.44, W17 0.1, W18 0, W19 0, W20 0.5, W24 0.80032.

The hands themselves strike with these records: a fist's contact is
measured along the hand's x axis toward its box (negated for the right
hand, whose box lies on −x), and one at or past W21 is ignored. With the
fists, claws or two weapons (weapon class bit `0x80`) an attack strikes
with the motion's hand only: the right (first held) in the main side, the
stab and the overhead (both hands in a clawed overhead, class `0xC0`), the
left (second held) in the other side, which then becomes the primary
striker. Equipping the fists leaves the character without a primary
striker until its first attack (a 400 ms cooldown); equipping a single
weapon makes it the primary. In Exarust each hand gets a striker segment
from its origin across its box, as long as W21; a contact at its far end
counts 0.1 cm inside it, since the segment ends on the box's face where the
original's hand body is met within it ([PLAYER.md](../PLAYER.md), Combat
mode).

**Shield record**, item section `0x8000` (22 items): weight, block and
bulk = bytes 4, 5, 6 × 0.01, each in double precision then stored as a
float (a round shield 0.5, 0.58, 0.6; a buckler's weight 15 is 0.15, where
single precision gives 0.14999999). In the off
hand it gives parry effectiveness 0.5 + 1.5·(1 − weight)² (weight/2 with
the Mobility technique), a block leak of 1 − block/2, a stamina cost per
block of the leak (× 2/3 with Endure), and adds (2 − u)·bulk/4 to the
encumbrance.

**Step reach**: 80·√scale·(1 − 0.3·feral, when the feral level is over
0)·(1 − 0.2·g), in double precision (a feral level of 0.84 gives 59.84 cm,
single precision 59.840004); the long steps and the footwork are measured
in it. Exarust has one implementation (`weapon_record::step_reach`), which
a character's stats read in metres.

In Exarust every held weapon and fist is held by its character's own
stats (`CharacterStats::holder`, [RFC.md](RFC.md), Stats derived at
spawn): s its size (0.95 to 1.05 for a human by height), the scale its
body scale and g its encumbrance ([Encumbrance](#encumbrance)); no worn
item makes held weapons weightless yet (u 0: the shield-lightening
gauntlets are unpowered in every shipped record). The race fields come from
the race's decoded stats (`races.rdb`, `+0x58`, `+0x84`, `+0x88`, `+0x90`,
above), kept with the race index on each placed character's body; the
player and the duellist are human. A duellist guards beyond four reaches
([PLAYER.md](../PLAYER.md), The combat stance).

## Armour

Apparel item body (`0x1000`) fields:
- `0x150 + 4·(bone − 1)`: coverage percent (u32) per bone;
- `0x1A0`: padding ×0.005;
- `0x1A1..0x1A3`: slash, crush and pierce ×1/140;
- `0x1A5`: encumbrance ×0.0025 ([Encumbrance](#encumbrance));
- `0x1A6`: hit sound class (0 flesh, 1 cloth, 2 leather, 3 mail, 4 plate,
  6 golem);
- `0x214`: special kind (4 the golem suit, 6 gauntlets that lighten a held
  shield; 1, 2, 3, 5, 7 other item effects);
- `0x218` bit 0 and `0x219`: a power crystal is in and its charge (0xFF
  spent): the special item is powered;
- `0x220`: the articulated set a piece belongs to, the item id of the set's
  central piece (14 sets, e.g. 445 for the "UBE" plate), 0 for none.

An item made from a template takes the template's bytes scaled by its own
condition, c = (1 − 0.4·d/255)·(0.8 + 0.5·q/255) with q and d the
appearance block's quality and damage: padding and resistances × c,
rounded (half to even), at most 160; the coverage display byte × (0.3c +
0.7); encumbrance ÷ ((1 − 0.15·d/255)·(0.85 + 0.3·q/255)), rounded. The
armour pass then reads the bytes as the item stores them: the level
databases' instances store them scaled already (of those whose condition
changes their encumbrance byte, 381 store it scaled and 1 as the template;
of their protection bytes 2200 against 4), and a record wearing a game
template's id reads the template's bytes. Exarust reads what each item
stores and scales nothing again (`exa_combat::damage::armour_layers`).
Races add a natural inner layer: `races.rdb` coverage at `0x21C`,
protection at `0x11C`. Skeletons, golems, demons and the Fael head have
one.

Each body part keeps two layers, filled in equipment order. A new item
folds the old outer layer into the inner one:
- each layer's effective coverage is γ(c) = c, or c + 1.75(1 − c)(c − 0.5)
  above ½;
- padding adds as γ-weighted;
- resistances combine as s0′ + (1 − s0′)·γ1·s1, with s0′ = γ0·s0;
- the inner layer's coverage becomes 1.

An item's coverage of a part is its percent × 0.01 (in double, stored as a
float); with the Deflection technique learnt it is c + 0.1 up to ½, else c
+ 0.1·(1 − c) + 0.05, at most 1 (30 % covers 0.4, 75 % 0.825, 95 % all).

The 19×8 slot table at `0xA8` is not read for armour.

## Encumbrance

A character's one weight stat, implemented in `exa_formats::encumbrance`.
The original's armour pass sums it at spawn, after the character's init and
before the reach pass (the held items, a shield among them, are attached
before both), and again on every equipment change. Each step runs in double
and is stored as a float:
1. for each worn item in the record's order whose id finds an item with a
   body (an actor's local ids name its own database, a level's its own, the
   rest the game's): an unpowered golem suit adds 2.0, powered gauntlets of
   kind 6 make the held shield weightless (unpowered ones make it count
   again), then + encumbrance byte × 0.0025;
2. each articulated piece (set word ≠ 0) whose covered bones touch another
   articulated piece's takes 0.025 off, once: piece A touches piece B when
   A's bones meet the bones B's bones neighbour, or the other way round. The
   set ids are not compared, so pieces of different sets count too. The
   neighbours of each bone 1–18 include the bone itself (pelvis: femurs and
   lumbar; thorax: lumbar and head; …); the table gives the left humerus
   (15) the right shoulder (11) instead of the left one (14);
3. − 0.3 × the highest Manoeuvring in Armour tier learnt (0–3);
4. below 0.001 (negative included) it is 0;
5. with a shield held: (2 − weightless) × bulk / 4 (the shield section's
   byte 6 × 0.01), less 0.3 with the Mobility shield technique, is added
   when positive. The shield is the last of the first two held items that
   has a shield section and no weapon section (the left hand's light
   section would be taken first; none of the 22 shields has one);
6. at most 1.

Shipped values: the duellist's leather jacket (10) and full helm (60) give
0.175; a tourney plate harness, helm and leggings of one set (120, 80, 80,
all touching) 0.625. Of the story levels' 451 living characters 386 weigh
nothing (many learnt Manoeuvring III), 27 hold a shield, Elestan weighs
0.77 (0.47 of armour and a round shield's 0.3) and Demian 0.67; the arena's
opponent07 0.475 and opponent13 0.07 (Manoeuvring II takes its armour off,
its buckler's 0.07 stays).

What it slows, by encumbrance e (the readers' own precision):
- the step reach: 80 cm·√scale·(1 − 0.3·feral)·(1 − 0.2·e)
  ([RFC.md](RFC.md), Stats derived at spawn);
- the run blend: at most 1 − 0.3·e, 1 − 0.4·e with the sprint latch (Exarust
  applies the first to the player's run, [PLAYER.md](../PLAYER.md));
- the sprint gait's clip clock: × (1 − 0.1·e);
- the body's turn toward its heading (out of the free walk): × (1 − 0.6·e);
- the alignment a standing body settles to: 1 − e;
- a long step: (1.5 − 0.3·e) step reaches (the player's held step keys, the
  feet's long steps); the AI's long footwork step (1.6 − 0.3·e);
- the get-up clip clock: (1.5 − 0.5·e) × the clip-rate base;
- the held weapon's record: the holding pass takes e as its grip input
  (recovery, swing clock), and so does the unarmed hands' set-up.

Exarust sums it at spawn for every placed character (`exa_world`) and for
the player's and the duellist's outfits (the first painted item worn as
each model); `exa-tool characters` prints it. The step reach, the player's
run blend and the held weapons' and fists' records read it; the other
readers take it when they are ported.

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
- Maximum health is 0.25 × the race's factor (`races.rdb` `0xE4`: human 1,
  ogre 6, golem 2, …) with the init's race-byte rules, and health starts at
  the record's saved pools ([RFC.md](RFC.md), Stats derived at spawn).
- Applying a blow: see Applying damage below.
- Daze = √(0.75P + 1.25J)·stun·0.5·0.25/max health.
- Readiness: before the outcome, P, J and T are scaled by 1 − 0.5g, g the
  victim's combat readiness (0..1; it rises 0.004 a tick while the victim
  stands in its combat idle, [PLAYER.md](../PLAYER.md), The combat stance).
  The knockback step takes J before it, so a ready fighter is shoved as
  far but wounded half as much.

**Applying damage** (REA on the damage call, its two callers and the
low-health check, constants read from the executable; `Health::take` in
`exa_combat::fighter`)
- The call takes yellow and red amounts Y and R (a weapon blow's yellow
  and red above, already × 0.75; an impact's 0.5·d and 0) and whether the
  damage is vital (a weapon blow always; an impact once its accumulated
  strength passes 0.6). In order:
  1. the regeneration rate is set to 0 (below);
  2. yellow −= 0.1·Y, or 0.05·Y while a potion works (state flag
     0x800000);
  3. with no lasting wounds (rule bit 0x8: feral humans, skeletons and
     ancients, [RFC.md](RFC.md), Stats derived at spawn) yellow −= 0.2·R
     and red is untouched; otherwise red −= 0.2·R and yellow ×=
     red_new/red_old, or yellow = 0 when no red is left (red may go below
     0);
  4. yellow still 0.05 or more: done. Otherwise, unless yellow is under
     0.01 **and** the damage is vital, the character lives and the
     low-health check runs (yellow is set to 0 if it was under 0.01): a
     second wind for one wearing the armour that grants it, else, for the
     player, the heartbeat ([UI.md](../UI.md), Heartbeat). A vital one
     under 0.01 kills: yellow 0, in the story red −= 0.1 × the maximum,
     and the collapse starts ([RSQ.md](RSQ.md), The collapse).
- Precision: each subtraction in double precision on the single-precision
  values (0.1, 0.05 and 0.2 are doubles), the red ratio in single, the
  0.05 test in single, the 0.01 test in double, so a yellow of exactly
  0.01 in single precision (0.0099999998) dies.
- Worked numbers: from full health (0.25), Y 1 and R 0.5 leave red 0.15
  and yellow 0.09; Y 0.5 and R 0.3 leave a human 0.152 / 0.19, a feral
  human 0.14 / 0.25, a human under a potion 0.171 / 0.19; a killing blow
  in the story takes a red of 0.2 to 0.175. In the installed story levels
  248 of the 451 living characters keep their red under every blow: the
  97 feral humans, 128 skeletons and 23 ancients (corpus test in
  `exa_combat::fighter`).
- The call also spares a character whose "no death" byte is set and calls
  a player hook on a killing blow; nothing in the executable ever sets
  either, so Exarust has neither. Story mode is the game mode of the story
  levels (practice and the arena clear it).
- Yellow under 0.01 on a living character (in double precision): the
  player's combat stance is the Defence stance ([PLAYER.md](../PLAYER.md),
  The combat stance); the stance by energy (the swing stall's, a cleared
  parry's, the AI's other branch) gives the Defence stance; the
  auto-parry clears any parry and starts none. Only `--health` makes it
  happen in Exarust today: the weapon blow is always vital.

**Regeneration** (REA on the body tick's prologue and health bookkeeping,
the damage call and the weapon damage routine, constants read from the
executable; a retail trace; `exa_combat::regen`)
- Each tick of a character not dead (a collapsing one included), after
  its posture and attack and before its clip advances: its rate r is 0
  while it attacks (condition 2) or while Cmbt - Idle plays (whatever the
  condition, so a weak fighter in it too); otherwise r rises by
  0.0003 × C a tick (in double precision), at most C = 0.01/83 (a float
  in the image, never written), and yellow gains √red × r (in float) while
  red is above 1.19209e-7. The ramp takes 3,334 ticks (40.2 s); at the cap
  a human with full red heals 0.005 a second, the whole bar in 50 s.
- r is set to 0 by every blow that deals health and by every weapon
  contact on a character, before the damage routine decides what the
  contact does; burning resets it too (not ported).
- Red heals only from the heal pool (bandages fill it; nothing does in
  Exarust yet): 0.25 × 5e-5 = 1.25e-5 a tick moves from the pool to red
  (a pool of 0.2 drains in 15,999 ticks, 193 s), and, dead or alive (the
  test comes before the dead test), for bodies whose rule bits hold 0x20
  without 0x100 ([RFC.md](RFC.md), Stats derived at spawn): 1.25e-5 a
  tick, 0.00104 a second. The ogre, Sir, the harvester and the demon have
  it; a golem suit has it only while it wears its core. In the installed
  story levels Sir and the three harvesters regenerate red; in the arena
  the demon.
- Then yellow is kept within red, and after it red within the maximum
  (so yellow follows a red cut to the maximum a tick later); at the end
  of the tick yellow is at least 0. Death sets yellow to 0 (the collapse
  regenerates about 0.0006 of it over its 250 ticks).
- The retail trace of the player after a fight: yellow 0.085, held for
  7.5 s in Cmbt - Idle, then in Idle 0.086, 0.090, 0.100, 0.110 and 0.121
  at 5.0, 9.6, 16.2, 21.1 and 25.3 s, 0.202 at 45.8 s; the rule with red
  0.203 gives 0.0864, 0.0902, 0.0997, 0.1100, 0.1209 and 0.2009. The
  Defence stance heals; Cmbt - Idle held yellow for 66 s in the same
  trace.
- Exarust: a light `--still-opponent --at 0,0,1 --health 0.085,0.203
  --press Tab@50 play arenao1` run, combat mode off at frame 50, logs
  yellow 0.086 at 4.4 s, 0.101 at 16.9 s, 0.118 at 24.4 s and 0.172 at
  39.4 s (the rule to the log's three decimals) and the full rate 40.16 s
  after the Tab. Since the health bookkeeping moved into each living
  character's tick (red regeneration in its upkeep stage, the rest in its
  attack stage after its posture; the dead and any character without a
  living body still once a fixed tick before the character ticks), the
  same run in the arena reaches the full rate at yellow 0.1755, the rule's
  value.

**What the struck character does** (REA on the damage routine and what it
calls)
- It starts no clip. The damage routine starts clips only through the
  striker's recoil, on blocked and clashing hits. The struck character
  takes the daze (energy × (1 − min(daze, 0.9)), at least 0.05), pain
  voices, and on J > 0.5 a knockback step: 2·(20·s + 10)·J cm along the
  swing flattened to the horizontal, s the Force Strikes level (0 without
  the power), plus 20 ticks of its chest's smoothed velocity, capped at
  its step length. Exarust starts that step when impact is over 0.5, and
  shoves the struck part by 10J cm/tick plus a tenth of the blade's
  velocity only when it is not.
- A character struck while swinging has its swing stalled. With D the daze
  (at most 0.9), f a factor of the blow (1 for a human in most blows),
  reach its attack reach (cm) and w its weapon's recoil factor,
  r = 4·D·f − (1 − D)·min(0.005·reach, 1)·w. When r ≥ 0.001 its swing
  strength P falls by r each tick and scales the swing's clock, and at
  min(80·P/r ms, the swing's remaining time) it returns to its stance,
  blended over a time from its weapon record. Exarust has no swing
  strength yet: a struck attacker returns to its stance at once and its
  blade deals no further hit.
- Rebalance, Defence stance, staggers and falls come from balance, not
  from the blow ([RSQ.md](RSQ.md), Falls and get-ups). Head Pain is a story clip ([RSQ.md](RSQ.md), Motion set).
  Defence stance with a step also answers electricity and energy effects.
- A lethal wound plays Fall Backward and collapses the character for 3 s
  before it dies ([RSQ.md](RSQ.md), The collapse), as Exarust does now.
- It bleeds (REA on the damage routine's blood branch, the wound queue and
  its consumer in the body tick) when its race bleeds (`races.rdb` flag
  bit 0) and the wound's size R = P/(P + 1)·m, before the 0.75 of red,
  exceeds 0.05. The wound waits on the victim, at most seven (a new one is
  dropped when seven wait), with the striking body (the weapon, or the
  hand), the contact point as weights in its tetrahedron, the world point
  and R. Each body tick counts every waiting wound down from 7; the first
  whose count reaches 0 is painted, one a tick, corpses included: 7 ticks
  (84 ms) after the blow (the blow is dealt in the striker's post-step,
  after that tick's body ticks, so the count starts on the next tick).
  The weights give where that point of the
  striker is then; D, how it moved on from the contact point, smears the
  wound, of radius 30·R + 10 cm ([RENDERING.md](../RENDERING.md), Blood).
  A punch bleeds on the struck only: hands never take blood themselves.
- The weapon that deals a bleeding blow draws 0.5·R of blood at once, kept
  within 0 to 1 and never washed off, whether the wound waits or is
  dropped (REA on the same branch: the striking item's blood slot, the add
  routine's 0.5 and clamp); hands have none. It stains the weapon from
  its tip ([RENDERING.md](../RENDERING.md), Blood on weapons).

A parry stops the blow. A shield that is not parrying lets through yellow
0.05·(1 − stamina)·(0.4 + 0.4·impact)·(1 − block/2) and no lasting wound
(`damage::guard_outcome`). Not yet: the bite (weapons sticking in), the
parry clips, grabs, shields as bodies,
encumbrance's effect on swings, and the state that knocks a character
unconscious.

## Actors

`actors.rpk` inside Resource: 21 entries, each a character record (the
layout of [RFC.md](RFC.md#starts-characters-and-placed-objects), versions 6
and 7: 14 and 7) followed by the actor's own item database (`DBDB`):
spawnable character templates with loadouts (`ArenaShpKp`, `opponent01..16`,
`Demon A01`, `proctor`, `villager`, `Derrin`). Every record parses to its
end (`exa-tool characters actors`); the opponents climb from no skills to
opponent16's Close Combat 20000 and Armour 25000 (Manoeuvring tier 3,
Deflection, Security). The sixteen opponents and the proctor store empty
pools (0, either version); ArenaShpKp, the demon, the villager and Derrin
store full yellow and red.

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
