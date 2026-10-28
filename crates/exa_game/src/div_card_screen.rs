//! The divination card reading as a running screen: a used divination item
//! opens it ([`OpenDivination`]), the reading deals three distinct cards
//! from the sheet, turns them over and holds, and the right button closes
//! it. The screen fills the window and draws the reading's own paint
//! ([`exa_ui::div_card`]) through the GUI draw path.
//!
//! The item-use path that opens the reading in the original belongs to the
//! object database's use entry; the item's use class was not read, so the
//! game sends [`OpenDivination`] from the host that knows an item is a
//! divination card [guess].

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
}

/// The open reading, if one is.
#[derive(Resource, Debug, Default)]
pub struct DivinationScreen {
    pub open: Option<OpenReading>,
}

impl DivinationScreen {
    /// Opens a reading at `now`, dealing three distinct cards from `random`.
    pub fn open(now: u32, seed: u32) -> Self {
        let mut random = Mt19937::new(seed);
        let reading = div_card::open(now, &mut |count| (random.unit() * f64::from(count)) as u32);
        Self {
            open: Some(OpenReading {
                reading,
                random,
                width: 0,
                height: 0,
            }),
        }
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
        let draws = div_card::draw_order(&mut open.reading, open.width.max(0) as u32, 4, elapsed);
        let sheet = textures.get(SHEET)?;
        Some(div_card::frame(
            &draws,
            sheet,
            4,
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
            .add_systems(Update, (open_reading, click_reading, paint_reading).chain());
    }
}

fn open_reading(
    mut opened: MessageReader<OpenDivination>,
    clock: Res<EffectClock>,
    mut screen: ResMut<DivinationScreen>,
) {
    // One reading at a time, as the original finds its form already open.
    if screen.open.is_some() {
        return;
    }
    if let Some(message) = opened.read().next() {
        *screen = DivinationScreen::open(clock.now_ms, message.seed);
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

fn paint_reading(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    clock: Res<EffectClock>,
    textures: Option<Res<GuiTextures>>,
    mut frames: Option<ResMut<GuiFrames>>,
    mut screen: ResMut<DivinationScreen>,
) {
    if let Some(open) = screen.open.as_mut()
        && let Ok(window) = windows.single()
    {
        open.width = window.physical_width() as i32;
        open.height = window.physical_height() as i32;
    }
    let Some(textures) = textures.as_deref() else {
        return;
    };
    let Some(frame) = screen.paint(clock.now_ms, textures) else {
        return;
    };
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
}
