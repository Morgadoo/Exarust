# Build and run

Requirements: Rust stable via rustup (`rust-toolchain.toml` pins the channel)
and GNU Make. On Linux, Bevy needs a C toolchain plus the ALSA, udev,
Wayland and X11 development headers:

| distro | packages |
|---|---|
| Fedora | `sudo dnf install gcc-c++ alsa-lib-devel systemd-devel libxkbcommon-devel wayland-devel libX11-devel libXcursor-devel libXi-devel libXrandr-devel` |
| Debian/Ubuntu | `sudo apt install g++ pkg-config libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev` |
| Arch | `sudo pacman -S base-devel alsa-lib systemd-libs libxkbcommon wayland libx11 libxcursor libxi libxrandr` |

On Fedora Silverblue, build inside a toolbox.

You need your own Exanima installation (Steam app 362490):

```bash
cp .env.example .env        # set EXANIMA_DIR to the folder with Exanima.rfp
make info                   # pack inventory via exa-tool
make run                    # Bevy window: first light
make run TEXTURE=afcrnflg_01d
make check                  # fmt + clippy + tests
```

`exa-tool` usage: `cargo exa --help` (release build of `crates/exa_tool`).
