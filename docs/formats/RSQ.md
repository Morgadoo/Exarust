# Skeleton and motions (`.rsq`)

Status: **framing**. Only `Resource/humanoid.rsq` ships (713 KiB).

```
chunk 0x3D000000  bone list: u32 count (18), then 0x00003D01 bone chunks
chunk 0x00003DAB  motion set (724 208 bytes)
```

## Bones (`3D01`)

`name[16]` whose first characters are overwritten by a two-digit index
(`01lvis` = 01 Pelvis, `08umbar` = 08 Lumbar, `09orax` = 09 Thorax,
`10ervical`), then small integer fields (`01 00 01 08 10 00 23 00`), a 3×3
rotation and a translation, followed by per-bone extra data (sizes 212–344
bytes; likely collision/body-part shapes and joint limits).

The 18 bones: pelvis, femur/lower leg/foot ×2, lumbar, thorax, cervical,
shoulder/humerus/forearm/hand ×2 (plus head).

## Motion set (`3DAB`)

Header `u32 18` (bones), `u32 48` (sequences), `u32 32` (name length?),
then named sequences, among them: `Walk`, `Backpedal`, `Defence stance`,
`Cmbt - Idle`, `Attack Mid/Down/Up`, `Attack Combo 1..3`, `Overhead Swing`,
`Get up` ×3, `Fall Forward/Backward/High`, `Parry Left/Right/Up/Left Dn/Rgt Dn`,
`Break Tangle`, `Unconcious 1/2`, `Sitting`, `Switch Weapons`, `Rebalance`,
`Force Blast/Barrier/Raise/Weapon/Torrent`, `Energy Shock`, `Disp Form`,
`Combat Walk`, `Head Pain`, `Idle 2`, `Default`.

Roughly 48 clips for the whole game supports what the class names suggest
(`TMotileDynamics`, `TMotionController`, `TMCSequenceSet`): characters are
physically simulated and clips are targets for the body's motors.
