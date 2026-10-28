# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), the gamemain2 and gamegui2
paragraphs: the game main unit's flow (session begin, continue and leave,
back to the title, the story end-menu exit, the practice start) and the
menu text shader were tested rules whose steps were only logged. Only the
page state and F1 ran in the game; nothing applied the steps, the end menu
was not drawn, and the menu rows were blitted in the base font.

## What Changes

- `exa_game::flow::FlowSteps` collects every step the flow returns and
  `apply_steps` applies the ones the app owns: the menu and end-menu forms,
  the title logo, the game view (HUD gate), controls, the practice heal's
  health, the music and the quit; a level load's first app-owned steps
  (the menu form hides, the end menu closes) run, the load itself stays
  the host's.
- `MenuChoice` carries the main menu's session commands to the flow: the
  resume, a practice start, the story start (Exarust's unknown background)
  and the leave to the title.
- `exa_game::menu_screen` paints the page through the GUI draw path, the
  selected row through `exa_ui::skin::SkinMode::MenuText`
  (`exa_ui::gui_draw::menu_text_fragment`, `menu_batch`), with the menu
  font's `titlefont.rfi` sheet and a base-font fallback.
- `exa_game::end_menu_screen` draws the end-of-session menu over the game
  and runs its rows through the original's end-session dispatch.
- The splash plays `BME Critters` and queues the menu theme and hand-over.
- Review fixes: the end-menu dispatch branches on the mode first (practice's
  RESTART leaves to the menu form, the second rows return to the title
  behind the transition, the arena's third row does nothing); the session
  begin no longer hides the menu form (the level load does); the boot and
  the hand-over no longer write the title logo; a launched level runs
  without the practice round hook, so its corpses do not raise NEXT FIGHT.

## Evidence

The end-session dispatch read from the assembly (mode word compare, row
compares, the menu-number compare on the first row only, and the
transition calls with their actions); the session begin and continue
routines (only the continue hides the menu form); the level loader's first
calls (hide the menu form, close the end menu and prelude); the boot,
hand-over and back-to-title routines (only the last writes the logo flag);
the end-menu builder's rows and scales.

## Out of scope

The in-process level reload (a practice start from the title, a next fight,
a teleporter and the story respawn change the flow state but keep the
loaded level); live saves; the world, camera and character steps; the
manual, settings, creator and character screen; the arena session bridge
behind the arena end menu; the transition screen's drawing; the
transition's fade flag; a scripted mode's end-menu handler.
