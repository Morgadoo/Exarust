# Print the role database and the records' power ids

## Why

Checking how a race rates the player, and which placed character knows a
power (the demon summoning), needed the role database's attitude entries
and each record's power ids, which no command printed.

## What changes

- `exa-tool roles` prints every role of `charroles.rdb` with its parent,
  weight and flags, and its attitude entries.
- `exa-tool characters` names each record's known power ids after their
  count.
