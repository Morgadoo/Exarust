## Why

Roadmap task B4, lane `saves3`: the save unit's remaining rows. Finding
F415: the running game never set the header's camera pair, so every save
carried the origin and zoom 1. The journal wrote each page's index as its
id (a guess). A door's arrival framed the pelvis at the pose's own target
over the probed feet instead of the landing's own word and lift, and left
the body's state as it was. The arena's match return read the hub from the
archive, where the original prefers the slot's saved hub.

## Evidence

REA (Exanima.exe 0.9.5), the assembly:

- The story and arena writers copy 12 bytes at the viewport's camera node
  +0x64 (the translation row of its 4×3 matrix at +0x40, the matrix the
  screen-ray routine transforms by) and 4 bytes of the nav controller's
  current zoom (the field the zoom ease writes from the wheel's target).
  Both readers read the 16 bytes into stack scratch that nothing reads
  again. A retail story save holds an eye about 20 m from the player and
  zoom 1280.0035.
- The journal is a dynamic array of {id, title, text}. Only the writer
  reads the id. The reader stores the saved word; the default page, the
  window's opening page and the erase's reset write 0; the add grows the
  array (zero fill) and writes only the title; the erase shifts the later
  entries with their words. The retail save's only page has id 0.
- The save landing writes the pelvis-target word as
  (single)(108.18 − record height × 11.1) in double, starts the stance
  motion with no blend, sets the capture key to −1, both tones to 1.0, the
  condition word, its flags and the locomotion flags to 0, probes 300 cm
  down from 100 cm up, writes both feet's ground words, lifts the frame by
  the word plus 13 (combat-mode byte set) or 5, then the living placement
  and the held items' re-place.
- The match return (`settle_return`) loads `arenahub` through the
  save-or-archive loader with its save flag set: the package's section by
  name, the archive only when absent.

## What changes

- `save_live::follow_the_view` keeps `SaveCamera` on the chase camera's
  eye (cm) and shown zoom (`ChaseCamera::shown_zoom`); both writers stamp it.
- `JournalEntry::id` and `story::Entry::id` carry the page word through the
  save and the load.
- `save_level::landing_floor`, `landing_resets`, `landing_height` and
  `limp::FrameHeight::Over`; the viewer runs them for a door's arrival.
- `prefer_saved_section` takes the arena slot's `arenahub` section on the
  match return.

## Impact

- Visible: none in a frame for the camera and the ids (written only). A
  door's arrival lands the pelvis 108.18 − 11.1·h + 5 cm over the floor
  (13 in combat mode) with a fresh stance clip; the hub after a match shows
  what the slot saved.
- Owed grid scenes (GPU on hold): a door from exanima01 to exanima02 in and
  out of combat mode (the landing height and stance); an arena match won
  and returned (`--arena-auto`), with an item dropped in the hub before the
  match's save; a story save's header read back (`exa-tool`).
- Lockstep: door arrivals change their first ticks (pelvis height, stance
  restart); the story traces through a door change. The hub return can
  change the hub's contents.
