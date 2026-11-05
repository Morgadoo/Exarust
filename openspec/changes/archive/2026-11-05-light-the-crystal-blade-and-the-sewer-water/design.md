# Design

## What the original does

### The ability word and the socket

- The weapon section holds two fields:
  - the ability type, a u32 at body byte 0x44;
  - from byte 0x48, four u16 crystal sockets: bits 0–6 the kind, `0x80`
    when the socket exists, the charge in the high byte.
- Only four base records have a type:
  - 1: "UB5 FlangedMace";
  - 2: "Swrd Self Sharp";
  - 3: "Sword GSF 01";
  - 4: "Sceptre A".
- The only placed copy is `exanima07`'s local record 0xE9 (parent 0x487),
  with an empty socket (0x0080).
- No routine writes kind 4 into a socket except the item GUI's drop. It maps
  the crystals to socket kinds:
  - Power Crystal A (0x1AF) → 1;
  - Power Crystal B (0x1C3) → 2;
  - Power Crystal C (0x1C4) → 4;
  - Power Crystal AR (499) → 0x11.

  The charge is the stack's bits 16–23. A crystal already in the socket is
  handed back.

### The ability tick

- When the hands are dressed (and when the character view refreshes), the
  install registers one callback per held item:
  - types 1, 2 and 3 only;
  - one entry per item;
  - type 2 also gets an on-hit hook.
- The body tick queues the registry's run whenever the game clock passes the
  character's next time, which then moves 1000 ms on. A callback that no
  longer finds its item in the hands is dropped.

### Type 3

- The check (`fire_row::held_tick`) runs on the item's first socket:
  - low five bits 4 with a charge below 254, and no live row → light;
  - a row on its way out → light again;
  - a cold socket with a live row → put out.

  Nothing drains a kind-4 socket. The crystal drain only drains sockets with
  bit 0x10, and the type-1 hit only drains type-1 items.
- Lighting:
  - sets the item's two hit hooks (sizzle, put out);
  - raises the item node's lit flag 0x800;
  - creates a flame row from 30 to 115 cm on the item's +Y, fading in, with
    the row node's warm light and `FireS02`;
  - loads `FireSizzleA` but does not play it;
  - re-runs the moveset choice with the overlay 0x10020000 ("Flaming");
  - sets a body byte;
  - re-runs the player light switch.
- Putting out:
  - sets the row's extinguish flag;
  - clears the lit flag and the hooks;
  - re-runs the switch.
- The damage routine calls the striker's hit hook. The type-3 hook plays
  `FireSizzleA` at the contact, volume 0.3, pitch `0.8 + 0.2·u`.
- The player light switch puts the light out when a hand's item is of type 3
  while the player's first held item has the lit flag.

### The water

- The scene's water level is the scene word the environment's frame writes:
  −25 when the environment's terrain wetness is above 0.4 (double precision),
  otherwise −100000.
- That wetness is written by the static terrain's load, from the palette
  header's float at file offset 28, and by the save load's reset (0).
- Of the shipped palettes only `UWSwrA1` has a non-zero value (0.5). Its
  levels are `exanima05_sw` and `exanima07_sw`, where the channel floors lie
  60–130 cm down.
- The water is drawn by the terrain itself: above 0.4 the terrain program
  takes its WetFloor flag. Exarust already had this. There is no separate
  surface mesh.
- Wet steps, the comparison being the contact's height under the level:
  - The footwork's release step plays `StpWet` at the contact after the
    footstep (already ported as `wet_release_step`).
  - The walk's gait plant plays it at the footstep's place:
    - pitch `(0.3·p + 0.9)(0.05·thrust + 0.9)`;
    - gain `volume · depth · (0.5·g + 0.2)(0.01·thrust² + 0.01)`;
    - thrust is the gait table's first column as the locomotion blended it;
    - the depth share is `min(1, 0.05·(−water − 1 − y))`. That is always 1
      under a −25 surface.

## Choices

- Exarust runs the ability check for the player's two hand slots, reading
  the live object database. The original runs it for every character. The
  registry is replaced by scanning the hands once a second. The two agree
  while the hands hold the same items.
- The flame row is a child of the held blade's entity.
  - Its light is a sibling, placed at the line's middle (72.5 cm).
  - Its ribbon is a world-space mesh rebuilt each frame, drawn additively.
    It neither casts nor takes shadows, because the ribbon runs through its
    own light.
- The sizzle's random unit comes from the sound bank's stream.
- Types 1 and 2 are traced but not ported.
- The inventory's drag of a crystal onto a socket is not wired. The debug
  flag `--flame-sword` applies the GUI's drop rule to the record.
