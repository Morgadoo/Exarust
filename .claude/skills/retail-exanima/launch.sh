#!/usr/bin/env bash
# Launch and drive the retail Exanima (Steam app 362490) for side-by-side
# comparison with Exarust. No machine paths or retail addresses live here: the
# install is resolved from Steam's own library index, and the camera tool it
# shells out to lives under context/ (git-ignored).
#
#   launch.sh [launch] [--direct|--via-steam] [--fullscreen|--windowed]
#             [--fix-driver] [--no-config] [--no-wait] [--wait N]
#   launch.sh status                 is it running, and is it on the GPU?
#   launch.sh stop                   kill the running game
#   launch.sh camera -- <args...>    passthrough to exa_freecam.py (get/set/fly/shot/release)
#
# Launch uses Steam's own Proton + prefix but invokes them DIRECTLY (--direct,
# the default): the Steam client's in-game overlay hooks Exanima's OpenGL
# context and makes it die with "Failed to configure device context
# pixelformat", so going through the client (--via-steam) currently fails.
#
# Works from the host or from the project distrobox: host-only actions
# (flatpak, nvidia-smi) are routed through host-spawn when inside a container.
set -euo pipefail

APPID=362490
STEAM_APP=com.valvesoftware.Steam
STEAM_ROOT="$HOME/.var/app/$STEAM_APP/.local/share/Steam"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
FREECAM="$REPO_ROOT/context/tools/exa_freecam/exa_freecam.py"

# ---- host bridge --------------------------------------------------------------
HOST=""
if [ -f /run/.containerenv ] || [ -f /run/.toolboxenv ]; then
  if command -v host-spawn >/dev/null 2>&1;        then HOST="host-spawn"
  elif command -v flatpak-spawn >/dev/null 2>&1;   then HOST="flatpak-spawn --host"
  else echo "error: inside a container but neither host-spawn nor flatpak-spawn found" >&2; exit 1
  fi
fi
hs() { if [ -n "$HOST" ]; then $HOST "$@"; else "$@"; fi; }

die() { echo "error: $*" >&2; exit 1; }

# ---- resolve the install from Steam's library index ---------------------------
LIB="" GAME_DIR="" COMPAT="" INI=""
resolve() {
  local vdf="$STEAM_ROOT/steamapps/libraryfolders.vdf"
  [ -f "$vdf" ] || die "Steam library index not found: $vdf (is the Flatpak Steam installed?)"
  local lib
  while IFS= read -r lib; do
    if [ -f "$lib/steamapps/appmanifest_${APPID}.acf" ]; then LIB="$lib"; break; fi
  done < <(grep -oP '"path"\s*"\K[^"]+' "$vdf")
  [ -n "$LIB" ] || die "Exanima (app $APPID) is not installed in any Steam library. Install it in Steam first."
  local installdir
  installdir="$(grep -oP '"installdir"\s*"\K[^"]+' "$LIB/steamapps/appmanifest_${APPID}.acf")"
  GAME_DIR="$LIB/steamapps/common/$installdir"
  COMPAT="$LIB/steamapps/compatdata/${APPID}"
  INI="$COMPAT/pfx/drive_c/users/steamuser/AppData/Roaming/Exanima/Exanima.ini"
  echo "install:   $GAME_DIR"
  echo "prefix:    $COMPAT"
}

# ---- NVIDIA Flatpak driver must match the host (else llvmpipe -> instant crash)
check_driver() {
  local hv ext ext32
  hv="$(hs cat /sys/module/nvidia/version 2>/dev/null | tr -d '[:space:]' || true)"
  [ -n "$hv" ] || { echo "driver:    no NVIDIA module loaded, skipping check"; return 0; }
  ext="org.freedesktop.Platform.GL.nvidia-${hv//./-}"
  ext32="org.freedesktop.Platform.GL32.nvidia-${hv//./-}"
  if hs flatpak info "$ext" >/dev/null 2>&1; then
    echo "driver:    Flatpak NVIDIA $hv OK"
    return 0
  fi
  echo "driver:    Flatpak NVIDIA $hv MISSING -> games render on llvmpipe and crash on launch"
  if [ "${FIX_DRIVER:-0}" = 1 ]; then
    echo "driver:    installing $ext (+ GL32) ..."
    hs flatpak install -y flathub "$ext" "$ext32"
  else
    echo "           fix: re-run with --fix-driver, or on the host:"
    echo "           flatpak install flathub $ext $ext32"
  fi
}

# ---- display mode (0 = windowed, 1 = fullscreen) ------------------------------
# Edit only while the game is stopped; Exanima rewrites the ini on exit.
set_display_mode() {
  local mode="$1"
  if [ ! -f "$INI" ]; then
    echo "config:    Exanima.ini not created yet (first launch will make it); skipping"
    return 0
  fi
  if grep -q '^Fullscreen = ' "$INI"; then
    sed -i "s/^Fullscreen = .*/Fullscreen = $mode/" "$INI"
  else
    printf 'Fullscreen = %s\n' "$mode" >> "$INI"
  fi
  echo "config:    Fullscreen = $mode ($([ "$mode" = 0 ] && echo windowed || echo fullscreen))"
}

game_on_gpu() { hs nvidia-smi 2>/dev/null | grep -qi 'Exanima\.exe'; }

wait_ready() {
  local timeout="$1" i=0
  echo -n "waiting for Exanima to draw on the GPU "
  while [ "$i" -lt "$timeout" ]; do
    if game_on_gpu; then echo " ok (${i}s)"; return 0; fi
    echo -n "."; sleep 3; i=$((i + 3))
  done
  echo
  echo "warning: not drawing on the GPU after ${timeout}s - check 'launch.sh status'" >&2
  return 1
}

# ---- launch helpers -----------------------------------------------------------
# Which Proton built the prefix (read from the prefix's own config_info), so the
# direct launch uses the same tool Steam would have.
detect_proton() {
  local p
  p="$(sed -n '2p' "$COMPAT/config_info" 2>/dev/null | grep -oP '/common/\K[^/]+(?=/files)' || true)"
  echo "${p:-Proton - Experimental}"
}

# Steam Linux Runtime to wrap Proton in (Proton Experimental ships on SLR 4).
detect_slr() {
  local d
  for d in SteamLinuxRuntime_4 SteamLinuxRuntime_sniper SteamLinuxRuntime_soldier; do
    [ -x "$STEAM_ROOT/steamapps/common/$d/_v2-entry-point" ] && { echo "$d"; return; }
  done
  echo SteamLinuxRuntime_4
}

stop_game() {
  hs pkill -9 -f 'Exanima\.exe' 2>/dev/null || true
  hs pkill -9 -x wineserver 2>/dev/null || true
}

# Reliable launch: Steam's own Proton + prefix, invoked directly (no Steam
# client, hence no in-game overlay - the overlay breaks Exanima's GL context).
start_direct() {
  local appdir="$HOME/.var/app/$STEAM_APP" proton slr
  proton="$(detect_proton)"; slr="$(detect_slr)"
  echo "launch:    direct Proton ($proton via $slr)"
  cat > "$appdir/.exarust-launch.sh" <<EOF
#!/bin/bash
export STEAM_COMPAT_DATA_PATH="$COMPAT"
export STEAM_COMPAT_CLIENT_INSTALL_PATH="\$HOME/.local/share/Steam"
cd "$GAME_DIR" || exit 2
exec "\$HOME/.local/share/Steam/steamapps/common/$slr/_v2-entry-point" \\
  --verb=waitforexitandrun -- \\
  "\$HOME/.local/share/Steam/steamapps/common/$proton/proton" \\
  waitforexitandrun "./Exanima.exe"
EOF
  chmod +x "$appdir/.exarust-launch.sh"
  setsid bash -c "${HOST:+$HOST }flatpak run --command=bash $STEAM_APP -c 'bash \"\$HOME/.exarust-launch.sh\"'" \
    >/dev/null 2>&1 &
  disown 2>/dev/null || true
}

# Steam-client launch. Currently fails on this setup: the overlay makes Exanima
# die with "Failed to configure device context pixelformat". Kept for the day
# the overlay is fixed (it tracks Steam playtime, which --direct does not).
start_steam() {
  echo "launch:    steam://rungameid/$APPID (via Steam client - may hit the GL overlay bug)"
  hs flatpak run "$STEAM_APP" "steam://rungameid/$APPID" >/dev/null 2>&1 || true
}

# ---- subcommands --------------------------------------------------------------
cmd_launch() {
  local display=0 touch_ini=1 do_wait=1 wait_s=60 via=direct
  while [ $# -gt 0 ]; do
    case "$1" in
      --direct)     via=direct ;;
      --via-steam)  via=steam ;;
      --fullscreen) display=1 ;;
      --windowed)   display=0 ;;
      --no-config)  touch_ini=0 ;;
      --fix-driver) FIX_DRIVER=1 ;;
      --no-wait)    do_wait=0 ;;
      --wait)       shift; wait_s="${1:?--wait needs a number}" ;;
      *) die "unknown launch option: $1" ;;
    esac
    shift
  done
  resolve
  check_driver
  if [ "$touch_ini" = 1 ]; then set_display_mode "$display"; fi
  stop_game
  sleep 2
  if [ "$via" = steam ]; then start_steam; else start_direct; fi
  if [ "$do_wait" = 1 ]; then wait_ready "$wait_s" || true; fi
  echo
  echo "Interact with the running game:  $(basename "$0") camera -- get   (also: set / fly / shot / release)"
}

cmd_status() {
  resolve
  if game_on_gpu; then
    echo "status:    RUNNING (drawing on GPU)"
    hs nvidia-smi 2>/dev/null | grep -i 'Exanima\.exe' || true
  elif hs pgrep -f 'Exanima\.exe' >/dev/null 2>&1; then
    echo "status:    process present but NOT on the GPU (starting, or fell back to llvmpipe)"
    check_driver
  else
    echo "status:    not running"
  fi
}

cmd_stop() {
  if hs pkill -f 'Exanima\.exe' 2>/dev/null; then echo "stopped"; else echo "nothing to stop"; fi
}

cmd_camera() {
  [ -f "$FREECAM" ] || die "camera tool not found: $FREECAM (it lives under context/, which is git-ignored)"
  [ "${1:-}" = "--" ] && shift
  # Run the tool on the host directly (EXA_FREECAM_HOST=1 stops it self-re-exec'ing
  # through host-spawn, which can swallow its stdout). fly/shot need DISPLAY.
  if [ -n "$HOST" ]; then
    $HOST env EXA_FREECAM_HOST=1 "DISPLAY=${DISPLAY:-:0}" python3 "$FREECAM" "$@"
  else
    python3 "$FREECAM" "$@"
  fi
}

usage() {
  # Print the leading comment header (everything after the shebang up to the
  # first non-comment line), with the "# " prefix stripped.
  awk 'NR==1{next} /^#/{sub(/^# ?/,""); print; next} {exit}' "${BASH_SOURCE[0]}"
}

case "${1:-launch}" in
  launch)        shift || true; cmd_launch "$@" ;;
  status)        cmd_status ;;
  stop)          cmd_stop ;;
  camera)        shift; cmd_camera "$@" ;;
  -h|--help|help) usage ;;
  --*)           cmd_launch "$@" ;;   # bare flags imply launch
  *)             die "unknown command: $1 (try: launch | status | stop | camera | help)" ;;
esac
