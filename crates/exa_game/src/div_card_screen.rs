//! The divination card reading as a running screen: a used divination item
//! opens it ([`OpenDivination`]), the reading deals three distinct cards
//! from the sheet, turns them over and holds, and the right button closes
//! it. The screen fills the window and draws the reading's own paint
//! ([`exa_ui::div_card`]) through the GUI draw path.
//!
//! A class-0 kind-5 use opens the reading ([`crate::use_cursor`] on an item
//! panel, [`crate::object_click`] on a taken world object) and passes zero.
//! The original deals from the run-time library's process-wide generator,
//! which runs on between readings; the screen keeps one stream for every
//! reading, seeded with the first reading's word, so each reading deals
//! from where the last one left it. The pointer's buttons reach the
//! reading's mouse slot ([`DivinationClick`]): the right one closes it. The
//! reading paints at the window's GUI unit.

use bevy::audio::PlaybackMode;
use bevy::prelude::*;
use exa_ui::div_card::{self, Reading, SHEET};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};

use crate::arena::Mt19937;
use crate::effects::EffectClock;

/// Open a reading; `seed` seeds the engine's draw.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenDivination {
    pub seed: u32,
}

/// A reading's pointer: the button mask (bit 0 left, bit 1 right).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivinationClick {
    pub mask: u16,
}

/// A shown reading.
#[derive(Debug, Clone)]
pub struct OpenReading {
    pub reading: Reading,
    pub random: Mt19937,
    pub width: i32,
    pub height: i32,
    /// The last paint asked for the `PprMov` sound at this step. The draw
    /// system plays it and clears the flag.
    pub sound_step: bool,
    /// The GUI unit the reading paints at (the window's; 4 until known).
    pub unit: i32,
}

/// The open reading, if one is.
#[derive(Resource, Debug, Default)]
pub struct DivinationScreen {
    pub open: Option<OpenReading>,
    /// The running stream the readings deal from, once one was dealt.
    pub draws: Option<Mt19937>,
}

impl DivinationScreen {
    /// Opens a reading at `now`, dealing three distinct cards from `random`.
    pub fn open(now: u32, seed: u32) -> Self {
        let mut screen = Self::default();
        screen.deal(now, seed);
        screen
    }

    /// Opens a reading at `now`, dealing from the running stream (seeded
    /// with `seed` when no reading dealt before).
    pub fn deal(&mut self, now: u32, seed: u32) {
        let mut random = self.draws.take().unwrap_or_else(|| Mt19937::new(seed));
        let reading = div_card::open(now, &mut |count| random.random(count as i32) as u32);
        self.draws = Some(random.clone());
        self.open = Some(OpenReading {
            reading,
            random,
            width: 0,
            height: 0,
            sound_step: false,
            unit: 4,
        });
    }

    /// Routes a click: the right button closes the reading. Returns whether
    /// the screen is open afterwards.
    pub fn click(&mut self, mask: u16) -> bool {
        if div_card::mouse(mask).is_some() {
            self.open = None;
            return false;
        }
        self.open.is_some()
    }

    /// One paint at `now`, as the draw path's frame. `None` when nothing is
    /// open or the sheet is not loaded.
    pub fn paint(
        &mut self,
        now: u32,
        textures: &GuiTextures,
    ) -> Option<exa_ui::gui_draw::GuiFrame> {
        let open = self.open.as_mut()?;
        let elapsed = now.wrapping_sub(open.reading.start_ms);
        let unit = open.unit.max(1);
        let draws =
            div_card::draw_order(&mut open.reading, open.width.max(0) as u32, unit, elapsed);
        open.sound_step = draws
            .iter()
            .any(|draw| matches!(draw, div_card::CardDraw::Sound));
        let sheet = textures.get(SHEET)?;
        Some(div_card::frame(
            &draws,
            sheet,
            unit,
            open.width.max(0) as u32,
            open.height.max(0) as u32,
        ))
    }
}

pub struct DivinationPlugin;

impl Plugin for DivinationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DivinationScreen>()
            .add_message::<OpenDivination>()
            .add_message::<DivinationClick>()
            // The reading paints after the screens, whose draw starts the
            // frame's GUI queue afresh.
            .add_systems(
                Update,
                (
                    open_reading
                        .after(crate::dialogue_live::paint_dialogue)
                        .after(crate::skill_screen::draw),
                    reading_pointer,
                    click_reading,
                    paint_reading.after(crate::arsenal_view::arsenal_live),
                )
                    .chain()
                    .after(crate::gui_screens::draw_screens)
                    .in_set(exa_physics::FramePhase::Gui),
            );
    }
}

pub(crate) fn open_reading(
    mut opened: MessageReader<OpenDivination>,
    clock: Res<EffectClock>,
    mut screen: ResMut<DivinationScreen>,
    game: Option<Res<exa_assets::Game>>,
    textures: Option<ResMut<GuiTextures>>,
) {
    // One reading at a time, as the original finds its form already open.
    if screen.open.is_some() {
        return;
    }
    if let Some(message) = opened.read().next() {
        screen.deal(clock.now_ms, message.seed);
        info!(
            "divination: the reading opens, cards {:?}",
            screen.open.as_ref().map(|open| open.reading.cards)
        );
        // The form loads its card sheet when it is made; the interface's
        // textures keep it after.
        if let (Some(game), Some(mut textures)) = (game, textures)
            && !textures.textures.contains_key(SHEET)
        {
            match game.load_rgba(SHEET) {
                Ok(image) => {
                    if let Some(texture) = exa_ui::gui_draw::GuiTexture::from_bottom_up(
                        image.width,
                        image.height,
                        &image.pixels,
                    ) {
                        textures.textures.insert(SHEET.to_string(), texture);
                    }
                }
                Err(error) => warn!("divination: the card sheet {SHEET:?}: {error}"),
            }
        }
    }
}

/// The pointer's buttons as the reading's mouse words while it is up: a
/// press is the button bits (left 1, right 2) with 0x1000, a release with
/// 0x2000.
fn reading_pointer(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    screen: Res<DivinationScreen>,
    mut clicks: MessageWriter<DivinationClick>,
) {
    let (Some(mouse), Some(_)) = (mouse, screen.open.as_ref()) else {
        return;
    };
    for (button, bit) in [(MouseButton::Left, 1u16), (MouseButton::Right, 2)] {
        if mouse.just_pressed(button) {
            clicks.write(DivinationClick { mask: bit | 0x1000 });
        }
        if mouse.just_released(button) {
            clicks.write(DivinationClick { mask: bit | 0x2000 });
        }
    }
}

fn click_reading(mut clicks: MessageReader<DivinationClick>, mut screen: ResMut<DivinationScreen>) {
    if screen.open.is_none() {
        return;
    }
    for click in clicks.read() {
        screen.click(click.mask);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_reading(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    clock: Res<EffectClock>,
    textures: Option<Res<GuiTextures>>,
    mut frames: Option<ResMut<GuiFrames>>,
    mut screen: ResMut<DivinationScreen>,
    game: Option<Res<exa_assets::Game>>,
    bank: Option<ResMut<exa_audio::SoundBank>>,
    mut sources: Option<ResMut<Assets<AudioSource>>>,
    mut commands: Commands,
) {
    if let Some(open) = screen.open.as_mut()
        && let Ok(window) = windows.single()
    {
        open.width = window.physical_width() as i32;
        open.height = window.physical_height() as i32;
        open.unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    }
    let Some(textures) = textures.as_deref() else {
        return;
    };
    let Some(frame) = screen.paint(clock.now_ms, textures) else {
        return;
    };
    // The paint's sound step plays `PprMov` once, as the original's paint
    // does at a deal step and the first three turns.
    let play = screen
        .open
        .as_mut()
        .is_some_and(|open| core::mem::take(&mut open.sound_step));
    if play
        && let (Some(game), Some(mut bank), Some(sources)) = (game, bank, sources.as_mut())
        && let Some(clip) = bank.get(&game, sources, div_card::SOUND)
    {
        commands.spawn((
            AudioPlayer(clip),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                ..default()
            },
        ));
    }
    // Pushed beside the other screens' frames; the renderer takes and
    // clears the list each frame.
    if let Some(frames) = frames.as_mut() {
        frames.push(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ui::gui_draw::GuiTexture;

    fn textures() -> GuiTextures {
        let mut textures = GuiTextures::default();
        textures
            .textures
            .insert(SHEET.into(), GuiTexture::solid([120, 80, 40, 255]));
        textures
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<EffectClock>()
            .add_plugins(DivinationPlugin);
        app
    }

    #[test]
    fn a_reading_deals_three_distinct_cards_and_holds() {
        let mut screen = DivinationScreen::open(1_000, 7);
        let open = screen.open.as_ref().unwrap();
        let [a, b, c] = open.reading.cards;
        assert!(a < 22 && b < 22 && c < 22);
        assert!(a != b && b != c && a != c);
        assert_eq!(open.reading.start_ms, 1_000);
        // The paint advances the phases by the reading's clock.
        let textures = textures();
        screen.open.as_mut().unwrap().width = 800;
        screen.open.as_mut().unwrap().height = 600;
        assert!(screen.paint(1_100, &textures).is_some());
        // The deal step asked for the paper sound; the draw system plays it
        // once and clears the flag.
        assert!(screen.open.as_ref().unwrap().sound_step);
        assert!(screen.paint(6_000, &textures).is_some());
        assert_eq!(
            screen.open.as_ref().unwrap().reading.phase,
            div_card::phase::HOLD
        );
    }

    #[test]
    fn the_right_button_closes_and_the_left_one_does_not() {
        let mut screen = DivinationScreen::open(0, 1);
        assert!(screen.click(1));
        assert!(screen.open.is_some());
        assert!(!screen.click(2));
        assert!(screen.open.is_none());
        assert!(screen.paint(10, &textures()).is_none());
    }

    #[test]
    fn the_open_message_starts_one_reading_and_the_frame_is_drawn() {
        let mut app = app();
        app.world_mut().resource_mut::<EffectClock>().now_ms = 500;
        app.world_mut().write_message(OpenDivination { seed: 3 });
        app.update();
        let screen = app.world().resource::<DivinationScreen>();
        let open = screen.open.as_ref().unwrap();
        assert_eq!(open.reading.start_ms, 500);
        assert_eq!(open.reading.cards.len(), 3);
        // The frame needs the sheet: without loaded textures nothing draws.
        assert!(
            app.world()
                .resource::<DivinationScreen>()
                .open
                .as_ref()
                .is_some()
        );
        app.world_mut().write_message(DivinationClick { mask: 2 });
        app.update();
        assert!(app.world().resource::<DivinationScreen>().open.is_none());
    }

    /// The readings deal from one running stream: the second reading goes
    /// on from where the first left it instead of dealing the first cards
    /// again.
    #[test]
    fn readings_deal_from_the_running_stream() {
        let mut app = app();
        let mut dealt = Vec::new();
        for _ in 0..4 {
            app.world_mut().write_message(OpenDivination { seed: 0 });
            app.update();
            let mut screen = app.world_mut().resource_mut::<DivinationScreen>();
            dealt.push(screen.open.as_ref().unwrap().reading.cards);
            screen.click(2);
        }
        let mut expected = Mt19937::new(0);
        for cards in &dealt {
            let reading = div_card::open(0, &mut |count| expected.random(count as i32) as u32);
            assert_eq!(*cards, reading.cards);
        }
        assert!(dealt.windows(2).any(|pair| pair[0] != pair[1]), "{dealt:?}");
    }

    /// The right button's press closes an open reading through the mouse
    /// slot; the left one does not.
    #[test]
    fn the_right_press_closes_the_reading() {
        let mut app = app();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.world_mut().write_message(OpenDivination { seed: 0 });
        app.update();
        let press = |app: &mut App, button: MouseButton| {
            let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse.clear();
            mouse.press(button);
        };
        press(&mut app, MouseButton::Left);
        app.update();
        assert!(app.world().resource::<DivinationScreen>().open.is_some());
        press(&mut app, MouseButton::Right);
        app.update();
        assert!(app.world().resource::<DivinationScreen>().open.is_none());
    }

    /// The reading paints at the GUI unit it is given.
    #[test]
    fn the_reading_paints_at_its_unit() {
        let textures = textures();
        let mut frames = Vec::new();
        for unit in [4, 8] {
            let mut screen = DivinationScreen::open(0, 7);
            let open = screen.open.as_mut().unwrap();
            open.width = 1600;
            open.height = 1080;
            open.unit = unit;
            frames.push(screen.paint(6_000, &textures).unwrap());
        }
        assert_ne!(
            format!("{:?}", frames[0].layers),
            format!("{:?}", frames[1].layers)
        );
    }
}
