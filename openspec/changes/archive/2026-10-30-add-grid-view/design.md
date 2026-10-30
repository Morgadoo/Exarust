# Design

## One game per process

The game's state lives in world resources, so the host never shares a
world with a game. It is a small Bevy app with `DefaultPlugins` and none
of the game's plugins. Each quadrant's game is the ordinary binary with
`--offscreen`, so the game runs exactly the code a windowed run does.

## Host: Bevy's split-screen pattern

The host follows `examples/3d/split_screen.rs`: one camera a quadrant,
`Camera::order` its index, a `CameraPosition` marking its place in the
2 × 2 grid, and a system that on `WindowResized` sets each camera's
`Viewport` to `window.physical_size() / 2` at `pos * size`. The labels
are interface nodes aimed at their camera with `UiTargetCamera`, as the
example's per-camera captions are. `many_cameras_lights` uses the same
`Viewport` per camera, which works because each camera renders only its
rectangle. The cameras here are 2D and each sees only its own
`RenderLayers` layer. That layer holds one sprite with the child's frame,
under an orthographic projection fixed to the child's frame size, so a
viewport of any size shows the whole frame.

## Child: no window, cameras into an image

Without winit, `DefaultPlugins` (with `bevy_window`) carries no runner, so
the child adds `ScheduleRunnerPlugin::run_loop(0)`. The game's frame-rate
cap (`--max-fps`, given by the host, 60 by default) paces it. The
`WindowPlugin` still spawns the primary `Window` entity at the quadrant's
size with scale factor 1. It has no raw handle, so the renderer creates
no surface and nothing is ever presented: no frame can wait on a hidden
window's presentation. Systems that read the window still see its size,
its focus (always on) and the cursor.

In `PostUpdate`, before the camera and interface systems, every camera
whose `RenderTarget` is the primary window is pointed at one
`Rgba8UnormSrgb` image (Bevy's default view format). That covers the
supersampling display camera, the loading/interface camera and the fly
camera without supersampling. The scene camera of a supersampled run
already draws into its own image. Bevy's own picks are kept: the
highest-ordered of those cameras gets `IsDefaultUiCamera` (Bevy picks the
highest-ordered window camera for the interface) and the lowest-numbered
one gets `ShadowLodOrigin` (Bevy picks the lowest-numbered window camera).
A camera retargeted later, such as a new display camera after the
supersampling switch, is caught the next frame.

`gpu_readback::Readback` reads the image every frame. The observer strips
the rows' copy padding, forces alpha opaque (a compositor ignores a
window's alpha, a host sprite would not) and writes the frame to
`/dev/shm/exarust-grid-<pid>-<n>.rgba` with `pwrite`. The file has a
64-byte header (magic, sequence number, size, frame counter, a
loading-screen flag) followed by the pixels. The sequence number is odd
while the pixels are written. The host reads header, pixels, header and
drops a frame whose sequence number changed. At 1280 × 720 that is
3.7 MB a frame, a memory copy on each side.

## Input

The host writes one line an event to the child's standard input, from a
writer thread of its own so a stalled child never blocks the host: keys
(key code and named logical keys by their reflected variant name,
characters and text as hex), buttons, the cursor in the child's pixels,
cursor left, mouse motion and the wheel. The child reads the lines on a
thread and, before Bevy's input systems, writes them as
`KeyboardInput`, `MouseButtonInput`, `CursorMoved`, `CursorLeft`,
`MouseMotion` and `MouseWheel` messages for its window, and moves the
window's cursor. The game then reads them exactly as it reads winit's.
Pressed keys and buttons are logged at info, which is how a test can
tell which game received what.

The active quadrant is the one last clicked: a press (with no other
button held) over another quadrant first releases everything held in the
old one, then goes to the new one. Hovering does not move the focus, so
a right-drag look may cross a quadrant's edge.

## Launch pacing

A thread polls `nvidia-smi` every second for the adapter
`WGPU_ADAPTER_NAME` names: used and free memory, and each process's use.
A quadrant launches only when no other game has yet to show its first
frame, and when free memory covers the per-game need plus what each game
still loading has yet to take (the need minus its process's current use).
The need defaults to `context/tools/gpu_run.sh`'s 4600 MiB a full run
(`--gpu-need`, 0 turns the check off). Without `nvidia-smi` the check is
skipped.

## Open questions

None for the original: nothing here reproduces original behaviour.
