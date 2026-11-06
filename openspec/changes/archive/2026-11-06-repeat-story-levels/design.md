## Decisions

- The stage timeout stays for interactive runs (a stalled stage must not
  hang the game) and is lifted for `--fixed-frames`.
- The build waits for dressing only behind the loading screen; a body
  spawned in play is built as before.
