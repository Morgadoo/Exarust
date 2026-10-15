# Free camera

A mod for the retail game. It is not part of the Exarust runtime.

While it is on, the view leaves the player and can fly out of a level. A
panel lists every map in the installation and loads the one you pick.

## Keys

| key | action |
|---|---|
| F1 | free cam on or off, and the panel |
| Ctrl + mouse | look |
| W A S D | move |
| Space / Q | up / down |
| Shift | four times faster |

Speed and field of view are fields in the panel. "Load map" opens the
selected level.

The panel opens on its own about a second after the game starts. F1 hides it
and turns free cam off; press F1 again to bring both back. Free cam only moves
the view once a level is loaded.

## Install

Build on Linux with a 64-bit mingw-w64 gcc named `x86_64-w64-mingw32-gcc` on
`PATH`:

```
cargo build --release
```

`cargo` links the mod against the Universal CRT, which Proton does not ship.
Rewrite those imports onto `ucrtbase.dll` (the same functions, one DLL Proton
does ship):

```
python3 - <<'PY'
src = "target/x86_64-pc-windows-gnu/release/exa_freecam.dll"
data = bytearray(open(src, "rb").read())
old, repl, at = b"api-ms-win-crt-", b"ucrtbase.dll", 0
while (at := data.find(old, at)) >= 0:
    end = data.find(b"\0", at)
    data[at:at + len(repl)] = repl
    data[at + len(repl):end] = b"\0" * (end - at - len(repl))
    at = end
open(src, "wb").write(data)
PY
```

The game loads the mod through a `winmm.dll` proxy that sits next to
`Exanima.exe`. It forwards `timeBeginPeriod` and `timeEndPeriod` to the system
`winmm.dll`, then loads `exa_freecam.dll` and calls its exported `enable` once
the loader lock is released. Starting the mod from `DllMain` closes the game.
Build that proxy from `loader/winmm.c` in this directory:

```
x86_64-w64-mingw32-gcc -shared -m64 -O2 -o winmm.dll loader/winmm.c -Wl,--kill-at
```

Copy `winmm.dll` and `exa_freecam.dll` into the game directory. Proton ships
its own `winmm.dll` and ignores one beside the exe unless told otherwise, so
the Exanima compatibility prefix needs one line under
`[Software\\Wine\\DllOverrides]` in its `user.reg`:

```
"winmm"="native,builtin"
```

`config.toml` is only read by EMTK. A loader that calls the exported `enable`
(and `disable` to remove the hooks) does not need the proxy.

The hooks are byte patterns from Exanima 0.9.5.2g, not fixed addresses. If a
game update moves the camera or the level loader, the mod does nothing
instead of writing to the wrong place.
