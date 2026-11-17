## Why

Roadmap task B4, lane `worldobj`: the triage's port lanes L16 (world
objects, script and audio) and L17 (runtime and platform). The door orders
discarded the lock sounds and the group's latch word, the lever never kept
its armed return in its object word, the arena's physician fixup removed a
"weapon" at the wrong spot where the original unlatches the physician's
door, the force effects cut their one-shots at their end, the scripts read
the session clock instead of the world clock and never saw the locale or
the skills, a kind-4 device's revelation never ran, the Proctor's Seal kept
a session flag where the original sets a persisted unlock bit, the journey
never released the controls, a static placement marked only the object's
root, and story levels played every track at one volume.

## Evidence

REA (Exanima.exe 0.9.5), the assembly read where constants and order
matter: the door open, close-and-lock, toggle and latch routines; the
lever's arm and timed return (the remaining time divided by 100, unsigned
at the arm and signed in the tick, into the high half of the object word);
the arena's spot finder (an object node whose record has the door section,
within 10 cm on x and z of (-2100, 900) cm) and its caller; the voice
finish (a playing one-shot is marked auto-free, the caller's word
cleared); the WorldTime builtin (the save unit's clock getter) and
InLocale (the current locale word); the object use's kind-4 branch and the
revelation (phase word 3, event 0xDA0115, power 0x10F); the id 0x24c
branch and the settings write it calls; the selection screen's journey
tick and the controls' release it calls; the create's recursive level
geometry mark under placement bit 0x10; the root chunk loops (pos + 8 <
size); the dialogue graph's node table (only a zero slot is the shared
stub, whose size word is 4); the story level hook's track and volume table.

## What Changes

- Door orders play the lock set the original plays and keep the latch bit.
- The lever's object word carries an armed return.
- The arena unlatches the physician's door; the weapon query is removed.
- Force effects finish their voices.
- Scripts read the world clock, the locale and the skill points.
- A kind-4 device's use runs the Raise-Dead revelation.
- Id 0x24c's use sets the persisted background unlock bit 2.
- The journey's level change and end release the controls.
- A static placement marks the whole object tree level geometry.
- Root chunk loops read only past 8 bytes; a short node slot steps by its size.
- Story levels play their track at the hook's volume.

## Impact

Lockstep: the nav grid stamps the bodies under statically placed objects
(scenes with such objects change their grid and so their AI traces). The
other changes touch sounds, settings and script values.
