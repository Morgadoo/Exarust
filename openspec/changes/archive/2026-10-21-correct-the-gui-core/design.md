# Design

## Approach

Each rule was re-read in the instruction listing, not only the pseudo-code,
wherever floats, shifts or comparisons decide the result: `CVTSS2SI` and
`CVTSD2SI` round half to even under the default mode; `SHR` on a 64-bit
difference is an unsigned shift; the camera's distance clamp is two
unordered compares, so a not-a-number distance passes both; the still's
share is a double division narrowed to single. The data the routines read
was taken from the binary (`read_bytes`): the menu font's design record and
its texture name, the GUI lengths' initial words, the eight supersampling
levels and the jitter table (checked point by point against the Rust
table), the seven class records with their default margins.

The window manager is modelled on the task's own fields: a focused form and
control (none is `i32::MIN`), a tooltip request (form, control, stamp), the
pointer's target, press point, held word, consumed buttons, capture and the
drop hand-over. The forms' virtual methods become `FormHost`'s: hit test,
select, press, release, drag move, drop over, drop, drop cancelled, close,
plus the gamepad's and the tooltip's questions with neutral defaults. The
root's hit descent and a form's activation belong to the forms' unit; the
task models them minimally (the last child first; activation as a focus
change) so it can stand alone, and says so.

The main menu's click handler takes the two words it reads separately: the
game mode (practice, story, arena, the versus lobby) and the session state
(main menu, playing, won, ended, round won). Its commands are named for
what they open, and the running screen performs only its own page work.

## Open questions

- The menu font's texture: the record names `titlefont.rfi`; the menu's
  constructor also loads `mmenutxt01.rfi`, read by none of these routines.
  A capture of the menu's draw would show which sheet the rows use.
- The forms' coordinate translation for the modal hit, the activation call,
  the gamepad focus test and the tooltip form's show: outside these units;
  answered when the forms' unit is read (REA on the form class).
- The record fields a tooltip reads for its text (a number or the hint
  string) and the tooltip classes' factories: the tooltip unit.
- The meaning of the main menu's 45-pixel constructor field, unread here.

## Docs

`docs/UI.md` (Text drawing, The main menu screen, The window manager, Text
editing, Controls), `docs/RENDERING.md` (Viewport still images), the
`exa_ui` and `exa_game` READMEs.
