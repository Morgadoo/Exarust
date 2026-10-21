//! A debug helper (`--debug-path x,y,z`): the planner
//! (`exa_world::nav::find_path_with`) run from the player's feet toward a
//! fixed point with a walker of size class 3 and the penalties doubled,
//! stopping at the goal's cell, and the player's own footprint off the grid
//! while it plans (as the original's AI lifts its own before it plans; the
//! size class and shift are those the earlier reading of the AI planner
//! gave). It plans again whenever the player has moved more than a metre
//! from where it last planned, logs each plan and draws the path: yellow
//! when it reaches the goal, orange when it ends short, a ring at each
//! point, a cyan cross at the goal. Nothing walks the path. Idle (no system
//! runs) without the flag.

use bevy::prelude::*;
use exa_combat::Fighter;
use exa_physics::CharacterBody;
use exa_world::nav::{ACCEPT_GOAL_CELL, NavFootprints, NavGrid, Path, find_path_with};

use crate::footprints::keep_footprints;
use crate::player::PlayerBody;

pub struct PathProbePlugin;

impl Plugin for PathProbePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PathProbe>()
            .add_systems(
                FixedPreUpdate,
                plan_probe
                    .after(keep_footprints)
                    .run_if(|p: Res<PathProbe>| p.goal.is_some()),
            )
            .add_systems(
                Update,
                draw_probe.run_if(|p: Res<PathProbe>| p.goal.is_some()),
            );
    }
}

/// The walker's size class and the penalty shift the probe plans with.
pub const PROBE_SIZE: u8 = 3;
pub const PROBE_SHIFT: u32 = 1;
/// The player plans again once it is this far (m) from where it last
/// planned.
const REPLAN_AFTER: f32 = 1.0;
/// Lines float this far over the floors (m); rings and the cross, half
/// sizes.
const LIFT: f32 = 0.06;
const RING: f32 = 0.06;
const CROSS: f32 = 0.15;

/// The probe's goal and its last plan.
#[derive(Resource, Debug, Clone, Default)]
pub struct PathProbe {
    /// Where the player plans to (m); none: the probe is off.
    pub goal: Option<Vec3>,
    /// Where the player stood at its last plan.
    from: Option<Vec3>,
    plan: Option<Path>,
}

impl PathProbe {
    /// A probe planning toward `goal` (m).
    pub fn to(goal: Vec3) -> Self {
        Self {
            goal: Some(goal),
            ..default()
        }
    }

    /// The last plan, if any.
    pub fn plan(&self) -> Option<&Path> {
        self.plan.as_ref()
    }
}

fn plan_probe(
    mut probe: ResMut<PathProbe>,
    grid: Option<ResMut<NavGrid>>,
    mut prints: ResMut<NavFootprints>,
    players: Query<(Entity, &ChildOf), (With<PlayerBody>, With<Fighter>)>,
    roots: Query<(&CharacterBody, &Transform)>,
) {
    let (Some(goal), Some(mut grid)) = (probe.goal, grid) else {
        return;
    };
    let Some((who, standing)) = players.iter().find_map(|(who, parent)| {
        let (capsule, at) = roots.get(parent.parent()).ok()?;
        Some((who, capsule.feet(at)))
    }) else {
        return;
    };
    if probe
        .from
        .is_some_and(|from| from.distance(standing) <= REPLAN_AFTER)
    {
        return;
    }
    let had = prints.of(who).is_some();
    prints.lift(&mut grid, who);
    let plan = find_path_with(
        &grid,
        standing,
        goal,
        PROBE_SIZE,
        ACCEPT_GOAL_CELL,
        PROBE_SHIFT,
    );
    if had {
        prints.stamp(&mut grid, who, standing);
    }
    info!(
        "debug path from ({:.3}, {:.3}, {:.3}) to ({:.3}, {:.3}, {:.3}): {} points, cost {}, {}",
        standing.x,
        standing.y,
        standing.z,
        goal.x,
        goal.y,
        goal.z,
        plan.points.len(),
        plan.cost,
        if plan.reached {
            "reached"
        } else {
            "ends short"
        }
    );
    for (k, p) in plan.points.iter().enumerate() {
        info!("  {k:>3}: ({:.3}, {:.3}, {:.3})", p.x, p.y, p.z);
    }
    probe.from = Some(standing);
    probe.plan = Some(plan);
}

fn draw_probe(probe: Res<PathProbe>, mut gizmos: Gizmos) {
    let flat = |at: Vec3| Isometry3d::new(at, Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
    let up = Vec3::Y * LIFT;
    if let Some(g) = probe.goal {
        let cyan = Color::srgb(0.0, 1.0, 1.0);
        gizmos.line(g + up - Vec3::X * CROSS, g + up + Vec3::X * CROSS, cyan);
        gizmos.line(g + up - Vec3::Z * CROSS, g + up + Vec3::Z * CROSS, cyan);
    }
    let Some(plan) = probe.plan.as_ref() else {
        return;
    };
    let colour = if plan.reached {
        Color::srgb(1.0, 0.9, 0.2)
    } else {
        Color::srgb(1.0, 0.45, 0.0)
    };
    gizmos.linestrip(plan.points.iter().map(|&p| p + up), colour);
    for &p in &plan.points {
        gizmos.circle(flat(p + up), RING, colour);
    }
}

#[cfg(test)]
mod tests {
    use exa_combat::Condition;
    use exa_world::nav::source::grid_base;
    use exa_world::nav::{Columns, Layer};

    use super::*;

    /// The probe plans with the player's own footprint lifted (4 cells at
    /// 5, not the doubled 32 and 16 its footprint would add), puts the
    /// footprint back, and waits for a metre of movement to plan again.
    #[test]
    fn the_probe_plans_without_its_footprint() {
        let mut c = Columns::new(40, 40, grid_base(3.4));
        for z in 0..40 {
            for x in 0..40 {
                c.fill_ground(x, z, 0.0);
            }
        }
        let mut app = App::new();
        app.insert_resource(NavGrid::from_columns(&c))
            .init_resource::<NavFootprints>()
            .insert_resource(PathProbe::to(Vec3::new(1.35, 0.0, 0.15)))
            .add_systems(Update, (keep_footprints, plan_probe).chain());
        let world = app.world_mut();
        let root = world
            .spawn((
                CharacterBody::default(),
                Transform::from_xyz(0.15, 0.0, 0.15),
            ))
            .id();
        world.spawn((
            Fighter::default(),
            Condition::NORMAL,
            PlayerBody::new(0.0),
            ChildOf(root),
        ));
        app.update();
        let plan = app.world().resource::<PathProbe>().plan().unwrap().clone();
        assert!(plan.reached);
        assert_eq!((plan.points.len(), plan.cost), (2, 20));
        let g = app.world().resource::<NavGrid>();
        assert_eq!(g.cell(20, 20, Layer::Lower).penalty, 63);

        // Half a metre on: no new plan.
        app.world_mut()
            .entity_mut(root)
            .insert(Transform::from_xyz(0.65, 0.0, 0.15));
        app.world_mut().resource_mut::<PathProbe>().plan = None;
        app.update();
        assert!(app.world().resource::<PathProbe>().plan().is_none());
    }
}
