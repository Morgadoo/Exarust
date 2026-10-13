# Exanima / Rayform: what's publicly known online

Research date: 2026-10-13. Current game build at that date: **Exanima 0.9.5.2g** (Steam app 362490).

The Exarust project is a Rust/Bevy reimplementation that loads the original assets. This document collects what the community has published about Exanima, its in-house engine (Rayform), its file formats and its gameplay. Every claim comes with its source URL. Statements marked **[inference]** are my own conclusions from the cited evidence and should be checked against real files.

> Copyright note: this document summarises facts and paraphrases sources. It avoids copying long passages. File-format facts such as offsets and magic numbers are summarised in our own tables. Several community repositories have no licence, a non-commercial licence or a "source-available, no redistribution" licence (see the tools table). **Do not copy code from those repositories into Exarust.** Re-derive the code from the format facts instead.

---

## 0. TL;DR, the most important findings

1. **https://rea.tools/ has nothing to do with Exanima.** It is "REA: Reverse Engineer Anything", an MIT-licensed MCP server and agent toolkit for reverse engineering binaries, Electron apps and websites (https://github.com/morluto/rea). It contains no Exanima format docs; its showcases are DX-Ball, Touhou, Notion and similar targets (https://rea.tools/showcase/, https://rea.tools/blog/). It could be used as a *tool* to study `Exanima.exe` with Ghidra, IDA or Hopper.
2. **The single best format reference is "Structs.txt"**, an 83 KB pseudocode spec maintained by *Inuk/Drinkynuk* and the "Council of Conservers" modding Discord. It sits in a public Google Drive folder: https://drive.google.com/drive/folders/1AKXMzu8YgQRmccfkEjlysonGguPX5-Ei (file id `1VJ7dUHDCnVe6PmH-_X2KvswGjZ7s6D1E`). It documents RPK, RDB, RFP, RFC (scene, meshes, lights, tilemap, object placement), RSQ (animation), RFI, RFT (terrain), PWR, FTY (factories), RSG/RCP (saves), `.set` and `.ini`. It was written for 0.8.x/early 0.9 (32-bit era), so some structs have changed since.
3. **The Exanima Modding Toolkit (EMTK)** is written in Rust, licensed MIT/Apache-2.0, and has the cleanest open-source code (RPK reader/writer, a magic-number table, an RFI header parser). Repo: https://codeberg.org/ExanimaModding/Toolkit; book: https://exanimamodding.codeberg.page/Book/
4. **The RFI RLE codec is fully solved.** The Turkish and Chinese translation projects have byte-exact decoders/encoders:
   - https://github.com/Psychenoumenon/exanima-turkce-yama
   - https://github.com/Myron-Owl/exanima-zh-cn
5. **Engine facts:**
   - Written in Delphi/Object Pascal with lots of hand-written assembly.
   - Renderer: OpenGL with GLSL shaders, generated dynamically. It is a clustered-style forward renderer with no hard light limit, voxel-based GI ("VXGI"), dynamic shadows from many lights, displacement detail shadows and a "supersampling" scheme that renders jittered copies of the scene.
   - Physics: fully custom, with continuous collision detection and a cloth/soft-body system. Characters are driven by virtual muscles plus a "dynamic balancer".
   - The game went 32→64-bit in 0.9.5 (May 2026).
   - Sources: Steam devlogs, the Cruor mod source and the ExanimaREP IDA header (§5).

---

## 1. Sources

- Official site: https://www.baremettle.com/ (news archive at https://www.baremettle.com/news/, forums, store). The news posts cover releases and include combat and development videos; there are no engine write-ups on the site.

### 1.1 Official (Bare Mettle)
| What | URL |
|---|---|
| Steam store page | https://store.steampowered.com/app/362490/Exanima/ |
| Steam news / devlogs (full text through the public API) | https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/?appid=362490&count=200&maxlength=0&feeds=steam_community_announcements |
| Steam news list | https://store.steampowered.com/news/?appids=362490&feed=steam_community_announcements |
| Exanima 0.9 announcement (forum) | https://www.baremettle.com/forums/index.php?threads/exanima-0-9.18340/ |
| Exanima 0.9 announcement (Steam) | https://steamcommunity.com/games/362490/announcements/detail/5711185603042059282 |
| Bare Mettle games page | https://www.baremettle.com/games/ |
| Official quick-start guide (PDF) | http://cdn.akamai.steamstatic.com/steam/apps/362490/manuals/Exanima_Quickstart_Guide.pdf |
| Sui Generis Kickstarter (blocked to fetch, cited by others) | https://www.kickstarter.com/projects/1473965863/sui-generis |
| Interview with lead developer Madoc Evans (blocked to fetch) | https://www.pcinvasion.com/sui-generis-interview-with-bare-mettles-madoc-evans/ |

Key devlogs, all reachable through the Steam news API above:
- 0.5.2 (2015-05-21): "OpenGL based rather than DirectX"
- 0.6 (2015-11)
- New Arena beta 0.6.1.0 (2015-12-31)
- 0.6.1.6, 0.6.1.7, 0.6.4.3, 0.6.5.2
- 0.7.0 (2018-07-10)
- 0.7.2 (2019-05-06)
- 0.8 (2020-04-01)
- 0.8.3 (2021-12-19): new renderer, GI, procedural weapons, arena overhaul
- 0.8.4 (2022-07-17): damage rework
- 0.9 (2023-10-13)
- Monthly "Coffee Diary" posts, 2023-11 to 2026-10, especially:
  - Jan 2024: 64-bit port
  - May 2024: physics rewrite
  - Feb 2025: displacement shadows, GI
  - Mar 2025: apparel system
  - Apr 2026: forward rendering, shader compilation, streaming
  - Oct 2026: networking
- 0.9.5 (2026-05-20): 64-bit, physics overhaul, streaming, tools
- 0.9.5.2 (2026-08-18): save system

### 1.2 Community format documentation
| What | URL |
|---|---|
| **Structs.txt** (Inuk / Council of Conservers) | https://drive.google.com/drive/folders/1AKXMzu8YgQRmccfkEjlysonGguPX5-Ei |
| Drinky Toolset README (Google Doc) | https://docs.google.com/document/d/1Z-mXzIvemm3QgK0nXAe3muuC379wejSzDqsGAdG68B4 |
| EMTK Book: file table | https://exanimamodding.codeberg.page/Book/unorganised/files/ |
| EMTK Book: RPK | https://exanimamodding.codeberg.page/Book/contributing/rpk.html |
| EMTK Book: engine details | https://exanimamodding.codeberg.page/Book/contributing/game_engine/engine_details.html |
| EMTK Book: calling conventions | https://exanimamodding.codeberg.page/Book/contributing/game_engine/ffi/calling_conventions.html |
| EMTK Book: Delphi VMTs | https://exanimamodding.codeberg.page/Book/unorganised/delphi_notes/virtual_method_tables.html |
| EMTK Book: converting assets | https://exanimamodding.codeberg.page/Book/developing_plugins/converting_game_assets.html |
| ZenHAX RPK thread + QuickBMS script | http://zenhax.com/viewtopic.php@t=2128.html , http://aluigi.org/bms/exanima.bms |
| ExanimaREP (IDA 7.7 database + C header, 0.9.1.6) | https://github.com/Horse4Horse/ExanimaREP |

### 1.3 Community / wiki / gameplay
| What | URL |
|---|---|
| Sui Generis/Exanima Fandom wiki (read through the MediaWiki API) | https://suigeneris.fandom.com/wiki/Skill , /Close_Combat , /Shields , /Armour , /Insight , /Concentration , /Thaumaturgy , /Weapons , /How_to_play_guide_for_Exanima |
| Nexus Mods (11 mods, mostly trainers/saves) | https://www.nexusmods.com/games/exanima/mods |
| Council of Conservers Discord (unofficial modding hub) | invites seen: https://discord.gg/KWrCfZbskt , https://discord.com/invite/XGZNAsEtCH , discord.gg/bcgXg3VDhQ |
| Jango's modding videos (RFC Editor, Blender) | https://www.youtube.com/@Jango_- |
| Jango's tool drive | https://drive.google.com/drive/folders/1ZWaXJaf1l6iGPRd4TPD6rtOrJrISTHrd |
| Custom model tutorial (credits "Inuk and Jango for reverse engineering the file formats") | https://www.youtube.com/watch?v=dnEhnjGq36g , https://www.youtube.com/watch?v=DS3IhZVFnxs |

---

## 2. Community tools

The licence column comes from each repo's metadata or LICENSE file. "none" means no licence was found: all rights reserved, so read for facts only.

| Tool | URL | Language | Licence | Format knowledge inside |
|---|---|---|---|---|
| **Exanima Modding Toolkit (EMTK)** | https://codeberg.org/ExanimaModding/Toolkit (mirror https://github.com/ExanimaModding/Toolkit) | Rust | MIT OR Apache-2.0 | `emtk_asset` crate (deku-based): complete RPK reader/writer with nested-path loading (`Resource.rpk/actors.rpk/opponent03`) and loose-folder overlay. A launcher injects a DLL (MS Detours) that hooks `ReadFile` and transparently rewrites RPK tables to add or replace entries (`emtk_framework/src/internal/utils/rpk_intercept.rs`). Git history (2023–2025) also has: a C++ "Repacker" (`Repacker/formats/*.cpp`), a Rust `repacker` that converts RFI→DDS, a full magic table (`crates/repacker/src/metadata.rs`), an `exparser` crate, Python bindings and a stub Blender RFC import/export add-on. |
| EMTK example mods | https://codeberg.org/ExanimaModding/Mods | Rust | (Toolkit's) | `godmode` hooks `proc_dmg_and_stamina(motile_ptr, f32)` by signature; `godhand` ports byte patches from Horse4Horse's cheat table (range limits, weight, interact while fallen). |
| **Drinky toolset** (Inuk/Drinkynuk) | https://drive.google.com/drive/folders/1AKXMzu8YgQRmccfkEjlysonGguPX5-Ei | Python | none stated | `Structs.txt` (spec); `DrinkyRPK.py` (RPK/RML pack/unpack and in-place insertion); `DrinkyChar.py` (0.8 save/char/objdb ⇄ text "pylit"); `DrinkyPatch.py` (exe patcher: slider limits, races, decomposition); `RFC Assembler.py` (splits RFC into chunk files); `FactoriesRPKTool.py`; `io_scene_rfc_hiatus.zip` (Blender RFC add-on: whole-RFC round trip, vertex and position edits only); "Item tables" (objdb spreadsheet). I did not download the .py and .zip files (Drive shows a virus-scan interstitial). |
| **Jango's RFC Editor** | https://drive.google.com/drive/folders/1ZWaXJaf1l6iGPRd4TPD6rtOrJrISTHrd ; videos on https://www.youtube.com/@Jango_- | Python (Blender add-on) | none stated | The most advanced RFC tooling, maintained up to 2025: tile/level editor, character editor (creation, equipment, relations, roles), factory editor (procedural weapon parts), terrain brush editor, RSG packing, a procedural level generator, and "Mesh Editor v2" for custom weapon/armour models. Distributed as zips (not downloaded). This is where current (0.9.x) RFC/RSG knowledge lives. |
| ExanimaResourceEditor | https://github.com/PoofImaFox/ExanimaResourceEditor | C# | MIT | RPK unpack/repack GUI. Tags entries as wav (`RIFF`) or rfi ("dds", `C6 3D 2D 1D`) by magic. |
| QuickBMS script | http://aluigi.org/bms/exanima.bms | BMS | – | Minimal RPK extractor (header, 32-byte entries). |
| ExanimaREP | https://github.com/Horse4Horse/ExanimaREP | IDA db + C header | none stated | 0.9.1.6 (64-bit test build): about 330 named structs/classes (§5.4) with field names such as `TTetraRBD`, `TSGCharacter`, `TVertexData`, `TGLGeometry`. |
| Horse4Horse ExanimaFormMaker | https://github.com/Horse4Horse/ExanimaFormMaker | C++ | GPL-3.0 | Spawns in-game GUI forms by calling engine GUI functions (signatures for 0.9.1.6c). |
| Horse4Horse ExanimaModLoader | https://github.com/Horse4Horse/ExanimaModLoader | C++ | none stated | `winmm.dll` proxy that loads DLLs from `mods/`. |
| gnalvesteffer exanima-mods | https://github.com/gnalvesteffer/exanima-mods | C++ | CC BY-NC-SA 4.0 | Old 32-bit memory structs (Character, ContainerItem 0x13 bytes), a Cheat Engine table and a ReClass.NET file. Game-mode enum: 0 menu, 1 novice practice, 2 expert practice, 4 story, 8 arena. |
| **Exanima Turkish patch** | https://github.com/Psychenoumenon/exanima-turkce-yama | Python | source-available, **no redistribution or modification** | Byte-exact **RFI RLE codec**; RDB reader/writer (`02 0C BF AF` string libs); books embedded in level RFCs; PWR text tables; `charroles.rdb` dialogue blobs and script syntax; font advance-width tables in the exe. |
| **Exanima zh-CN patch** | https://github.com/Myron-Owl/exanima-zh-cn | Python + C (runtime DLL via `winmm.dll` proxy) | none (README says no licence chosen yet) | RFI codec, RPK rebuild, RDB rebuild, finding RFI blobs embedded in the exe, exe AnsiString patching (8-byte length prefix), `.fds` manual extraction. Lists SHA-256 of the 0.9.5.2 exe and `Resource.rpk`. |
| Exanima Physique-matters | https://github.com/Justin52361/Exanima-Physique-matters | Python | MIT | 64-bit exe internals: `TMotileDynamics`, `TSGCharacter`, hit routine, attack-animation clock (§5.5). |
| Cruor (physics blood) | https://github.com/Monochrome94/Cruor | Rust (EMTK plugin) | none stated | Very detailed notes on the **OpenGL render pipeline**: the "Global" std140 UBO layout, `ObjMat` uniforms, supersampling copies, `TWeaponBlood`, hit code (§5.2). |
| EXAMINER | https://github.com/reirao/EXAMINER | Rust | MIT | Input-hook experiment (WH_GETMESSAGE). No format knowledge. |
| Exanima Mod Launcher | https://github.com/PilotClassic/Exanima-Mod-Launcher | C# | MIT | Reads the logo and bitmap fonts from `Resource.rpk` (RFI decode notes), edits `Exanima.ini`. |
| Xaver (save manager) | https://github.com/HadwynLink/Xaver | Rust | none stated | Scans `.rsg` for level ids (`Zoexanima01` …) and the player name. |
| ExanimaTimeThaumaturgy, SaveScum, Savegame Manager, save-to-checkpoint | https://github.com/SerGreen/ExanimaTimeThaumaturgy , https://github.com/Bjwills7/SaveScum , https://github.com/Tzirrit/ExanimaSavegameManager , https://github.com/VitalyArtemiev/exanima-save-to-checkpoint | Py / JS / C# / PS | none | Save files live in `%APPDATA%\Exanima`. Renaming `.rsg`→`.rcp` turns a save into a checkpoint. |
| MapExanimaC / exanimaMap / exanimapHelper | https://github.com/staniBosch/MapExanimaC , https://github.com/englishpancake/exanimaMap , https://github.com/Sokkero/exanimapHelper | C++ / C# | none | Read player X/Y from memory (`Exanima.exe+48DDD0` / `+48DDD8` in a recent build) for a minimap. |
| exanimaHeal / ExanimaHack | https://github.com/englishpancake/exanimaHeal , https://github.com/jereremy/ExanimaHack | C / C++ | none | Health-field offsets (yellow/red/blue) and a character state enum (§6.6). |
| ExanimaTools (Xiao-Houzi) | https://github.com/Xiao-Houzi/ExanimaTools | C# | none | Arena team-manager app. Its stat tables are *approximate/AI-generated "pip" values*, not game data, so treat them as **unreliable**. Its armour-layering notes match the wiki. |
| exrpc | https://github.com/ProffDea/exrpc | Rust | none | Discord RPC through EMTK. |

Not found on GitHub, GitLab or Codeberg: any Noesis plugin, a standalone model viewer, or a skeleton/animation (RSQ) exporter. Searches covered "exanima noesis", "exanima blender import", "rayform engine", "exanima model viewer" and `gh search code` for the magics. Model work happens in Jango's and Inuk's Blender add-ons distributed through Discord and Drive.

---

## 3. File format knowledge

### 3.0 Global conventions
- **Endianness:** little-endian everywhere. Every tool reads `<I`.
- **Integer sizes:** Structs.txt was written when Exanima was 32-bit. It uses i32 throughout and states the game "is not compiled for 64-bit"; that changed in 0.9.5, but the *file* formats stayed 32-bit-field based. Source: Structs.txt; Steam 0.9.5 devlog.
- **`str128`:** a fixed 16-byte name field, used for asset names, character names, material names and so on. The game compares all 16 bytes as one 128-bit value. Two consequences:
  - Names are **not necessarily NUL-terminated**.
  - **All padding bytes matter**, so always zero-fill them when writing. (Structs.txt)
- **`lenstr`:** `i32 len; u8[len]`, with no terminator. (Structs.txt)
- **`chunk`:** `i32 type; i32 size`. `size` counts the bytes *after* the 8-byte header. Chunk arrays are pervasive, and unknown chunks can be skipped by `size`. (Structs.txt)
- **`heavychunk`:** same as `chunk`, but `size` *includes* the 8-byte header. It is used in `Exanima.set` and in object (item) class definitions. (Structs.txt)
- **Object/character reference ids:**
  - Low 28 bits: the id.
  - High nibble flags: `0x8000_0000` = local db (the level's objdb/chardb, or an actor's), `0x4000_0000` = the save's `#worldobjdb` / `#worldchrdb`, `0` = global `Resource.rpk/objdb.rdb`.
  - Source: Structs.txt (`held_item_s`, `characterposchunk_s`, DBaseNodes).
- **Matrix:** 3×4 floats (a 4×4 with the last column dropped), rows `[a b c][d e f][g h i][x y z]`. Translation is in the last row. Structs.txt gives the mapping to Blender, which swaps Y and Z: Exanima appears to be **Y-up**, Blender is Z-up **[inference from the mapping]**.
- **World units:**
  - One dungeon tile is 600 units wide, and an object 100 units tall raises z by 100 (Structs.txt).
  - Cruor works in cm against game coordinates, e.g. "hips are this high above the feet (cm)" (https://github.com/Monochrome94/Cruor).
  - **[inference]** 1 unit = 1 cm, so a tile is 6 m.
  - Note the axis-naming conflict: Structs.txt calls height "z", but its own matrix mapping suggests Y-up. Verify against real files.
- **Strings:** ANSI/Windows-1252 (translators write cp1254/latin-1). The manual (`.fds`) decodes as cp1252. (zh-CN `tools/export_resource_text.py`)

### 3.1 Magic-number table (consolidated)
Values are u32 little-endian as read from offset 0; the on-disk byte order is shown for the main types.

| Ext | Name | Magic (u32 LE) | Bytes on disk | Notes / sources |
|---|---|---|---|---|
| `.rpk` | Rayform Package | `0xAFBF0C01` | `01 0C BF AF` | Shared by `.fds`, `.flb`, `.rml` (all the same container). EMTK, Structs.txt |
| `.fds` | (manual "document set"?) | `0xAFBF0C01` | | RPK container whose entries are manual chapters (cp1252 text); the entry `id` field is used. Structs.txt, zh-CN |
| `.flb` | unknown library | `0xAFBF0C01` | | RPK container. EMTK |
| `.rml` | Rayform Material Library | `0xAFBF0C01` | | RPK container, listed in the RFP (`Resource\Common.rml`, `Resource\Underworld0.rml`, `Resource\UnderworldB.rml`). Structs.txt |
| `.rdb` (strings) | String library | `0xAFBF0C02` | `02 0C BF AF` | `objstrings.rdb`, and embedded in level RFCs (books/notes). Turkish patch `tr_rdb.py`, Structs.txt |
| `.rdb` (objects) | Object DB | `0xDBAC0D03`…`0xDBAC0D08` | | Versioned; 08 is current since 0.8.3. Structs.txt |
| `.rdb` (chars) | Character DB | `0xDBACCD00` | | Inside chunk `0xCECDDB00`. Structs.txt |
| `locales.rdb` | Locale DB | `0xDBAE0D01` | | Structs.txt |
| `charroles.rdb` | Role DB | `0xDBCB0D01` | | Dialogue/AI roles. Structs.txt |
| `.rfp` | Rayform Project | `0xAFDFBD10` | `10 BD DF AF` | `Exanima.rfp`. The check is `sig & 0xAFDFBD10 == 0xAFDFBD10`. Structs.txt |
| `.rfc` | Rayform Content | `0x3D23AFCF` (also `0x3D21AFCF`; code handles `0x3D22`) | `CF AF 23 3D` | Really `u16 0xAFCF` + `u16 FmtVersion`. EMTK, Structs.txt |
| `.rfi` | Rayform Image | `0x1D2D3DC6` | `C6 3D 2D 1D` | §3.6 |
| `.rft` | Rayform Terrain | `0x3EEFAD01` | | §3.9 |
| `.ftb` | Terrain palette/brush | `0x3EEFBD01` | | Undocumented. EMTK, Structs.txt |
| `.fty` | Factory (procedural item recipe) | `0xAFCE0F00`, `0xAFCE0F01` (pre-0.9: `0xAFCE010F`) | | §3.11 |
| `.pwr` | Thaumaturgy power tree | `0xAFCE01CE` | | §3.10 |
| `.rsg` / `.rcp` | Save game / checkpoint | `0xDA7AEA02` (pre-0.5.3: `0xDA7A5A01`) | | §3.14 |
| `.rsq` | Rayform Sequence (animation) | first chunk `0x3D000000` | | §3.8 |
| `.set` | Settings/progress (`Exanima.set`) | `0x6440A401` listed by EMTK | | Actually a heavychunk array (§3.15) |
| `.wav` | RIFF WAVE | `0x46464952` ("RIFF") | | Plain RIFF. EMTK |
| `.rcr` | Character (role?) record | `0x00CDAC06` | | Same as the character chunk type v6 (§3.12) **[inference]** |
| `.rcd` | Character dialogue | – | | Contains `[u32 len][ASCII]` strings. zh-CN scans it for text |
| `.rui` | Rayform UI | `0x615B0A0D` | | EMTK |
| `.rab` | Rayform Ambience | `0x7EF6DC8A` | | EMTK |
| `.rpp` | Rayform Post-Processing | `0xDCEACCD2` | | EMTK |
| `.det` | unknown | `0xDCD2EC40` | | EMTK |
| `.rva` | level-related, obsolete | `0xECFFAD01` | | Structs.txt |
| unknown | – | `0x7EF6D298`, `0x3F49C9CA`, `0xC2D8DEF2`, `0xCAEC68C6`, `0x7EF6D0A6`, `0xD2C8CCCC`, `0xCAEC66C6` | | "IDK1–7" in EMTK `metadata.rs` |

### 3.2 Game directory layout and packages
Per the EMTK book (https://exanimamodding.codeberg.page/Book/unorganised/files/):

| Package | Contents | Depends on |
|---|---|---|
| `Resource.rpk` | Core data: FDS, FLB, FTB, PWR, RCD, RDB, RFC, RFI, RFT, RML, nested RPKs (e.g. `actors.rpk`), RSQ | – |
| `Apparel.rpk` | RFI textures for clothing/armour | – |
| `Characters.rpk` | RFC character and apparel models | Apparel.rpk |
| `CharExt.rpk` | RFC hair models | Textures.rpk |
| `Textures.rpk` | RFI textures for CharExt | – |
| `Components.rpk` | RFC weapon component models | ComponentTex.rpk |
| `ComponentTex.rpk` | RFI textures for Components | – |
| `Factories.rpk` | FTY procedural weapon (and apparel) recipes | Components.rpk |
| `Objects.rpk` | RFC level and object models (weapons, shields) | – |
| `Objlib.rpk` | RFC "lib" utility props placed through LibNodes | – |
| `Sound.rpk` | WAV | – |

- **RDB files in `Resource.rpk`:** `charroles.rdb` (roles), `locales.rdb`, `objdb.rdb` (master item DB), `objstrings.rdb` (item names and descriptions). (EMTK book)
- **Texture suffixes:** `*d` diffuse, `*e` emissive, `*g` gloss, `*h` height, `*n` normal, `*s` specular. (EMTK book)
- **Level ids:**
  - `exanima01`, `exanima02`, `exanimac1` (Catacombs), `exanima03`, `exanima04` (Archives), `exanima05` (Crossroads), `exanima05_sw` (Crossroads sewers), `exanima06` (Golem Forge), `exanima07` (Market), `exanima07_sw`, `exanima08` (Gentry), `exanima67` (Gardens), `exanima67_c1` (Garden crypts); `arenahub` for arena.
  - In saves the ids appear prefixed, e.g. `Zoexanima01`.
  - Source: https://github.com/HadwynLink/Xaver `src/savemanager.rs`.
- **Save folder:** `%APPDATA%\Exanima\`: `ExanimaN.rsg`, `ArenaN.rsg`, `*.rcp`, `Exanima.set`, `Exanima.ini`, `Exanima.log`.
- **Hidden `userdata` folder:** if a folder named `userdata` exists in the game directory, it is used instead of the AppData save folder. (Drinky README)
- **Exe and project file names are tied together:** renaming `Exanima.exe` requires renaming `Exanima.rfp` too. (Structs.txt)

### 3.3 RPK container (`.rpk`, `.fds`, `.flb`, `.rml`)
Sources: EMTK `emtk_asset/src/lib.rs`, EMTK book, Structs.txt, QuickBMS, ERE.

```
u32 magic        = 0xAFBF0C01
u32 table_size   ; bytes; entry_count = table_size / 32
Entry[entry_count]:
  u8[16] name    ; str128: not NUL-terminated if 16 chars, zero-padded; includes extension (e.g. "objdb.rdb", "actors.rpk")
  u32 offset     ; relative to data_start = 8 + table_size
  u32 size
  u32 unknown    ; always 0
  u32 id         ; 0 except in .fds (maybe hash/search key)
data...          ; no padding/alignment between entries in official files; order arbitrary
```

- **Sorting:** entries **must be sorted lexicographically by raw bytes**. The game binary-searches the table. (Structs.txt)
- **Nesting:** RPKs nest (`Resource.rpk/actors.rpk/<name>`). Nested offsets are relative to the nested RPK's own start. (EMTK `Package::load`)
- **"RPK overflow":** before 0.8.3h, offsets were read as signed, so packages over 2 GiB broke. (Structs.txt)
- **Load priority:** the RFP lists packages in priority order, and an earlier RPK overrides later ones. (Structs.txt)

### 3.4 RDB database family
Source: Structs.txt; Turkish `tr_rdb.py`; zh-CN `read_rdb`.

```
u32 uid          ; subtype magic (see 3.1)
u32 table_size   ; bytes, multiple of 16; ALWAYS includes one extra trailing garbage entry
Entry[(table_size/16) - 1]:
  u32 id         ; dictionary key (unused in string libs: strings are indexed by order, first = 1)
  u32 flags
  u32 offset     ; relative to 8 + table_size
  u32 size
u8[16] trash     ; uninitialised (present only if table_size > 16)
data pool        ; entries may share offsets (deduplicated strings); some RDBs pad entry data
```

- **Empty RDB edge case:** `table_size` is still `0x10`, but the trash entry is absent.
- **`stringlib` (`0xAFBF0C02`):** non-terminated ANSI strings. When it sits inside a level, it lives in chunk `0x0000DBCD` within the `0x0000DBC0` array.
- **`rdb_objs` (`objdb`):** each entry is an *array of `obj_class_s` heavychunks* (§3.13). The entry `flags` field is the OR of the class types present.
- **`rdb_chars`:** each entry holds one character chunk (§3.12).
- **`charroles.rdb` (`0xDBCB0D01`):** entries point to `charrole_s`:
  - `str128 name`, `parent`, relations `(role_a, approval, role_b, code)`.
  - Topic sets: participants with personality, topics with editor node coordinates, and recursive branches.
  - A branch holds `texts[(speaker, lenstr)]`, a `condition` code string, an action `code` string and nested responses.
  - The scripting language is C-like, e.g. `result = (Actor[0].Health < 60);` or `result = (TopicUsed(Who are you?) = 0);`.
  - Dialogue topics are referenced from scripts *by their visible text*.
  - Some role blobs contain nested size fields (e.g. `u32 @0x38` + 56 = blob length).
  - Sources: Structs.txt; https://github.com/Psychenoumenon/exanima-turkce-yama `tr_charroles.py`.

### 3.5 RFP project file (`Exanima.rfp`)
Source: Structs.txt.

```
u32 signature = 0xAFDFBD10
u32 flags (0)
str128 project_name  ; "Exanima" (also the %APPDATA% folder name)
lenstr unused (len 0)
lenstr resource_name ; "Resource"
u32 lib_n; lenstr[lib_n] objlib packages   ; "Objlib"
u32 tex_n; lenstr[tex_n] texture packages  ; "Textures"
u32 rml_n; lenstr[rml_n] material libs     ; "Resource\Common.rml", "Resource\Underworld0.rml", "Resource\UnderworldB.rml"
```
Package names omit `.rpk`. Most other package names are hard-coded in the exe.

### 3.6 RFI image (`.rfi`)
Sources: EMTK book and `emtk_asset/src/rfi.rs`; Structs.txt; Turkish `rfi_codec.py` ("verified by byte-exact re-encoding of all 45 compressed .rfi files"); zh-CN `exanima_zh.py`; Mod-Launcher `GameAssets.cs`.

32-byte header followed by the body:

| Off | Type | Name | Notes |
|---|---|---|---|
| 0x00 | u32 | magic | `0x1D2D3DC6` |
| 0x04 | u32 | width | |
| 0x08 | u32 | height | |
| 0x0C | u32 | frames/"single" | 0 or 1 (Turkish patch: always 1) |
| 0x10 | u32 | pixel format | "type/options"; see table below |
| 0x14 | u32 | flags | Bits in the high 16: `0x0008` (i.e. `0x00080000`) = **mipmapped** (Structs.txt). Turkish patch writes `0x20200000` by default ("uv/sampler bits"). |
| 0x18 | u32 | storage | `0x10000000` = raw, `0x50000000` = RLE (test `& 0x40000000`). At runtime `0x2000<<16` marks "RLE decoded in memory". |
| 0x1C | u32 | size | Size of the GPU-ready payload: the **decompressed** size for RLE, the compressed size for DXT, **mipmaps included** |
| 0x20 | … | body | raw/DXT payload or RLE stream |

**Pixel-format word (0x10).** Mask with `& 0x800CFFFF` per Structs.txt; the low 16 bits determine bytes per pixel for raw and RLE images.

| Value | Meaning (best consensus) | Bytes/px |
|---|---|---|
| `0x0100E608` | 32-bit **BGRA** (Mod-Launcher says BGRA; Structs.txt says "RGBA") | 4 |
| `0x0100C600` | 24-bit **BGR** | 3 |
| `0x01006208` | 16-bit grey+alpha (L8A8) | 2 |
| `0x01004200` | 8-bit L8 | 1 |
| `0x01002008` | 8-bit A8 (fonts, GUI skin, cursors). EMTK guessed BC4U, which is **wrong**; the Turkish/zh-CN/Structs sources show it is raw 1 byte per pixel | 1 |
| low16 `0x8400` | 2 bytes/px raw (Mod-Launcher) | 2 |
| `0x813BC600`, `0x823BC600` | DXT1 (BC1) per EMTK (low16 matches RGB) | block |
| `0x817BE608` | DXT5 (BC3) per EMTK | block |
| `0x813B4200`, `0x01114200`, `0x0111C600`, `0x11118400` | "BC4U" per EMTK (unverified) | block |
| `0x927B8400`, `0x827BA408`, `0x1111C600`, `0x827BE608` | "BC5U" per EMTK (unverified; `8400` = 2 channels fits BC5) | block |

**[inference]** Bits 16–23 (`0x3B`, `0x7B`, `0x11`) and the top bit `0x80000000` plausibly encode block compression and mips, and the low 16 bits a channel layout (`C6` = RGB, `E6` = RGBA, `42` = L, `20` = A, `62` = LA, `84` = RG). Verify against real files by comparing `size` with `w*h*bpp` or the BCn block size.

**Row order:** rows are stored **bottom-up** (first row = bottom), as in OpenGL. (Turkish codec, Mod-Launcher)

**Mipmaps:** if `size == mipsum(w,h) * bpp`, a full mip chain down to 1×1 follows the base level. Each level halves both dimensions, clamped to 1.

**RLE codec** (byte-oriented run/literal coding; the decoder is a few lines):
```
nchan   = bytes per pixel (1..4)
runbase = 64 if nchan == 1 else 63
loop until output size == header.size:
    c = next byte
    if c < runbase:  copy (c + 1) pixels verbatim (each nchan bytes)       ; literal 1..runbase px
    else:            read 1 pixel, emit it (c - 61) times                   ; run (runbase-61)..194 px
```
- The RLE stream covers the base level *and* the mips (`datasize` includes the mips).
- Bare Mettle's encoder emits a run whenever run length ≥ 3 (1 byte/px) or ≥ 2 (multi-byte), with runs capped at 194 and literals at `runbase`. This reproduces the original files byte-exactly.
- Source: https://github.com/Psychenoumenon/exanima-turkce-yama `rfi_codec.py`. Structs.txt has an equivalent formulation.

**Fonts:** fonts are RFI atlases. The body font `fontbase24r` is 16×16 cells of 64 px, Windows-1252 order starting at the *bottom* row, alpha only. The title font is 4×8 cells of 128 px, with channel 0 = metallic shading and the last channel = alpha/glow. (Mod-Launcher `GameAssets.cs`) Per-font **advance-width tables are not in the RFI**: they are 256-entry `int16` arrays inside `Exanima.exe`, six fonts in total (Turkish `tr_widths.py`). Some RFIs (fonts) are also embedded directly in the exe (zh-CN `embedded_rfi_blobs`).

### 3.7 RFC "Rayform Content" (models, levels, characters)
Main source: Structs.txt, written against 0.8.3.9c / 0.8.4 with `FmtVersion 0x3D23`. Reader functions are named after the engine's Delphi methods (`TRFCReader.*`).

```
u16 sanity     = 0xAFCF
u16 FmtVersion = 0x3D23 (0x3D21, 0x3D22 older)
root chunk array until EOF (never duplicate types)
```
Across all 4888 RFCs in 0.8.3.9c, models have `{0xBA00?, 0x3D000000}`, and every level/locale has `0x3DED` plus some of `0xDBC0, 0x3DE10000, 0xEC00DC00, 0xCAC00001`.

| Root chunk | Engine reader | Content |
|---|---|---|
| `0x0000BA00` | `TRFCReader.ReadMaterial` | `i32 n`, then n × chunk `0xBA01 { str128 matname; material_s }`. Skipped if a material of that name already exists (global material manager `TMaterialManager3`). |
| `0x3D000000` | `Read3DObject` | **Scene graph** `scene_t` (below) |
| `0x00003DED` | `ReadDungeonData` | **Tile map**: wallsets `str128[]` ("GEWS…"), floorsets `str128[]` ("GEFS…"), `GridExt`, `GridExt2`, `0`, `600` (tile size), then three `i32[GridExt*GridExt2]` grids: `WGrid` (packed 4-bit wall/corner components per cardinal direction n/e/s/w and ne/se/sw/nw), `WGridV`, `WGridT` (tile environment type). Row-major from the NW corner; the map is centred on the origin; 600 units per tile; −y = north. |
| `0x0000DBC0` | `ReadDBaseObjs` | Sub-chunks **in this order**: `0xDBDB` objdb (local items), `0xDBCD` string lib, `0xDBCE` placed db objects: `i32 n`, then n × chunk `0xDB01 { u32 node_id(flags); [u32 flags if 0x3D23]; i32; matrix; i32; i32; [i32 if 0x3D22]; bonelocs_s* }`. Books and notes are an embedded `0xAFBF0C02` string lib here (Turkish `tr_rfc.py`). |
| `0x3DE10000` | `ReadLibNodes` | Placed Objlib props: n × chunk `0x3DE10100 { u32 flags (0x80000 = immovable); str128 name; matrix; i32; i32; bonelocs_s* }` |
| `0xCAC00001` | `ReadCharacters3` | `i32 n`; optional chunk `0xCECDDB00` (`rdb_chars`); then n × `characterposchunk_s` (type `0xCAC0CA00`: `u32 char_id(flags); f32[3] pos; f32 yaw [-π,π]; i32 extra`; sub-chunks `0xCAC0E100` = per-bone matrices (n × 0x30 bytes), `0xCAC0EA00`, `0xCACE0F77`). `0xCAC00000` is an older version. |
| `0xEC00DC00` | ? | contains the string "EnvMap" (environment) |
| `0x00003D0B` | `ReadTerrainData` | undocumented |

**`scene_t`:** `i32 ImpNodes`, then ImpNodes × object chunks. The chunk **type selects the class**:
- `0x3D03` TMesh
- `0x3D06` TLightSource
- `0x3D08` TCamera
- `0x3D0A` TSector
- `0x3D0C` TVoxelLight
- anything else is a T3DNode (`0x3D01` = bone, `0x3DE10100` = lib object)

Common node header:
```
str128 objname; i32 nodeflags; i32 ObjFlag; i32 suppress; i32 parent (1-based, 0 = root)
matrix obj_transform (parent-relative); f32[3] vec_a; f32[3] vec_b (bounds?/click & icon fill); f32; i32
if suppress == 0: type-specific payload, then a sub-chunk array
```

**Mesh payload (`0x3D03`):**
- `[i32 GeomFlag if FmtVersion > 0x3D21]` (`0x200` = draggable).
- `i32 VrtDats` (stored as n−1). Each vertex stream is `{i32 v_flags; i32 v_type; i32 count(n−1); data}`. The four observed types are:
  - `0x00F30001`: positions, f32×3
  - `0x00F20010`: UVs, f32×2
  - `0x02B40100`: u32 per vertex, probably RGBA colour
  - `0x02A41000`: 8×i32 per vertex, probably 4 bone ids + 4 weights
- Then chunk `0x3DC0`: `i32 ignored; i32 CmpVrts(n−1); i32 SubObjs(n−1)`.
  - SubObjs are vertex ranges per material: `{i32; i32 start; i32 stop; str128 material}`.
  - `i32 VrtMaps(n−1)`: each is `{i32 vm_flags; u16[CmpVrts] (u32 if CmpVrts ≥ 0xFFFF)}`. `0x10001`/`0x1` = position index map, `0x10010`/`0x10` = UV index map, `0x2` = unknown.
- Then chunk `0x3DC1`: `i32; i32 Faces(n−1); u16/u32[3][Faces]` triangle indices into the *VrtMaps*, so position = `pos[vm_pos[tri[i]]]` and uv = `uv[vm_uv[tri[i]]]`. Then `SubObjs` × `{start, stop}` face ranges, then sub-chunks `0x3DC2` (`u8` FcFlag per face, values 6/7) and `0x3DC3` (`u32` smoothing group per face).
- **Note the "count stored minus one" convention.**

**Light (`0x3D06`):**
- `LgtFlags`, `f32[3] colour`, `f32`, `Ambient`, `BaseMult`, `AttenDist`, `AttenParam`, `ShadowSoften`.
- Spot lights (`ObjFlag & 0x300 == 0x200`) add `SpotCut`, `SpotSoft`; otherwise the light is a point light (`ObjFlag |= 0x100`).
- If `LgtFlags & 0x30000`, animation parameters follow: `AnimStr`, `AnimSpd`, `AnimEmi` (flicker).

**Voxel light (`0x3D0C`):** colour, multiplier, AttenDist. This ties into the GI system (§5.2).

**Node sub-chunks:**
- `0xEDC0C589`: n bones × k matrices, probably body-slider morph targets.
- `0x3AC1`: 16-byte records.
- `0x3D0CEC04`: path points + `PathWgt` (pathfinding).
- `0x3DD0B001`: **`TRFCReader.ReadTetraRBD`**, the physics collision body.
  - Header: matrix, f32[4], params.
  - Sub-chunks: `0xCD00` (f32×4 records, probably spheres), `0xDDB0` (records with two vec3s + 3 floats + optional vec3; capsules/primitives?), `0xDDB3`.
  - These define colliders that differ from the visual mesh.

**`bonelocs_s`:** chunk `0x3DE0EC00` holding name + matrix records, used to pose articulated objects.

**`material_s`:** 0xC8 bytes of mostly unknown fields, then texture slots of about 0x38 bytes each.
- Each slot is `{i32 flag; i32 size; …; i32 nameflags; i32; str128 filename}`.
- Slot types by flag: `0x20000`, `0x40000`, `0x80000`, `0x1000000`, `0x4000000`, `0x8000000` (texture-like), and `0x400000`, `0x100000`, `0x800000` (another layout).
- Compiled by `TMaterial.Compile`. A `.rml` package groups materials.

The newest RFC/RSG knowledge (0.9.x items, characters with roles, procedural levels) lives in Jango's RFC Editor, which is unpublished outside Discord/Drive.

### 3.8 RSQ "Rayform Sequence" (skeleton + animation)
Source: Structs.txt (0.8.3.9c).

The file is a chunk array.
- `0x3D000000` holds the skeleton: a `scene_t` of `0x3D01` bone nodes (name, parent, matrix).
- `0x00003DAB` holds the animation data:
```
i32 bones_n; i32 sequences_n; i32 movesets_n
sequences_n × { str128 name; i32; i32 }
movesets_n × { str128 name; str128 name2;
   sequences_n × { i32 frames_n; if frames_n >= 1:
       i32 flags (0, 1, 0x3, 0x1000)
       f32[frames_n]                 ; frame times?
       f32[3][frames_n]              ; root translation per frame?
       frames_n × bones_n × { f32 x4 (quaternion?), i32 } } }
```
**[inference]** The bone payload looks like a rotation quaternion plus a flag. Movesets are per weapon type/race. The game blends these hand-made keyframes with physics: walking and running use designed motions, while combat motions are generated dynamically with muscles (Coffee Diary Jan 2026). In the 64-bit Physique-matters notes, attack sequences 5..12 have a clock at `TMotileDynamics+0xA20`.

### 3.9 RFT terrain / FTB
Source: Structs.txt.
```
u32 0x3EEFAD01; i32; str128 name; f32 (20.0, vertex spacing?); i32[2] dims (blocks); i32 blocks_n
chunk array: 0xBDAD block { i32 0; i32[2] block pos; i8[4]; f32[0xF4*0xF4] heightmap (244x244);
             sub-chunk 0xBD01 { i32 mats_n; mats_n × { str128 mat; u8[0xF1*0xF1] weights (241x241) } } }
```
`.ftb` (`0x3EEFBD01`) holds terrain palettes/brushes and is undocumented. Since 0.9.5 the terrain system was reworked (displacement, more materials, holes, procedural brushes), so the RFT layout has probably changed (Steam 0.9.5 devlog).

### 3.10 PWR (thaumaturgy power trees)
Files: `pwr_mind.pwr`, `pwr_force.pwr`, `pwr_energy.pwr`, `pwr_displace.pwr`.
- Header: `u32 0xAFCE01CE; i32; i32; u8[0x24]; i32; i32 trees_n`.
- Trees: `trees_n × {flags, i32, i32, f32, f32, i32 nodes_n, u8[16][nodes_n]}`.
- Footer: chunk `0xABCD0000` ⊃ `0xABCDAD00` ⊃ `0x00006E65`, then `3×i32; i32 strings_n (+1); i32 offsets[strings_n+1]; i32 bufsize; buffer of [u32 len][text]`.
- The game reader is `TThaumTree.ReadTreeData`. Offsets are relative to the start of the text table, and the last one is the end sentinel.
- Sources: Structs.txt; Turkish `tr_pwr.py`.

### 3.11 FTY factories (procedural items)
These are the recipes for assembling procedural weapons from parts. Source: Structs.txt (`itemfactory_s`).
```
u32 0xAFCE0F00|0xAFCE0F01; i32; i32; str128; i32[2] (texture size? 512x1024, 256x256)
i32 stuff_n; stuff_n × 8 bytes
i32 areas (clamped to 8); areas × { str128 name; i32[10] }      ; material/colour areas
i32 parts_n; parts_n × {
   i32 flags (bit0: texture comes from ComponentTex.rpk)
   str128 name; f32[3] pos; f32[3] rot; f32[3] scale; f32 deform; f32[2]; f32; f32
   v0F00: taper/length/curve each { u8[4] settings; f32 variability[0,1]; f32 off_start; f32 off_end }
   v0F01: u8[0x54] (new layout)
   u8 opt_chance (0..100); u8 attach_to (1-based part index, 0 = root); u8 cmp_type; u8 0
   i32 components_n; components_n × { str128 name; i32 cflags; i32; i32; i32 (flip); u8 stat[8] (stat3 == 0 → 100) } }
i32 names_n; str128[names_n]
```
Per the 0.8.3 devlog, a weapon's components vary in haft length and thickness, spike length, curve, taper, blade angle and hole patterns, alongside material quality and condition. Since 0.9.5.2g weapon masses are explicitly defined rather than estimated from model volume (Steam devlog 2026-10-08).

### 3.12 Character record (chunk `0x00CDAC##`, current version 06)
Source: Structs.txt (combined v1–v6); memory layout `TSGCharacter` in ExanimaREP.

```
str128 Name; str128 SurName
f32[2] Physique   ; [0] fat (lean 0..1 fat), [1] muscle (buff 0..1 weak)
f32 Height        ; short 1 .. -1 tall
f32 Age           ; young 0 .. 1 old
u16[2] SkinCol    ; tan, tone
i32 HairStl       ; 0 bald, 1-12 styles
u8[4] HairCol     ; hue, darkness, roughness, -
[v>=3] u16 FaceStyle0, FaceStyle1
str128 Voice      ; "HM01".."HM09", "HF01".."HF04"
f32 VoicePitch    ; 0.9..1.07
u32 Profession    ; bytes: body flags (bit0 female, bit6 cannot be resurrected), race, hair flags (low nibble wavy; high nibble beard style 0-6), misc (bit0 secondary loadout; high nibble arena profession 2 fighter, 3 trainer, 4 merchant, 5 physician)
i32 LastLocale
held_item[2] HandEq; held_item[2] HandEq2      ; held_item = {u32 id(flags); u32 flags}
i32 Equips; held_item[Equips]                 ; worn apparel
f32 Decomp        ; zombification 0..1 (non-zero → zombie voice, no KO wake-up)
u16[2] Health     ; stamina, health
[v>=5] u16[2] SFocus   ; focus (mana): stamina, health
f32 CmbSkill (0..100, derived from learned techniques); i32 UndeadVoice
personality_s { u8 Trustfulness, Bravery, BenAch, Neuroticism }   ; stored +100, used as [-100,155]
i32 RNGSeed; i32 Experience; i32
i32 skills; skills × { i16 skill_id; i16 skill_xp; u8 abilities_n; u8[abilities_n] technique ids }  ; 5000 xp per technique
i32 inventory_n; item_s[inventory_n]          ; item_s = {u32 id; u32 flags; s16 x; s16 y} (grid position in container)
[v>=5] i32 Roles; Roles × { i32 role_id (charroles.rdb); i32 flags }
[v>=6] relations_s world; i32 Locales2; Locales2 × { i32[2]; relations_s }
[v>=5] i32 Conditions; × { u32 type (0xCECC0001 salve, f32 remaining | 0xCECC0002 elixir, i32 remaining) }
[v>=3] trailing chunks: 0x00CDEC00 loadouts (arena), 0x00CDEC01/02 thaumaturgy {[0.9+] i32 potential; i32[10] PowerBind; i32 n; i32[2][n] powers}, 0x00CDEC12 home {locale, uid}
```
- **Races:** 0 Human, 1 Skeleton, 2 Ancient, 3 Ogre, 4 Sir, 5 Fael, 6 Ghoul, 7 Alpha ghoul, 8 Gobbler, 9 Meat skeleton, 0x0F Spectre, 0x10 Golem, 0x11 Golem president, 0x12 Miniature golem. The race system was rebuilt in 0.9.5 to support "virtually unlimited" types (Coffee Diary Nov 2025).
- **Item `flags`:** `0x1` locked door, `0x10` immovable, `0x100` held in hand, `0x200`/`0x400` arena arsenal states. Potions keep their glow level in `(flags >> 16) & 0xFF`.

### 3.13 Object classes in objdb (`obj_class_s` heavychunks)
Source: Structs.txt (RDB version `0xDBAC0D08`). Each item definition is a list of these classes:

| Type | Class | Known fields |
|---|---|---|
| `0x10000000` | Thing (base, always first) | `str128 name; flags (1 singular, 2 deprecated); parent id` (inheritance) |
| `0x00100000` | Object (visual) | `str128 modelname` (Objects.rpk); `name_id`, `desc_id` (index into objstrings; `0x80000000` = local); bytes including `weapon_hold_type` (0 one-handed, 1 two-handed, 2 polearm, else fists) and `weight` |
| `0x00040000` | Door | 5×i32 |
| `0x00020000` | Container | `i16 w, h; i32 key?; i16 capacity, max_size; …; item_s[]` contents |
| `0x00010000` | Operative | keys, switches, potions: 4×i32 + `str128` |
| `0x00008000` | Shield | 20×i32 unknown |
| `0x00004000` | Weapon | 11×i32 unknown (0x34 bytes) |
| `0x00001000` | **Apparel** (0x22C bytes) | `str128 ModelName` (Characters.rpk), coverage table, defence bytes, materials (see below) |
| `0x00000200` | DropTable | `id, chance%` records |
| `0x00000100` | Torch | |
| `0x8000AC07` | Trigger | 5×i32 + script code `lenstr` |
| `0x80000B03` | Map | `str128 mapname` |

**Apparel record in detail:**
- **Coverage:** forty 8-byte `coverage_s` records. Each byte is a bitfield of **4 layers**; all 4 bits set means the body part is hidden.
  - The first 20 records are "slot use".
  - The next 20 are "slot covering" for: pelvis, thigh_r, calf_r, foot_r, thigh_l, calf_l, foot_l, abdomen, chest, hair, shoulder_r, upperarm_r, lowerarm_r, shoulder_l, upperarm_l, lowerarm_l, hand_r, hand_l, plus 2 unknown.
- **Defence:** `u8 defence_impact, slash, crush, pierce`.
- **Other stats:** `u8 coverage, encumbrance, sound` (1 cloth, 2 leather, 3 mail, 4 plate), `bulk`, `type`, `grade`, `traits`, `rank` (<32 Inept … ≥160 Master), `i32 rng_seed`, `u8 quality, wear, dirt, variance`.
- **Materials:** eight `material_properties_s` records {layer, part, element, design, material (0–19: cloth×4, velvet, padded, quilted, leather, silk, leather 2/3, mail, metal, shiny metal, lacquer, shiny lacquer, broken, wood×2, dull metal), trait, colour, theme}.
- **Extras:** `i32 ability` (1 night vision, 2 stamina regen, 3 healing, 4 golem suit), crystal socket and charge, and `i32 equipment_set` (id of the central torso piece; drives set encumbrance bonuses).

### 3.14 Save games `.rsg` / `.rcp`
Source: Structs.txt (`TLargeDataManager.*`).

```
u32 0xDA7AEA02; i32 sections_n
sections_n × { i32 offset; i32 size; str128 name }   ; NOT sorted; sections padded to 0x2000
```
- **Section types:**
  - The first entry is always 12 bytes of zeros.
  - `#playerdata`
  - `#worldchrdb` (chunk with `rdb_chars`: the player and followers)
  - `#worldobjdb` (items carried between levels)
  - One section per visited locale (e.g. `exanima01`), each holding **a full modified copy of the level RFC**. This gives "perfect persistence": every object position and NPC memory is kept (Coffee Diary Apr 2025).
- **Write strategy:** sections are rewritten in place. A section that grows is relocated to the end, and old bytes are not cleared.
- **`#playerdata`:**
  - Signature: story `0x6F5AEA13` (0.9+), arena/"career" `0xE8AAEA10`.
  - Then `str128 locale; f32[3] camera pos; f32 zoom; u8 copper; u8 silver; u16 gold` (12 copper = 1 silver, 12 silver = 1 gold); 3×i32; the player `characterposchunk`.
  - A chunk array follows:
    - `0xA0100000`: arsenal, 5 categories × 5 boxes.
    - `0xA0400000`: employees.
    - `0xA0600000`: tournament state: 6 events, each with match type (4 Fray, 7 Elimination, 16 Conquest, 17 Champion, 18 Melee), tier and a weapon restriction (46 weapon types listed in Structs.txt); plus 19 rival company names.
    - `0x00010000`: hireable NPCs.
    - `0x10`: journal entries.
    - `0x00100000`: global XP.
    - `0x100`: books read + explored-tiles bitmaps per locale.
    - `0xA0DC0000`: company record (emblem, match stats, money earned and spent).
    - `0xA0D10000`.
- **Checkpoints:** `.rcp` files have the same format. Renaming the latest `.rsg` to `.rcp` makes it the death-reload point (https://github.com/VitalyArtemiev/exanima-save-to-checkpoint).
- **Size:** late-game saves reach about 26–35 MB (Xaver README; ExanimaTimeThaumaturgy README).

### 3.15 `Exanima.set` (progress + keybindings)
A heavychunk array. Source: Structs.txt; Drinky README.
- **Root characters:** `0x6431BC01` career, `02` the unknown, `03` knight, `04` proctor, `05` villager, each containing `0x6431CC01` with a character chunk (character-creator presets).
- **Other root entries:**
  - `0x6431AC02`: unlock bits (knight, proctor, villager).
  - `0x6431AC04`: global XP (5000 per level, max 25000).
  - `0x6431AC01`: expert practice unlocked.
  - `0x6431EA02`: arena tutorial-popup flags.
  - `0x6440A401` ⊃ `0x6440CB01`: key bindings as i32s.
  - `0x6431AC03`, `0x6431AC13`: unknown.

### 3.16 `Exanima.ini` (plain `Key = Value`, no sections, case-insensitive)
Sources: Structs.txt; Mod-Launcher `GameSettings.cs`; 0.8.3 devlog.
- **Settings menu keys:** `Fullscreen`, `VerticalSync`, `ShowFPS`, `TextureQuality` (2–5), `TextureDetail` (2–3), `ShadingQuality` (3–6), `ShadowQuality` (1–4), `ShadowDetail` (4–5), `HWMultisample` (0/2/4/8, forced off with supersampling), `Supersample` (0/1/3/4), `Volume` (0–150), `Music`, `ShowGameTips`, `Checkpoints`, `CamFollow`, `InputMode`, `AimSpeed`, `PerfIndex`, `FirstRun`.
- **Hidden keys:** `LinearRGB`, `SoftwareAudio`, `NoProcMips`, `ShadowFmt`, `DynamicVBO`, `ForceVBO`, `SwapCopy`, `GILowPrecision`, `GUISize`, `NoTopMost`, `FPSLimit`, `ScreenWidth/Height`, `WindowWidth/Height`, `TouchScreen`, `StereoMode`, `KeyDef`, `CursorSize`, `CursorInt`/`CursorCom` (RRGGBBAA cursor colours).
- **[inference]** `VBO`, `SwapCopy` and `GILowPrecision` are OpenGL/GI-specific switches.

### 3.17 Data embedded in `Exanima.exe`
- **Text:** much of the UI text is stored as Pascal AnsiStrings with an 8-byte length prefix in the 64-bit exe. Translators patch the exe text in place, using neighbouring strings as anchors. (zh-CN `patch_exe_string`; Turkish `tr_patch2.py`)
- **Fonts:** 6 font advance-width tables (`int16[256]` each) and some RFI font atlases are embedded in the exe. (Turkish `tr_widths.py`; zh-CN)
- **Known exe hashes:** 0.9.5.2 exe SHA-256 `97a83509…6cba6a`, `Resource.rpk` `e32dbf99…b39b751a` (zh-CN `docs/EXTRACTING.md`). A later Steam build is `af9033c7…6b70` (Physique-matters README).

---

## 4. Sound
- Audio is plain RIFF WAVE inside `Sound.rpk` (EMTK, ERE).
- Since 0.9.5.2e there are randomised 3D positional ambient sounds and soundscapes (Steam devlog 2026-10-03).
- Items carry a sound class byte (cloth/leather/mail/plate) (§3.13).
- Impact sounds depend on physics; "haft types with different behaviour and sounds" arrived in 0.8.4.
- A `SoftwareAudio` ini switch exists.
- I found no public information on the audio API (DirectSound/XAudio/OpenAL). **[open question]**

---

## 5. Engine technology facts

### 5.1 Language, toolchain, binary
- **Language:** Delphi (Object Pascal) with performance-critical hand-written x86/x64 assembly. (EMTK Book engine details; Steam 0.9.5 devlog and Coffee Diary Jan 2024)
- **Engine name:** "Rayform", Bare Mettle's proprietary engine. File extensions use "R…" for Rayform. (EMTK Book; Structs.txt)
- **Classes:** Delphi-style `T…` names with standard Delphi VMT layout (size, negative size, parent VMT pointer, method pointers). Dynamic arrays keep refcount and length at negative offsets. (EMTK Book VMT page; ExanimaREP)
- **Calling convention:** 32-bit builds used the Borland *register* convention (EAX, EDX, ECX, then stack, callee cleans up) (EMTK Book). The 64-bit build follows the Win64 ABI; EMTK hooks take `extern "C"` on x64 (EMTK Mods `godmode`).
- **Architecture history:**
  - The game was **32-bit until 0.9.5**, with a 3 GB RAM limit.
  - The whole engine, game and tools were ported to **64-bit** (Coffee Diary Jan 2024; shipped in 0.9.5 on 2026-05-20), with rewritten assembly and new low-level optimisations.
  - The 64-bit exe has image base `0x100000000` and **no ASLR** (Physique-matters README).
  - **[inference]** That default image base is Free Pascal's for Win64; Delphi's default is `0x400000`. The 64-bit build may use FPC or a customised Delphi, which is unresolved. The function prologue `53 56 48 8D 64 24 D8` (push rbx; push rsi; lea rsp, [rsp-0x28]) is also FPC-like.
- **Team:** a single engine/game programmer (Madoc Evans) for most of development; a second programmer (Jimmini) joined around 2025 for tools. (0.9.5 devlog; Coffee Diary Nov 2025)
- **Modding hooks:** `winmm.dll` proxy loading is the de-facto mod loader (Horse4Horse ModLoader, zh-CN patch, Mod-Launcher). EMTK instead injects with Detours.

### 5.2 Rendering
- **API: OpenGL.**
  - "The game is OpenGL based rather than DirectX" (0.5.2 devlog).
  - Minimum spec says "OpenGL 4.6+ compatible" (Steam store).
  - Mods hook `opengl32.dll!wglSwapBuffers`, and shaders use `gl_Position = ObjMat * vec4(vPos,1)`. Cruor injects `#version 150` shaders, but the game's own GLSL version is unknown. (https://github.com/Monochrome94/Cruor)
  - The IDA header has `TGLGeometry`, `TShaderBuilder`, `TShaderManager`, `TFrameBuffer`, `TPostProcessor`, `TAbstractShadowMap`, `TGlobalAOMap`, `TCharTexGenerator`, `TRenderer` (ExanimaREP).
- **Renderer history:** a completely new renderer arrived in 0.8.3 (Dec 2021). GPU driver problems drove the change, and it "triple[d] framerates" in CPU-bound cases. It is **still a forward renderer**, but without the old hard light-count limit. (0.8.3 devlog)
- **Forward rendering and dynamic shaders:** the engine leverages forward rendering for material versatility and many specific optimisations. It needs "a massive number of specific shaders that are generated dynamically". Shader pre-compilation tracking was added in April 2026. Cruor counts about 500 scene shaders sharing a UBO. (Coffee Diary Apr 2026; Cruor)
- **Global UBO "Global" (std140)**, byte offsets as observed by Cruor in 0.9.5.x:
  - `ViewPos@0`, `ViewSiz@16`, `ViewPrj@24`, `CamMat@32`, `SdwMat@96`
  - **`VXGIScl@160`**: **[inference]** a voxel-based GI scale
  - `Ambient@176`, `TimeDlt@192`, `ViewRcp@200`, `FogColr@208`, `FogPos@224`, `BufrPos@240`, `Wetness@248`
  - Per-object uniforms `ObjMat[0]` (object→world) and `ObjMat[1]` (object→screen); matrices are column-major. (Cruor `lib.rs`)
- **Global illumination:**
  - A fully dynamic, fast GI system was introduced with the 0.8.3 renderer. It lets emissive objects cast glow and soft shadows without light sources. (0.8.3 devlog)
  - Improved in Feb 2025 to brighten shadowed areas (Coffee Diary Feb 2025).
  - The "ultra" shading setting adds specular GI (0.9 forum post).
  - RFC files contain **TVoxelLight** nodes (`0x3D0C`) and the IDA header has `TVoxelLight`/`TAVoxelLight`, consistent with VXGI.
- **Shadows:** "many dynamic shadow casting lights" with soft shadows (0.9.5 devlog). Lights have `ShadowSoften`, spot cut-off and softness, and flicker animation params (§3.7). Since 2025 there are **displacement-map shadows from every light**, a custom technique (Coffee Diary Feb 2025; 0.9.5).
- **Materials:** all items use **procedural materials** for fabrics, leathers, buttons, paints, rust, dirt and wear, driven by item quality, condition and traits (0.8.3 devlog; Jango's procedural materials video https://www.youtube.com/watch?v=0JbxBC3D8D8). Procedural materials became a general engine feature in Jan 2026. Character materials include subsurface scattering (0.7.2.1 devlog).
- **Anti-aliasing / "supersampling":** image quality is "our engine's signature". The supersampling draws several jittered ("nudged") copies of the scene side by side in one framebuffer and then combines them (Coffee Diary Apr 2026; Cruor notes on "copies" and the combine step). The ini allows MSAA or supersampling, but not both.
- **Terrain and vegetation:** a unified terrain + modular-dungeon system since 0.6.5.2. It has time of day and weather (rain, wetness), 3D grass/undergrowth with translucency, dense vegetation lit by many local lights, and procedural ground painting (0.6.5.2, Coffee Diaries 2024–25, 0.9.5).
- **Asset pipeline:** most modelling is done in third-party tools; textures, materials, level design, rigging, skinning, animation, roles and dialogue are made in in-house tools. A cloth simulator is used to author clothing, and item icons are rendered dynamically. (0.9.5 devlog; Coffee Diaries Nov 2025 and Jan 2025)
- **Apparel rendering:** an arbitrary layered "slotless" apparel system. A single garment can be assembled from hundreds of meshes depending on what it is worn with, and body or apparel parts hidden under other layers are culled. The coverage layer bits in objdb drive this (§3.13; Coffee Diary Mar 2025; 0.9.5).
- **Streaming:** a content-streaming system arrived in 0.9.5, so NPCs and procedural items generate without stutter (0.9.5 devlog; Coffee Diary Apr 2026).

### 5.3 Physics and animation
- **Custom physics engine** "designed to handle complex interactions between intricate objects while remaining stable … on low end systems".
  - It has **Continuous Collision Detection**.
  - It was rewritten in 2024 to solve thin fast objects (weapon hafts in parries and grapples) and stacking.
  - Objects deform rather than explode under impossible pressure.
  - It "will never miss or skip a collision".
  - Sources: 0.9.5 devlog; Coffee Diaries May 2024 and Oct 2026.
- **Rigid bodies:**
  - **[inference from ExanimaREP]** The class `TTetraRBD` has fields `PrtMass[4]`, `CrPoint`/`PrPoint`/`OlPoint` (current, previous and old positions of 4 points), `TetraM` (4×4), `PrtVel`, `ColSp` (an array of collision spheres), `Link` (an array of `TARBLink` joints), `Friction`, `MaxVel`, and `RootRB`/`ParentRB`/`LinkRBD`.
  - This strongly suggests rigid bodies are simulated as **4 mass points (a tetrahedron) with Verlet/position-based integration**, using sphere-based collision volumes and link constraints for articulated bodies.
  - RFC chunk `0x3DD0B001` (`ReadTetraRBD`) stores per-model collision primitives.
  - `TSimSector` and `TCollisionSolver` suggest a spatially gridded simulation (`GrdLoc`, `GrdPrv`/`GrdNxt`).
  - Bodies at rest go idle (`RestCount`).
- **Cloth and soft bodies:** a new system in 2024 handles cloth, hair, vegetation, nets, sacks, paper and "fleshy growths". It is used for environment cloth with wind; clothing simulation is planned for "human 2.0" after 1.0 (Coffee Diaries Jan and Mar 2024; 0.9.5).
- **Characters:**
  - "Characters … make thousands of dynamic adjustments to muscle tensions per second and generate movements on the fly based on forces, points of balance, analogue inputs, points of contact and collisions" (Coffee Diary Oct 2026).
  - All motion comes from physical forces and **virtual muscles** (0.5.2 devlog).
  - Muscle strength is scaled per body part (Coffee Diary Jan 2026).
  - Joints can over-extend or dislocate under extreme stress (Coffee Diary May 2024).
  - An "AI" controls muscle tension (0.8).
- **Locomotion vs combat:**
  - Walking and running use **hand-designed motions** plus "invisible forces/hacks" to stay upright, with muscle control making precise adjustments. The character scans the ground ahead to plan steps on stairs and slopes.
  - **Combat motion is fully physics-driven and generated on the fly.**
  - A **dynamic balancer** detects loss of balance, tripping and slipping, and works even mid-air (0.7.0).
  - Sources: Coffee Diaries Jan and Feb 2026; 0.7.0.
- **Movement control:** free-direction combat movement without a fixed step pattern since 0.8.4. Dashes, steps and crouching are supported (0.8.4; wiki How-to-play).
- **Damage from physics:**
  - Damage depends on momentum, impact angle, which part of the weapon hits, impact area and the armour on that exact spot (wiki How-to-play; quick-start PDF; Bare Mettle games page).
  - General environment and fall damage since 0.9.
  - Hit damage is computed in a contact/hit routine. The 64-bit build applies three damage values (**[inference]** most likely impact/slash/pierce-type channels) before applying them (Physique-matters `cave.py`: three floats at `rbp-0x98/-0x70/-0x88`).
- **Determinism:** "Exanima runs entirely on exact physics and collisions". Multiplayer uses a custom peer-to-peer approach with prediction and dynamic time sync, tested at about 100 ms latency (Coffee Diary Oct 2026). An unofficial co-op mod with a host-authoritative design exists (https://www.youtube.com/watch?v=EApAJID0Ubc).

### 5.4 Engine class inventory (from ExanimaREP IDA header, 0.9.1.6)
Selected names, useful for mapping subsystems. The source is https://github.com/Horse4Horse/ExanimaREP `Exanima.exe_LTS_.h`, and many entries are marked `_UNVERIFIED`.
- **Scene:** `T3DNode`, `TMesh`, `TCamera`, `TLightSource`, `TVoxelLight`, `TSector`, `TScene`, `TSceneEnvironment`, `TSkyEnvParams`, `TAbstractSkyGenerator`, `TRainEffect`, `TGridEnvManager`.
- **Geometry:** `TVertexData` {MapFlg, DatFlg, VMap, Entries, Data}, `TVertexMapping`, `TFace`, `TASubObj`, `TMeshModifier`, `TTendril`.
- **Render:** `TRenderer`, `TAbstractRenderer`, `TGLGeometry`, `TShaderBuilder`, `TShaderManager`, `TFrameBuffer`, `TPostProcessor`, `TAbstractShadowMap`, `TGlobalAOMap`, `TTexture`, `TTextureSet`, `TRFImage`, `TMaterial`, `TMaterialRefList`, `TCharTexGenerator`, `TItemRenderer`, `TIconManager`, `TTextRenderer`.
- **Physics:** `TPhysicsControlller` (sic), `TTetraRBD`, `TRBLink`, `TConvexVol`, `TACHPoly`, `TAColSphere`, `TSimSector`, `TCollisionSolver`, `TSimulation`, `TPhysPrioHandle`.
- **Game:** `TSGCharacter`, `TCharacterSkills`, `TSkillDat` {SklLvl, TecDat0..5}, `TInventory`, `TSGItemContainer`, `TContainerItem`, `THandEquipSet`, `TGameItemRef`, `TObjectDB`, `TProcItemPars`, `TRandomItemPars`, `TournamentEvent`, `TournamentCharacter`, `TSaveGameManager`, `TPathFinder`, `TAbstractPathFinder`.
- **Data:** `TProjectManager`, `TDataLibrary`, `TAPackageLibrary`, `TAMemBlockLibrary`, `TDataStream`, `TWorkQueueItem`, `TThread`.
- **GUI:** `TGUIForm`, `TGUIControl`, `TBaseGUISkin`, `TGUITableView`, `TCharEditForm`, `TPhysiquePicker`, `TTonePicker`.
- **From Structs.txt and mods:** `TRFCReader` (`ReadRFC`, `ReadMaterial`, `ReadMesh`, `ReadLight`, `ReadVoxLight`, `ReadTetraRBD`, `ReadDungeonData`, `ReadDBaseObjs`, `ReadDBaseNodes`, `ReadLibNodes`, `ReadCharacters3`, `ReadCharacter3`), `TMaterialManager3`, `TStringLibrary`, `TObjectDB.LoadMem`, `TCharacterDB.LoadMem`, `TLargeDataManager` (saves), `TThaumTree.ReadTreeData`, `TMotileDynamics` (character physics body), `TWeaponBlood`.

### 5.5 Runtime memory facts (64-bit, 0.9.5.x; useful for behaviour reference, not for the reimplementation)
- **Player body:** the player's `TMotileDynamics` global sits at `0x1004F0830` in one 0.9.5 build. From there:
  - `+0x10A0` points to `TSGCharacter`.
  - Physique x/y (the char-creator marker, 0..1) sits at `TSGCharacter+0x2C/+0x30`.
  - The current motion sequence index is at `+0x9B8` (attacks = 5..12).
  - The sequence clock is at `+0xA20` and advances by a per-creature step at `+0xDC8` each physics tick.
  - The hit routine is at `0x100159A60`.
  - Source: https://github.com/Justin52361/Exanima-Physique-matters
- **Character state enum (older build):** 0 moving, 3 combat stance, 4 attack stance, 5–7 right attacks 1–3, 8–10 left attacks 1–3, 0xB alternative attack (thrust), 0xC heavy attack (overhead), 0xE/0xF getting up from stomach/back, 0x13 resting, 0x15–0x18 blocks, 0x1F swapping weapons, plus magic states. (https://github.com/jereremy/ExanimaHack `gd_player.h`)
- **Health floats:** yellow (stamina-type) and red (health) are each in **0.0–0.25**; blue (focus) is in 0–1 with a cap. They live at `+0xE80/+0xE6C/+0xE84` in 0.9.5.2c. (https://github.com/englishpancake/exanimaHeal `trainer.c`)
- **Possible bar scale [inference]:** the trainer's 0.25 maximum may mean one bar unit is 0.25 internally. The u16 `Health[2]` in the save file (stamina, health) is the persisted form.

---

## 6. Gameplay systems

### 6.1 Modes
- **Story mode.** An isometric dungeon crawler in the underworld, a prelude to Sui Generis set about 20 years before it.
  - Levels: Level 1 → 2 → Catacombs → 3 → 4 Archives → 5 Crossroads (+ sewers) → 6 Golem Forge → 7 Market (+ sewers) → 8 Gentry → Gardens (+ crypts). The level ids are listed in §3.2.
  - Mechanics: checkpoints; a permanent portal save point; unconsciousness/knock-out rather than instant death (two-tier health, since 0.6.2); companions (Derrin); a narrator (0.9); a journal; lootable procedurally drawn maps.
  - Saving is deliberately limited. 0.9.5.2 added item-gated late-game save points that trade off against other powers.
  - Global XP carries over between characters (multiple characters since 0.6).
  - Sources: Steam devlogs 0.6, 0.6.2.0, 0.7.0, 0.7.2, 0.9, 0.9.5, 0.9.5.2; Sui Generis wiki "Exanima"; Bare Mettle games page.
- **Arena mode** (persistent "career"; 0.6.1.0 beta, overhauled in 0.8.3).
  - **Company:** you play a manager (never fight with them: if the manager dies, the save is lost). You hire and recruit fighters and hirelings, and can hire a **merchant**, a **physician** (faster recovery, can save fighters from death) and a **trainer** (teaches skills).
  - **Economy and calendar:** a shop and an arsenal (5 categories × 5 boxes). Money is talins: copper, silver and gold at 12 to 1. There is daily staff upkeep, a day system with up to 3 matches a day, and a company emblem.
  - **Ranks:** Inept, Aspirant, Novice, Adept, Expert, Master. A fighter ranks up every 2 techniques trained. Each rank has its own equipment loadout and a points limit.
  - **Match types:** Fray, Elimination (5 consecutive fights with no red-bar healing), Conquest, Champion, Melee, Doubles, Skirmish, 1v2/1v3, pugilism, beast/ogre matches. Matches split into standard (configurable) and daily specials.
  - **Tournaments:** weekly, 6 events; win at least 3 without a tie. A season is 4 tournaments, and winning all 4 earns ornate armour. Opponents are 19 rival companies.
  - **XP:** based on hits, parries, blocks and damage dealt; shared among a team; 50% XP on a loss.
  - Sources: Steam devlogs 0.6.1.0, 0.6.1.6, 0.6.1.7, 0.6.4.3, 0.8.3; Structs.txt `#playerdata`; wiki How-to-play.
- **Practice mode.** The legacy arena kept as "practice" (0.6.1.6): novice practice, then expert practice once novice is beaten (`Exanima.set` flag). Game-mode enum: 0 menu, 1 novice practice, 2 expert practice, 4 story, 8 arena (gnalvesteffer Cheat Engine table).

### 6.2 Character creation and attributes
- **Body:** physique on a 2D pad (fat/lean × muscular/weak), height, age, skin tan and tone, hair style (12) and colour, wavy or straight hair, 6 beard styles, voice (9 male, 4 female) and pitch, sex. (Structs.txt; 0.8.3 devlog)
- **Physique is cosmetic in vanilla.** The Physique-matters mod changes that. The physics *is* affected by body mass and proportions, though: muscles are scaled per body part. (https://github.com/Justin52361/Exanima-Physique-matters; Coffee Diary Jan 2026)
- **No innate stats:** "human opponents never possess innate statistical advantages". NPCs follow the same rules and simulate human reaction times (quick-start PDF).
- **Personality (NPCs):** trustfulness, bravery, benevolence/achievement, neuroticism (Structs.txt).
- **Thaumaturgy:** an innate thaumaturgy domain is chosen at creation (wiki Thaumaturgy).

### 6.3 Skills and techniques
- **Structure:** 6 skills × 8 techniques. Technique XP is 5000 each. Learned techniques are capped at about 15 out of 56 (planned at 7 skills; 0.5.3 devlog), and techniques can be forgotten. (Structs.txt; wiki Skill)
- **Close Combat:** Remise, Feint, Fend, Riposte, Dual Wield (needs Remise + Riposte); Draw, Impel, Brace are unimplemented.
- **Ranged:** none implemented.
- **Armour:** Manoeuvring in Armour I/II/III (reduce encumbrance effects); Deflection (effectively increases armour coverage and protects gaps); Security (keep attacking through weak blows that armour mitigated; proportional to how much was mitigated).
- **Shields:** Counter, Synchrony, Ward (need Counter), Persist, Endure, Mobility.
- **Insight:** Inversion, Expansion, Convergence, Union, Coherence, Iteration, Revelation (unlocks a Mind power from Corlian's journal on level 8), Amalgamation.
- **Concentration:** Sustain, Continuity I–III, Abstraction, Clarity, Resolve, Volition.
- **AI skill:** an NPC's fighting skill is set by its number of trained techniques (0.6.1.6), and companions improve with experience (0.9.5).
- **XP sources:** exploration, reading books, inspection, combat (wiki Skill).

### 6.4 Thaumaturgy (magic)
- **Domains:** six forms in the lore: Mind, Force, Energy, Displacement, Light, Body. Implemented:
  - **Mind** (0.8; 16 Insight/Concentration techniques; influence and control, necromancy)
  - **Force** (0.9; 11 physics powers)
  - **Energy/Displacement** (0.9.5; mostly NPC powers, learnable by absorbing defeated "shapers")
- **Resource:** powers consume **focus** (the blue bar).
- **Cast types:** sustained, immediate (with optional charge-up) and continuous toggles (one at a time without Continuity techniques).
- **Customisation:** powers can be modified ("tweaked") by Insight techniques; for example, Expansion turns Enrage into Mass Enrage. "Potential" scales power strength (0.9).
- **Controls:** P opens the power tree; C + 1–0 casts.
- **Sources:** 0.8, 0.9, 0.9.5 devlogs; wiki Thaumaturgy and Insight; Structs.txt (`PowerBind[10]`, `potential`).

### 6.5 Equipment, armour layering, damage
- **Weapon stats** (all "qualities", not damage numbers):
  - **Weight:** the real mass since 0.9.5.2g, relative within each weapon class.
  - **Balance:** responsiveness.
  - **Impact:** force transfer; mostly non-lethal (stamina) damage, but large impacts injure.
  - **Slash:** sharpness; effective at low force; easily stopped by armour.
  - **Pierce:** swung points; exploits gaps.
  - **Crush:** bends armour; needs force.
  - **Thrust:** point damage via thrusts, plus impact.
  - Sources: quick-start PDF; 0.9.5.2g devlog.
  - 0.8.4 split slashing into **hewing vs cutting** and reworked mitigation.
- **Armour stats:** impact, slash, pierce and crush ratings (comparable only between armours), **coverage** (how well the piece covers its body part), encumbrance, plus bulk, sound class, quality, wear, dirt and rank in data (§3.13). (quick-start PDF; Structs.txt)
- **Locational protection:** armour protects only what it covers. A breastplate gives zero defence to the head, and a stab to an exposed face does full damage. (Bare Mettle games page; wiki How-to-play)
- **Layering:**
  - The data has **4 layer bits per body region**.
  - The conventional order, inner to outer: clothing → padding (gambesons, arming caps) → mail → rigid/plate (cuirass, brigandine, plate), with full suits spanning several regions.
  - The system is "slotless": anything can be worn in any combination as long as it physically fits.
  - **Layering is very effective for impact mitigation.**
  - **Sets:** pieces connected to the central torso piece of the same set reduce encumbrance.
  - Since 0.8, chain under full plate is no longer worthwhile.
  - Sources: Structs.txt; 0.9.5 and 0.8 devlogs; quick-start PDF; wiki How-to-play; https://github.com/Xiao-Houzi/ExanimaTools `project-management/ArmourLayeringReference.md` (community summary).
- **Body regions** (from coverage data): pelvis, abdomen, chest, hair/head, and left/right shoulder, upper arm, lower arm, hand, thigh, calf, foot (Structs.txt).
- **Health model:** two tiers.
  - **Yellow** (stamina/temporary): regenerates exponentially out of combat mode.
  - **Red** (health/permanent): heals only with salves or the elixir in story mode, and overnight in arena.
  - **Blue** is focus.
  - Losing yellow means a KO; losing red means death. Defending is impossible with depleted stamina (0.7.0).
  - Sources: wiki How-to-play; 0.6.2.0 devlog; exanimaHeal.
- **Encumbrance:** computed from equipped items and Manoeuvring skills. It slows movement, swings and reactions. Heavy armour also changes physics independently of the encumbrance stat. (wiki How-to-play; 0.6 devlog)
- **Item quality, grade, condition, traits:**
  - About 17 quality descriptors (Extremely Crude → Exceptionally Well Made) and about 17 condition descriptors (Terrible → Flawless).
  - Traits: heavy, light, thick, tough, reinforced, hardened.
  - Condition (wear/rust) does not decay over time and can be improved with "spheres" in story mode. Quality affects trade value and appearance; item captions are coloured by quality since 0.9.
  - Sources: 0.8.3 devlog; wiki Armour and Weapons; 0.6.5.4 devlog.
- **Procedural items:** weapons have used procedural factories since 0.8.3 ("billions of combinations"); apparel uses procedural materials; procedural shields are in progress (Coffee Diary Aug 2026). Loot tables weight quality, condition and component choice by level.
- **Crafting:** none yet. Dyes, fixatives and alchemy-like products are being designed (Coffee Diary Sep 2026). Secondary skills (alchemy, smithing, cartography, music) are planned (wiki Skill).
- **Inventory:**
  - Containers have a grid size and capacity, and items have grid positions (`item_s.x/y`).
  - Portable containers exist (0.6.5.2), and inventory weight has an unknown limit.
  - Interaction: double-click to use or open; click shows a description window, which can be pinned.
  - Two hand loadouts can be swapped; held items stay in the inventory, flagged as "in hand".
  - Sources: Structs.txt; quick-start PDF; wiki.

### 6.6 Controls (PC defaults)
- **Exploration mode** (yellow cursor):
  - Hold RMB to move toward the cursor; cursor distance sets walk vs run; Run/Sprint (Shift) runs faster.
  - WASD uses tank-style controls.
  - LMB drags objects (WASD rotates while dragging; doors are dragged open); double-click uses or opens; T talks; I opens the inventory.
  - (quick-start PDF; wiki How-to-play)
- **Combat mode** (Tab toggles; red cursor):
  - The character always faces the cursor; WASD steps relative to it.
  - Dashes: the "Hold" scheme dashes on a brief hold, the "Double Tap" scheme dashes on a double tap.
  - **Hold LMB** swings right-to-left, then follows with the opposite swing if Remise is learned.
  - Starting the click with the cursor over the character's **left shoulder** gives a left-to-right swing.
  - **Double-click and hold** gives an **overhead**.
  - **Alt + LMB** thrusts, aimed continuously at the body point nearest the cursor. A "blurry stickman" projection picks the height (0.6.1.6 devlog).
  - Releasing LMB cancels or feints; **blocking and parrying are automatic** when not attacking. You still have to position and turn to intercept blows.
  - Winding the cursor against the swing direction and stepping into the swing adds power. Crouching lowers swings (under shields).
  - The cursor's aim plane is fixed at about hip height.
  - Unarmed: punches, hooks and uppercuts from cursor motion.
  - (quick-start PDF; wiki How-to-play; 0.6 devlog)
- **Camera:** mouse wheel zooms; "Orbit Camera" + drag orbits; "Align Camera"; optional `CamFollow`. Objects that block the view fade out (0.8.4).
- **Controller:** full controller and Steam Deck support since 0.8.4, with dedicated move and aim sticks.

### 6.7 AI
- **No special rules for NPCs:** they are treated exactly like players, based only on observable behaviour.
- **Emotions:** NPCs are driven by emotions such as distracted, curious, agitated, concerned, angry, scared and surprised, plus memory and relationships, through a **role system**:
  - Roles are character-defining, situational or relational, and are authored in visual tools with a custom scripting language (`charroles.rdb`).
  - They drive dialogue with branching topics, conditions and actions.
  - NPCs search and loot on their own (0.9.5).
- **Combat AI:**
  - Rebuilt in 0.7.0 and 0.8.3, and redesigned again in 0.9.5.
  - Skill levels vary, and the AI predicts tactics, avoids friendly fire, and plans power use.
  - Combat stance makes neutral NPCs anxious.
- **Pathfinding:** "polygon soup" 3D dynamic pathfinding (0.9) with no navmesh. Characters can walk on dynamic objects, and multi-layer, clearance and fall-hazard handling were added in 2025–26.
- **Sources:** 0.6, 0.7.0, 0.9, 0.9.5 devlogs; Coffee Diaries Mar 2026, Jun 2025, Jul–Sep 2025; wiki How-to-play.

---

## 7. Open questions / gaps
1. **Current RFC (0.9.5) deltas.**
   - Structs.txt targets `FmtVersion 0x3D23` (0.8.3.9c–0.9.1). After the 0.9.5 apparel, terrain, streaming and race overhauls, new chunk types or versions are likely: `0x3D0CEC05` is already noted as "UNFINISHED".
   - Jango's RFC Editor (Discord/Drive) is the most current, unpublished reference. Ask permission or contact on the Council of Conservers Discord.
2. **Mesh skinning stream `0x02A41000`:** is it bone ids as i32 plus weights as f32 (as guessed), and is there a bone palette per SubObj? How do character meshes in `Characters.rpk` bind to the RSQ skeleton (bone name matching)?
3. **RSQ semantics:** frame times vs. root motion; whether the per-bone `f32×4 + i32` is a quaternion + flags; moveset ↔ weapon/race mapping; how physics muscles consume the keyframes.
4. **RFI pixel-format bitfield:** the exact meaning of bits 16–31 and of the BC4/BC5 codes; whether DXT payloads are stored top-down or bottom-up; cube maps (`EnvMap`).
5. **`material_s`:** most of the 0xC8-byte header is unknown (shader flags, colours, specular, SSS), and the texture-slot flag → map-type mapping (diffuse/normal/spec/height/emissive) is undocumented.
6. **`TetraRBD` collision data (`0x3DD0B001`):** primitive types (spheres `0xCD00`? capsules or convex `0xDDB0`/`0xDDB3`?), mass properties, joint/link definitions for characters (muscle definitions are probably elsewhere, perhaps in the race data).
7. **Unknown containers and magics:** `.flb`, `.ftb`, `.rui`, `.rab`, `.rpp`, `.det`, `.rcd`, `.rcr`, and the IDK1–7 magics.
8. **Weapon and shield objdb classes** (`0x4000`, `0x8000`): field meanings. FTY v`0x0F01` (0.9) part layout is `u8[0x54]`, not decoded.
9. **Audio:** which API (OpenAL? DirectSound? custom mixer), the reverb/3D model, and how impact sounds are chosen (material pairs?).
10. **Character physics model:** the number of bodies and joints in a human, the muscle model, the balancer and the hit-damage formula are not public. A faithful reimplementation will need either new design work or RE of `TMotileDynamics`. REA (https://rea.tools/) plus Ghidra/IDA on the no-ASLR 64-bit exe, together with the ExanimaREP database, is the practical route.
11. **Delphi vs Free Pascal** for the 64-bit build (image base `0x100000000` points to FPC). This matters for interpreting RTTI and strings while doing RE.
12. **Licensing and legal:**
    - The game assets are Bare Mettle's. Like OpenMW, Exarust should load them from the user's own install and never redistribute them.
    - Several community sources are unlicensed or restrictively licensed (Turkish patch: no modification or redistribution; gnalvesteffer: CC BY-NC-SA; many: none). Use them as factual documentation only and re-implement independently.
    - EMTK (MIT/Apache-2.0) code can be reused with attribution.
