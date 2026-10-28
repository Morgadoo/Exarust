# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), the game flow paragraph:
the flow ran every world change (the quit to the title, the story respawn,
the revival, level exits, the next practice fight, RESTART) a fixed 100 ms
after asking for it, with no screen. Checked against the game-main unit's
transition helper and the transition screen's run routine, the 100 is the
priority the screen's run task takes in the GUI task list: the action is
the screen's callback and runs when the screen has faded to black, after
600 ms or 2000 ms. The screen, its music fade and its hold were not live.

## What Changes

- The flow keeps one `exa_ui::transition::TransitionScreen` per transition
  and `Flow::transition_tick` steps it on the real clock every frame,
  whatever covers the game; the action runs at black.
- Each caller passes the original's music-fade argument; the level exit
  passes the story's level-entry music check for its destination; the exit's
  fade bit sets the screen's long hold, which the open leaves alone.
- With the music fade the main track's gain falls with the screen, the track
  stops and forgets its name at black and the screen's audio fade drops to 0
  and rises with the fade back in; the world fade resets at black.
- An action that changes the level holds the screen black until the change
  is done (bounded wait when no change shows up).
- The class cancel's end hook runs inside the action, at black.
- The session frame task runs in every phase but the transition's, as the
  original's task tests the phase word against zero.
- `transition_screen` draws the screen's alpha as a black form over the game.
- `exa_audio`: the main voice's gain accessors and the entry check against
  Exarust's lowercase track key.

## Evidence

The transition helper stores the action on the screen form, sets the music
fade and short-hold bytes from its arguments, clears the phase, alpha and
saved music words, shows the form and inserts the run routine into the GUI
task list with priority 100 (a sorted insert). The run routine fades by
1/600 (1/2000 with the long hold, the byte the level exit's fade bit sets
before it opens the screen), calls the action once the level passes 1,
holds two frames and 300/500/1000 ms, fades in by 1/600 and closes. Each
of the 52 call sites' arguments was read from the assembly.

## Out of scope

The screen's close also clears four engine words whose roles are not
identified; the "TO BE CONTINUED" hold has no caller in the running game;
the short hold and the long wait have no caller in this unit.
