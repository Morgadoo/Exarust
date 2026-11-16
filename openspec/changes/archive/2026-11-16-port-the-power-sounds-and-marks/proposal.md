## Why

Roadmap task B4, triage lane L06a (powers gameplay, lane `powersplay`):
the powers' sounds and their random draws, the Mind powers' bar marks,
the cast failures and a few ends the ledger left partial: Shock's charge
and release, the displacements' start and come-back, the wave's loops,
Force Strikes' swing set, the failure sound of a throw and of a refused
kinesis pick, the barricade's end by event, the cast end's stored stance
and the tree's domain on opening.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly where order and constants
matter:

- Shock's control: the first tick plays the charge set at the hand node's
  place at 0.2 and `0.9 + 0.2·r` (the runtime stream's draw, in double);
  the fire posts the burst and then plays the release set at 1.
- Vanish's and the Long Door's start: `thmdsp_vns` at the body's place,
  0.4, pitch `0.9 + 0.2·r` with the engine generator's draw (the Long Door
  only for a player; the AI's at its grid's found place); each control
  tick moves the class's own voice to the body; the come-back retriggers
  `thmdsp_apr` on it.
- The wave's start allocates the blast's two voices, its charge loop
  started `length·0.8·r` (double) into its buffer: the field the ledger
  read as the voice's gain is the start offset. The wave shares the
  blast's frame (one class slot), which levels both voices.
- Force Strikes' start writes `ThmFrc_Swing` to the body's swing-sound
  slot and 3 to its swing factor, which the swing routine reads as the
  swing voice's set and as a factor of its attack ticks' volume; the end
  writes `SwingHA` and 1 back.
- The throw cast plays `ThaumFail01` when its pick fails and still
  answers 1; the hand's left press, kept for combat or out of reach,
  calls a waiting pick hook with no node and flag 0, and the kinesis's
  hook then plays `ThaumFail01` and ends the power.
- The bar hooks call each running power's slot-15 mark, Mind classes
  included (their table: lit, fade on end, the instant sense's and mask's
  clocks, else the base rule).
- The barricade power's control writes its wall's state 2 whenever the
  power's state is 2, by event or range.
- The cast end starts the body's stored stance over 30 frames, linear.
- Command Dead's and Loyalty's marks call the experience system's party
  removal on their body as they end.
- The tree's open takes the last domain only when the player body is the
  one that opened it last, else the Mind domain.

## What changes

- `exa_game::thaum::barricade_sound::ForceSounded`: a following voice.
- `exa_game::thaum::cast`: Shock's sounds, the displacement sounds, the
  throw's failure, the barricade's wall end, the swing sound sync, the
  Mind marks in the bar's load, the stance on the cast end.
- `exa_game::thaum::kinesis_live::KinesisPicked`: a refused pick.
- `exa_game::thaum::wave_live`, `blast_sound`, `effects::blast`: the
  wave's voices through the blast's sound system (`voice_levels`).
- `exa_game::mind`: `MindClass::slot_mark`, `MindPower::running`, the
  player's marks in `tick_minds`, the marks' party leave.
- `exa_game::power_screen`: the opener and the domain rule.
- `exa_combat::swing_voice::SwingSound` read by the swing; `exa_audio`
  plays the body's swing set.

## Out of scope

The wave's light, stun and ribbon; the kinesis's node walks and the
throw's sub-nodes ([inf] rows without a DEC); the barricade's AI sector;
the decorative shell; the summon's light and sleep
word.
