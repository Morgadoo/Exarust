---
name: retail-exanima
description: Launch the retail Exanima game (Steam app 362490) under Proton and interact with its live render camera (read, pin, fly, screenshot) for side-by-side comparison with Exarust. Use when the user wants to start, run, or launch the original / retail Exanima from Steam, or control, pin, fly, or screenshot its camera. Triggers include "launch exanima", "run the retail game", "start the original from steam", "pin the retail camera", "grab a retail shot", "compare with retail".
allowed-tools: Bash
license: MIT
compatibility: Flatpak Steam + Proton on a host with an NVIDIA GPU. Runs from the host or the project distrobox (host actions routed through host-spawn). The camera tool needs context/tools/exa_freecam/exa_freecam.py and kernel.yama.ptrace_scope=0.
metadata:
  author: exarust
  version: "1.0"
---

Drive the **retail** `Exanima.exe` — the user's own Steam copy, single player,
under Proton — so its output can be compared against Exarust. Everything goes
through one script:

```
.claude/skills/retail-exanima/launch.sh
```

It resolves the install from Steam's own library index (app id **362490**), so
no machine paths are hard-coded, and it works whether Claude Code runs on the
host or in the project distrobox (host-only commands are routed through
`host-spawn`). **This skill touches only the user's running game and local Steam
config; it writes nothing to git** (the camera tool and its retail addresses
live under `context/`, which is git-ignored).

## Launch

```bash
.claude/skills/retail-exanima/launch.sh              # windowed, waits until it draws
.claude/skills/retail-exanima/launch.sh --fix-driver # also fix an NVIDIA driver mismatch
.claude/skills/retail-exanima/launch.sh --fullscreen # exclusive fullscreen instead
.claude/skills/retail-exanima/launch.sh status       # running? on the GPU?
.claude/skills/retail-exanima/launch.sh stop         # kill it
```

The launcher, in order: resolves the install and Proton prefix; checks the
Flatpak NVIDIA driver; sets the display mode; stops any running instance;
launches **Steam's own Proton directly** (not the Steam client — see failure
mode 3); waits until `Exanima.exe` is a graphics process on the GPU.

Defaults to **windowed** (`Fullscreen = 0` in the prefix's `Exanima.ini`).
Exclusive fullscreen on a multi-monitor, mixed-resolution desktop makes the game
pick a bad mode and exit a few seconds after it starts — windowed avoids that.
Pass `--fullscreen` to force it, or `--no-config` to leave the ini untouched.

### Three failure modes this skill exists to handle

1. **Instant crash / renders on `llvmpipe`.** The Flatpak NVIDIA driver
   extension must match the host driver exactly. After a host driver update the
   match breaks and every GPU game falls back to the software renderer and dies
   within ~10s. `launch.sh` reports this; `--fix-driver` installs the matching
   `org.freedesktop.Platform.GL.nvidia-<ver>` (+ `GL32`). Verify a real launch
   with `status` — it should show `Exanima.exe` with GPU memory, not nothing.
2. **Starts then exits after ~30s.** Usually fullscreen on this desktop; the
   windowed default fixes it.
3. **`Failed to configure device context pixelformat`.** The Steam client's
   in-game overlay hooks Exanima's OpenGL context and kills it. That is why the
   default launch (`--direct`) invokes Steam's Proton + prefix directly, with no
   overlay. `--via-steam` goes through the client (tracks playtime) but currently
   hits this error.

## Interact with the running game

The camera tool `context/tools/exa_freecam/exa_freecam.py` reads and overrides
the live render camera through `/proc/PID/mem`. Reach it via the launcher so the
host hop and tool path are handled:

```bash
launch.sh camera -- get                              # print pose as Exarust --camera args
launch.sh camera -- set x,y,z,yaw,pitch [--fov F] [--shot out.png] [--persist]
launch.sh camera -- fly [--fov F] [--speed M]        # WASD/QE + arrows; Esc restores
launch.sh camera -- shot out.png                     # capture the 1600x1080 client area
launch.sh camera -- release                          # undo a `set --persist`
launch.sh camera -- selftest                         # offline maths + cave-assembly check
```

Pose syntax is the Exarust viewer's (`--camera x,y,z,yaw,pitch --fov F`): metres,
Y up, yaw/pitch in degrees, FOV vertical. The intended loop is: `get` the retail
pose, run Exarust with the same `--camera …` line, and compare. `--shot` /
`shot` need the game drawing (see failure mode 1).

**Safety:** `get`, `shot`, `fly`, and a plain `set` restore the game on exit
(Ctrl-C / SIGTERM / SIGHUP / Esc). Only `set --persist` leaves bytes patched —
undo with `release`. If a run is `SIGKILL`ed mid-patch the game may stay stopped;
resume with `kill -CONT <pid>`, then `release`. The hardcoded addresses target
retail Exanima **0.9.5.2g**; on a game update they must be re-verified or the
patch will crash the running game. Full engine layout and patch-site evidence:
`context/tools/exa_freecam/README.md`.

## Notes

- Requires the game already installed in Steam and `kernel.yama.ptrace_scope=0`
  for the camera tool. Inside the distrobox the tool re-execs itself on the host.
- The launcher never commits anything and reads the game path live, so it keeps
  working if the user moves Exanima to another Steam library.
