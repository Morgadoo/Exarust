## Why

Roadmap task B4 with B1 (lane `forcefx`): the Force power effects
(TForceTorrentEffect, TForceBarricadeEffect, TForceBlastEffect,
TForceWaveEffect, TPwrForceStrikes_C) and the dynamics watch list. The
ledger rows named a missing contact (the torrent's simulated objects), a
watch list with no live caller, loop and impact sounds whose words the
ticks computed but no voice played, and the wave's missing alert at the
throw.

## Evidence

REA (Exanima.exe 0.9.5), the instructions read where constants, order or
branches matter:

- The torrent's simulated-object contact: the object kind test, each
  stream's live points from the head back, the reach width × the reach scale
  the last contact left + margin + 40, the sphere and box tests (an
  unordered compare passes), each object point within reach moved along the
  stream point's last move by (1 − d²/r²)·0.01, the stir summed in double
  (1e-5 a point), the wake (256) when one met. The part contact writes the
  reach scale before its walk; the static contact writes 0.25.
- The watch list: the blast's, wave's and barricade's starts add their sight
  entity; the blast's and wave's fizzle and spent chunks and the barricade's
  finished fall remove it; the removal calls each scene character's
  behaviour slot that forgets a character.
- The torrent's start: two looping voices at the node, volume 0, pitch 1,
  the stream loop's pitch then 6, each start the buffer length × 0.8 × the
  process stream's draw in double, the charge loop first. The tick writes
  the charge voice at the hand and its volume (0.4·(2 − c)·c charging, then
  −0.01 and freed on an ordered "below 0.02"), the stream loop's ease and
  fade; the frame moves the stream loop to the node; the destructor frees
  both.
- The barricade's entry: the hum voice at the node from the process
  stream's draw; its frame's hum words and node place; the body test's
  impact set at the last struck sphere in front, carried by the node frame.
- The wave's tick calls the blast's release alert (reach 1800 cm, its aim
  cleared first; the 1200 passed in a register is not read).
- Force Strikes' end: its set at the caster's place, volume 0.3, pitch
  0.6 + 0.4·r (the engine generator, in double).

## What changes

- `effects::torrent::stir_sim_object` and
  `thaum::torrent_live::stir_torrent_cloth` stir the level's cloths after
  the level contact; `LiveTorrent` keeps the reach scale the last contact
  left.
- `exa_physics::dynamics::{watch_add, watch_remove}` and
  `effects::danger::{HazardWatch, watch_hazards}`: the hazards are the watch
  list; one that leaves it is forgotten by every brain.
- `thaum::torrent_sound` plays the torrent's two loops; `LiveTorrent` keeps
  the start's two draws.
- `thaum::barricade_sound` plays the barricade's hum and the Force powers'
  one-shot sets (`ForceSounded`: the barricade's impacts, Force Strikes'
  end); the barricade's entry draws the hum's start from the process stream
  (it drew from the engine generator before).
- `thaum::blast_live::alert_known` is shared by the blast and the wave.

## Impact

Lockstep traces change in scenes with a torrent (two more process-stream
draws at each start; the cloths it pours through move), a barricade (its
entry draws from the process stream instead of the engine generator), a
thrown wave near characters its caster knows (their concern), a fallen
Force Strikes (one engine-generator draw) and any brain that kept a force
effect's contact after the effect went.
