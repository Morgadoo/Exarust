# Design

## Presets at a load

`level_lighting_preset` applies the level's environment record, then the
preset, then derives the Bevy `Lighting`. The preset overrides the record's
stored sun mode and time, as the format notes say the game does. The indoor
preset leaves every shipped indoor level's lighting identical (checked over
the installed game): those levels had neither sun nor fog. The story
levels' fog variants differ only in the fog distance, which no indoor
picture shows, so the base preset stands for all of them. The outdoor
preset's local stream is seeded from the process clock at the load (the
original seeds it with the tick count at the match start); the engine's
shared weather stream is seeded from the same read, a labelled stand-in.

## Arena tables on a load

The table reader is not proven lossless on retail saves (no retail arena
save is available to test). Accepting the tables only when re-encoding
reproduces the save's table chunks byte for byte keeps the earlier
behaviour, chunks carried through unchanged, for any save the model cannot
rebuild, and gives the live tables for saves Exarust wrote.

## Not wired

The clock stamp, the table capture/apply and the level export have no live
caller until the slots are wired into the running game; the landing has
none until a level change happens in process.
