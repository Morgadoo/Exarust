## Why

Roadmap task B4, lane `saves2`: the triage's port lanes L11 (saves, flow
and arena), L16 (world objects, script and audio) and L17 (runtime). A save
wrote the part records of a placed object's or prop's own bodies and cloth
from the section it had loaded, so a door left open, a swung lever arm or a
moved chain came back where the level shipped them; the doors' and levers'
object words (latch, mode, countdown, an armed return's time) were kept on
the bodies but never written; and the main menu's music code switched its
theme without the audio environment's crossfade; a companion's rise computed
its AI's two shares but never wrote them.

## Evidence

REA (Exanima.exe 0.9.5): the exporter's part-record walk (an owned child
with an attachment: a body writes its name, the attachment's word and the
node's world matrix, a cloth its particles through the compacting copy, a
child without one is passed over, any other attachment ends the
siblings' records), the placed-object record writer (the placement word is
the object node's word), the lever arm (the remaining time written into
that word), the menu music toggle (the crossfade at 0.2 and 0.1), the companion
conversion and the AI constructor (the two shares at the same AI fields).

## What changes

- `exa_world::saved_parts::PartTree` on each placed object's and prop's
  spawned root: the model's nodes, their attachments and their spawned
  local transforms; `ClothInstance::start` keeps the cloth's built places.
- `exa_game::level_save::part_records` writes the live records; the save
  writes the mechanisms' live object words (`LiveWord`).
- A companion's rise writes the AI's skill share and confidence.
- `exa_audio::MusicCrossfade` and `flow::Step::MusicCrossfade` carry the
  menu's switch to `AudioEnvironment::music_crossfade`.
