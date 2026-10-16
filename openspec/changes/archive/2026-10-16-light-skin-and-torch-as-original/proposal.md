## Why

Lit character skin bloomed: in the character view the sunlit body burned to
white, and in daylight arenas the carried torch lit the player's skin orange
and the bloom spread onto the ground. The torch hung in the wrong hand, and
the character view was a grey plane under a sun, unlike the original's
creation screen. The post chain is the original's; the causes were set-up
differences.

Phase 5 (visual fidelity), with Phase 3's player look (M3).

## What Changes

- Human and ancient bodies draw with the original's built-in skins: the race
  texture's stored texels times the middle skin tone, decoded once; F0 0.04,
  roughness from the gloss map; a subsurface share lit by a wrapped term.
- `exarust character` shows the original's creation scene (`sgccflr01`, its two
  omni lights, no sun or ambient, a fade to black at 2 m, the original camera
  at 50°).
- The torch goes in the right hand with the "Torch" moveset, or the left with
  "1H & Torch" beside a weapon, placed by the original's grip table in the
  original's rotation order. Practice and arena levels start without a torch.

Evidence: reverse engineering of `Exanima.exe` 0.9.5 (the race texture load
flags, the skin composite and its sRGB target, the built-in skin materials, the
grip table and its matrix builder, the character screen's scene and fog mode);
the user's screenshots of the original's creation screen and of the player with
the torch.

Out of scope: per-character skin tones, women's textures, the physique blend of
the normal maps, dirt and blood layers, the inventory and trade dolls' lighting,
scaling the grip by character size, the hand's closed fist on the handle.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: carried torch (hand, grip, levels without a torch).
- `rendering`: built-in skin.
- `world-viewer`: character creation scene.

## Impact

- `exa_assets` (`human_skin.rs`, `model.rs`), `exa_render` (skin shading,
  fade to black), `exa_game` (`torch.rs`, `player.rs`), `exa_world`
  (`start.rs` modes, `environment.rs`), `exarust` (`viewer.rs`);
  `docs/RENDERING.md`, `docs/PLAYER.md`, `docs/PLAN.md`.
