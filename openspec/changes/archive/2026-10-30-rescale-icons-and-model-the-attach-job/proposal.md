# Proposal

## Why

docs/PLAN.md Phase 4 (Combat and items), the item windows and the
equipment cluster (round 40, lane itemgui5). The item icon atlas stayed at
the default forty-times-four side while the pack's icons followed the
interface scale; the model-attach routines' decisions (light, shadow mark,
part clear, the job they leave) and the take-in job's readiness wait had no
Rust counterpart; the card facts did not carry the socket row's words; and
the protection figure multiplied in a different order from the original.

Evidence: the icon manager's unit-change hook, the weapon and shield model
attach routines, the take-in job, the scheduled take-out, the job queue's
runner (a job that returns true stays queued) and the protection routine,
read in REA (Ghidra listing) and the objdump listing; the constants 40,
0.5, 0.01 and 0.005 read from the binary's data.

## What Changes

- `exa_game::item_panel_draw::PanelIcons::rescale`, run live by
  `exa_game::item_icon_render::rescale_icons` from `exa_ui::GuiScale`: a
  new unit rebuilds the atlas at forty units a side and queues every cached
  icon for a new render.
- `exa_items::doll`: `AttachJob`, `weapon_attach` (lit, shadow mark, part
  clear, take-in job), `shield_attach` (part clear, node finaliser),
  `take_in_ready` (the job waits for the node's and the part's marks),
  `scheduled_take_out`.
- `exa_items::catalogue::CardFacts::socket_words` from
  `exa_items::item_gui::Sockets::of_section`.
- `exa_items::instance::protection_value`: the original's multiply order.

## Out of scope

- Drawing the socket row on the live card (needs the crystal sprite sheet
  and the per-cell draw size).
- The attach routines' scene, sector and physics steps; nothing calls the
  attach decisions live yet.
- The icon scene's bit-31 mesh subdivision and streamed icons.
