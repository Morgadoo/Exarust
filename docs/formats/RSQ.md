# Skeleton and motions (`.rsq`)

Status: **verified** (REA on the reader; `exa_formats::rsq` consumes all
729 932 bytes of `humanoid.rsq`; the walk cycle plays correctly in Exarust).
Only `Resource/humanoid.rsq` ships.

The file is a bare chunk stream (no sanity word):

```
chunk 0x3D000000  scene-node list: 18 bones (plain nodes 0x3D01, see RFC.md)
chunk 0x00003DAB  motion set
```

## Bones

Standard scene nodes. The first two characters of the name are the bone
index (`01lvis` = 01 pelvis, `08umbar` = 08 lumbar); the rest of each node
is sub-chunks (likely body-part collision shapes). Hierarchy:

```
01 pelvis ─ 02 femur ─ 03 lower leg ─ 04 foot
          ─ 05 femur ─ 06 lower leg ─ 07 foot
          ─ 08 lumbar ─ 09 thorax ─ 10 cervical
                                  ─ 11 shoulder ─ 12 humerus ─ 13 forearm ─ 17 hand
                                  ─ 14 shoulder ─ 15 humerus ─ 16 forearm ─ 18 hand
```

Character models (`humanbase.rfc`, …) are hierarchies of body-part meshes
named with the same indices (`01Pelvis`, `09Chest`, `17RHand`).

## Motion set (`3DAB`)

```
u32 bones (18), u32 actions (48), u32 movesets (32)
action × actions:   char name[16], u32, u32
moveset × movesets:
  char name[16]          "One Handed"
  char parent[16]        "Default": the moveset that supplies missing actions
  per action:
    i32 keys             ≤ 0: not defined in this moveset
    i32 mode             (3 in most clips)
    f32 time[keys]       frames (10, 20, 30, …)
    f32 root[keys][3]    pelvis position (cm)
    pose[keys][bones]:   f32 rotation[4] (local x, y, z, w), u32 flags
```

Actions: Walk, Run, Backpedal, Defence stance, Cmbt - Idle, Attack
Mid/Down/Up, Attack Combo 1–3, Stab, Overhead Swing, Kick, Get up ×3, Fall
Forward/Backward/High, Jump, Idle, Idle 2, Parry Left/Right/Up/Left Dn/Rgt
Dn, Stand 2, Scratch 1, Break Tangle, Unconcious 1/2, Sitting, Switch
Weapons, Kneel, Rebalance, Force Blast/Barrier/Raise/Weapon/Torrent, Energy
Shock, Disp Form, Cower, Combat Walk, Power, Head Pain.

Movesets (parent): Default; One Handed (Default); 1H & Shield, 1H & Torch,
2W Light, Torch, 1H Heavy (One Handed); Shield C1/C2 (1H & Shield); Two
Handed (Default); 2H Light, Greatsword (Two Handed); 2H Flame (2H Light);
Polearm (Default); Golem, Golem B; Herath, Gobbler, Fael, Demon A01,
Harvester, Herath Ogre, Ogre, Ogre old, UnarmClaw; and creature variants of
One Handed / 1H & Shield / Polearm.

About 1 200 keys in total serve every humanoid and creature: the original
drives physically simulated bodies toward these poses rather than playing
them directly.
