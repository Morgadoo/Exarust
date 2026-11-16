//! The arena's NEW CAREER at the front end: the company dialog.
//!
//! The main menu's NEW CAREER closes the menu (the show's close: the menu
//! form hides and the version line goes), opens the company dialog (the
//! company name and the emblem generator, built by
//! [`crate::hub_windows::HubWindows::open_company_with`]) with the emblem
//! the last NEXT stored when its background word is above 0, centres the
//! pointer on the dialog's 370-scaled line ([`crate::emblem::company_cursor`]),
//! focuses the name field and sets the session state to the setup forms'
//! (0x40).
//!
//! BACK runs behind the transition: at black the dialog is freed and the
//! game goes back to the title ([`crate::flow::Flow::company_back`]). NEXT
//! refuses an empty name with the "Company Name" box; a name is stored with
//! the emblem record ([`CompanyDraft`], the original's two globals that the
//! arena's new-career start reads), character creation starts in the
//! arena's mode (the creator answers [`MenuCommand::NewCareer`]) and, at
//! black, the dialog is freed and the session state becomes the creator's
//! ([`crate::flow::Flow::company_named`]). The creator's own reset and the
//! sound and material releases that follow are the creator's (DEC-3).
//!
//! The dialog's forms, their records and callbacks are the hub forms'
//! ([`crate::option_forms`]); this unit opens them from the menu and runs
//! their way out.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::flow::{Flow, FlowSteps, Phase};
use crate::hub_forms::FormWindow;
use crate::hub_windows::{ArenaCareer, HubWindows};
use crate::menu_pages::{MenuCommand, session_state};
use crate::menu_screen::{MenuChoice, MenuScreen};
use crate::option_forms::CompanyExit;

/// The company name and emblem record the last NEXT stored: the original
/// keeps them in two globals, which the next dialog restores (the emblem,
/// when its background word is above 0) and the arena's new-career start
/// reads.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct CompanyDraft {
    pub name: String,
    pub emblem: exa_ui::EmblemRecord,
}

/// The front end's company dialog: whether it is up, and the free its
/// transition's step asked for ([`crate::flow::Step::CloseCompany`]).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompanyDialog {
    pub open: bool,
    pub close: bool,
}

/// Opens the company dialog from the menu and runs its way out.
pub struct CompanyDialogPlugin;

impl Plugin for CompanyDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CompanyDraft>()
            .init_resource::<CompanyDialog>()
            .init_resource::<HubWindows>()
            .init_resource::<ArenaCareer>()
            .add_message::<MenuChoice>()
            .add_message::<crate::os_pointer::WarpPointer>()
            .add_systems(
                Update,
                (open_from_menu, run_exit, free_dialog)
                    .chain()
                    .after(crate::arena_hub_live::HubWindowSystems)
                    .before(crate::hub_windows::draw_hub_windows)
                    .in_set(exa_physics::FramePhase::Hub),
            );
    }
}

/// The menu's NEW CAREER: the menu closes as before a show, the dialog
/// opens, the pointer moves to its line and the session state becomes the
/// setup forms'.
#[allow(clippy::too_many_arguments)]
fn open_from_menu(
    mut choices: MessageReader<MenuChoice>,
    mut windows: ResMut<HubWindows>,
    mut dialog: ResMut<CompanyDialog>,
    draft: Res<CompanyDraft>,
    mut menu: Option<ResMut<MenuScreen>>,
    mut flow: Option<ResMut<Flow>>,
    mut overlays: Option<ResMut<crate::view_overlay::ViewOverlays>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut warp: MessageWriter<crate::os_pointer::WarpPointer>,
) {
    if !choices
        .read()
        .any(|MenuChoice(command)| *command == MenuCommand::CompanyDialog)
    {
        return;
    }
    // The show's close: the menu form hides and the version line goes.
    if let Some(menu) = menu.as_mut() {
        menu.close();
    }
    if let Some(overlays) = overlays.as_mut() {
        overlays.title = false;
    }
    if let Some(flow) = flow.as_mut() {
        flow.menu_open = false;
        flow.phase = Phase::from_word(session_state::SETUP);
    }
    windows.forms.remove(FormWindow::Emblem);
    windows.forms.remove(FormWindow::Company);
    windows.forms.options.clear();
    windows.open_company_with(Some(draft.emblem));
    windows.forms.options.company_front_end = true;
    dialog.open = true;
    dialog.close = false;
    if let Ok(window) = window.single() {
        let screen = (
            window.physical_width() as i32,
            window.physical_height() as i32,
        );
        let [x, y] = crate::emblem::company_cursor(screen);
        let scale = window.scale_factor().max(f32::EPSILON);
        warp.write(crate::os_pointer::WarpPointer(Vec2::new(
            x as f32 / scale,
            y as f32 / scale,
        )));
    }
    info!("company dialog: NEW CAREER opens the company dialog over the front end");
}

/// Runs the dialog's BACK or NEXT once its button asked for it: behind the
/// transition, as the original's buttons do. Without the game flow (a host
/// with no front end) the dialog closes at once.
fn run_exit(
    windows: Res<HubWindows>,
    mut dialog: ResMut<CompanyDialog>,
    mut draft: ResMut<CompanyDraft>,
    mut flow: Option<ResMut<Flow>>,
    mut steps: Option<ResMut<FlowSteps>>,
    mut choices: MessageWriter<MenuChoice>,
) {
    let Some(exit) = windows.forms.options.company_exit.clone() else {
        return;
    };
    // Asked once: the dialog takes no more buttons, and the transition's
    // step frees it.
    if dialog.close || flow.as_ref().is_some_and(|flow| flow.transition_running()) {
        return;
    }
    let named = match exit {
        CompanyExit::Back => false,
        CompanyExit::Named { name, emblem } => {
            info!("company dialog: {name:?} is stored with its emblem; the creator starts");
            draft.name = name;
            draft.emblem = emblem;
            choices.write(MenuChoice(MenuCommand::NewCareer));
            true
        }
    };
    match (flow.as_mut(), steps.as_mut()) {
        (Some(flow), Some(steps)) => {
            let fade = if named {
                flow.company_named()
            } else {
                flow.company_back()
            };
            // The transition is up from here, so the exit runs once.
            steps.steps.extend(fade);
        }
        _ => {
            dialog.close = true;
        }
    }
}

/// Frees the dialog once the transition's step asked for it.
fn free_dialog(mut windows: ResMut<HubWindows>, mut dialog: ResMut<CompanyDialog>) {
    if !dialog.close {
        return;
    }
    dialog.close = false;
    dialog.open = false;
    windows.forms.remove(FormWindow::Emblem);
    windows.forms.remove(FormWindow::Company);
    windows.forms.options.clear();
    info!("company dialog: freed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::Mode;
    use crate::menu_pages::Page;
    use exa_ui::emblem_form::{company, records as emblem_records};

    /// The menu, the flow with its transition screen, the hub's forms and
    /// this plugin, on a 16 ms frame, at the arena page of the title.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                crate::flow::FlowPlugin,
                CompanyDialogPlugin,
            ))
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .init_resource::<exa_physics::sim_app::SimPause>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ));
        app.update();
        {
            let mut windows = app.world_mut().resource_mut::<HubWindows>();
            windows.task.resize((1280, 720));
            windows.unit = 4;
        }
        app
    }

    fn new_career(app: &mut App) {
        // The arena page's first row, as the menu's click hands it over.
        let command = crate::menu_pages::menu_command(
            Page::Start,
            1,
            crate::menu_pages::game_mode::MENU,
            session_state::MAIN_MENU,
            false,
        );
        assert_eq!(command, MenuCommand::CompanyDialog);
        app.world_mut()
            .resource_mut::<Messages<MenuChoice>>()
            .write(MenuChoice(command));
        app.update();
    }

    fn press(app: &mut App, record: usize) {
        app.world_mut()
            .resource_scope(|world, mut career: Mut<ArenaCareer>| {
                world.resource_mut::<HubWindows>().answer(
                    &mut career,
                    FormWindow::Company,
                    record,
                    true,
                );
            });
    }

    fn name(app: &mut App, text: &str) {
        let mut windows = app.world_mut().resource_mut::<HubWindows>();
        let form = windows.forms.get_mut(FormWindow::Emblem).unwrap();
        form.set_text(emblem_records::NAME, text);
    }

    fn dialog_up(app: &App) -> bool {
        let windows = app.world().resource::<HubWindows>();
        windows.forms.get(FormWindow::Company).is_some()
            && windows.forms.get(FormWindow::Emblem).is_some()
    }

    fn run_out(app: &mut App) {
        for _ in 0..400 {
            app.update();
            if !app.world().resource::<Flow>().transition_running() {
                return;
            }
        }
        panic!("the transition never closed");
    }

    fn creator_asks(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<MenuChoice>>()
            .drain()
            .filter(|MenuChoice(command)| *command == MenuCommand::NewCareer)
            .count()
    }

    /// NEW CAREER closes the menu and opens the company dialog over the
    /// front end in the setup state, the name field focused; BACK leaves
    /// the dialog up until the transition's black, which frees it and goes
    /// back to the title.
    #[test]
    fn new_career_opens_the_company_dialog_and_back_fades_to_the_title() {
        let mut app = app();
        new_career(&mut app);
        assert!(!app.world().resource::<MenuScreen>().open);
        assert!(dialog_up(&app));
        let flow = app.world().resource::<Flow>();
        assert_eq!(flow.phase, Phase::from_word(session_state::SETUP));
        assert!(!flow.menu_open);
        let windows = app.world().resource::<HubWindows>();
        assert!(windows.forms.options.company_front_end);
        assert_eq!(
            windows.forms.options.key_focus,
            Some((FormWindow::Emblem, emblem_records::NAME))
        );
        assert!(app.world().resource::<CompanyDialog>().open);

        press(&mut app, company::BACK);
        app.update();
        assert!(app.world().resource::<Flow>().transition_running());
        assert!(dialog_up(&app), "the dialog stays up until black");
        // A second press while the screen fades asks nothing more.
        press(&mut app, company::NEXT);
        run_out(&mut app);
        assert!(!dialog_up(&app));
        assert!(!app.world().resource::<CompanyDialog>().open);
        assert_eq!(creator_asks(&mut app), 0);
        let flow = app.world().resource::<Flow>();
        assert_eq!(flow.mode, Mode::Menu);
        assert_eq!(flow.phase, Phase::Menu);
        let menu = app.world().resource::<MenuScreen>();
        assert!(menu.open);
        assert_eq!(menu.page, Page::Title);
    }

    /// NEXT with an empty name raises the "Company Name" box and keeps the
    /// dialog; with a name it stores the name and the emblem, asks the
    /// creator for the arena's new career and, at black, frees the dialog
    /// with the session state at the creator's. The next dialog restores
    /// that emblem.
    #[test]
    fn next_names_the_company_and_starts_the_creator() {
        let mut app = app();
        new_career(&mut app);
        press(&mut app, company::NEXT);
        app.update();
        assert!(!app.world().resource::<Flow>().transition_running());
        assert!(app.world().resource::<HubWindows>().forms.message_on_top());
        assert_eq!(creator_asks(&mut app), 0);

        let mut app = app_with_named("Ash");
        let draft = app.world().resource::<CompanyDraft>().clone();
        assert_eq!(draft.name, "Ash");
        assert!(draft.emblem.background > 0);
        assert_eq!(
            app.world().resource::<ArenaCareer>().company.name,
            "Ash".to_string()
        );
        assert_eq!(creator_asks(&mut app), 1);
        assert!(app.world().resource::<Flow>().transition_running());
        run_out(&mut app);
        assert!(!dialog_up(&app));
        assert_eq!(app.world().resource::<Flow>().phase, Phase::Creating);

        // The next NEW CAREER restores the stored emblem.
        let mut edited = draft.emblem;
        edited.background = 7;
        edited.design = 9;
        app.world_mut().resource_mut::<CompanyDraft>().emblem = edited;
        new_career(&mut app);
        let windows = app.world().resource::<HubWindows>();
        let editor = &windows.forms.options.emblem.as_ref().unwrap().editor;
        assert_eq!(editor.record().background, 7);
        assert_eq!(editor.record().design, 9);
    }

    fn app_with_named(text: &str) -> App {
        let mut app = app();
        new_career(&mut app);
        name(&mut app, text);
        press(&mut app, company::NEXT);
        app.update();
        app
    }
}
