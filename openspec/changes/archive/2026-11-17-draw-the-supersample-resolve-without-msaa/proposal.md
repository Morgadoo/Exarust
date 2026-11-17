## Why

Lane `blackfix`: on the owner's RTX 3090 (NVIDIA driver 620.70.06, Vulkan)
the default game (supersampling on) drew the 3D world black in a window
while the interface drew normally; `--no-supersample` rendered. The build
worker (RTX 4000 SFF Ada, driver 615) runs offscreen only and rendered.

## Evidence

- The black frames reproduce only in a window: the same binary, scene and
  size render correctly offscreen on the 3090 (mean luminance 22 against
  0.04 in a window on a virtual X display). The 2026-11-16 measurements
  that looked offscreen were windowed runs.
- A resolve shader that returns solid magenta still gave black; a red clear
  colour on the display camera gave a red screen. So the display view ran
  and cleared, but its resolve quad's draw never reached the target.
- With `Msaa::Off` on the display camera the frame renders (24.2), and
  with MSAA 4 but an 8-bit view (no HDR) it renders too (18.7): the loss
  needs a multisampled HDR display view drawing to a window. No wgpu error
  is logged.
- The original multisamples nothing (`HWMultisample = 0`), and the display
  camera draws one quad shaded once a pixel, so MSAA cannot change its
  picture.

## What Changes

- The supersampling display camera is spawned with `Msaa::Off`.
- `EXA_CAMERA_TRACE=1` logs every camera's order, activity, target, layers
  and size every 120 frames (a debug aid).
- Finding F570.
