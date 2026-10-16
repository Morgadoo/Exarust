# Exanima data formats

What is known about every format in a retail installation. Status words:
**verified** (a parser in this repo reads every shipped instance and the
output was checked), **framing** (container understood, payload partly),
**observed** (header fields seen, meaning inferred), **unknown**.

All integers are little endian. Entries in the main packs have no file
extension; the format is identified by its leading magic
(`exa_formats::FileKind::detect`).

| format | magic (bytes on disk) | where | status | spec |
|---|---|---|---|---|
| pack `.rpk` (also `.rml`, `.flb`, `.fds`, `actors.rpk`) | `01 0C BF AF` | every pack | verified | [RPK.md](RPK.md) |
| project `.rfp` | `10 BD DF AF` | `Exanima.rfp` | verified | [RPK.md](RPK.md#project-file-rfp) |
| chunk tree `.rfc` (meshes, materials, levels) | `CF AF 23 3D` (`CF AF 21 3D` on 3 meshes) | Objlib, Objects, Characters, CharExt, Components, Resource | verified (nodes, static meshes, lights, material texture layers, level grid header, props); grid planes B/C observed | [RFC.md](RFC.md) |
| image `.rfi` | `C6 3D 2D 1D` | Textures, Apparel, ComponentTex, Emblems, Resource | verified | [RFI.md](RFI.md) |
| skeleton + motions `.rsq` | `00 00 00 3D` | `Resource/humanoid.rsq` | framing | [RSQ.md](RSQ.md) |
| database `.rdb` | `xx 0D xx DB` (strings: `02 0C BF AF`) | `Resource/*.rdb` | framing | [OTHER.md](OTHER.md#databases-rdb) |
| terrain `.rft` | `01 AD EF 3E` | Resource (17) | verified | [TERRAIN.md](TERRAIN.md) |
| terrain table `.ftb` | `02/03 BD EF 3E` | Resource (15) | verified | [TERRAIN.md](TERRAIN.md) |
| power `.pwr` | `CE 01 CE AF` | Resource (4) | observed | [OTHER.md](OTHER.md#powers-pwr) |
| weapon factory | `0x 0F CE AF` | Factories (106) | observed | [OTHER.md](OTHER.md#weapon-factories) |
| actor | `06/07 AC CD 00` | `Resource/actors.rpk` (21) | verified (character record) | [OTHER.md](OTHER.md#actors) |
| material library `.rml` | pack | Resource (4) | verified (texture layers) | [OTHER.md](OTHER.md#material-libraries-rml) |
| shader library `.flb` | pack, entries obfuscated | Resource (2) | unknown | [OTHER.md](OTHER.md#shader-libraries-flb) |
| save `.rsg` (not shipped) | `02 EA 7A DA` | `%APPDATA%\\Exanima` | reference only | [OTHER.md](OTHER.md#saves-rsg-rcp) |
| manual `.fds` | pack of marked-up text | Resource (2) | verified | [OTHER.md](OTHER.md#manual-pages-fds) |
| sound | `RIFF` (PCM WAVE) | Sound (1131) | verified | [OTHER.md](OTHER.md#sound) |

The engine itself is a Free Pascal 3.2.2 Win64 program using OpenGL 4.x
(core functions loaded at runtime, `glTexStorage2D`, uniform blocks,
instancing, S3TC/RGTC/LATC compressed textures) and OpenAL Soft with EFX.
The class map recovered from the binary is in
[`../research/engine-classes.md`](../research/engine-classes.md).
