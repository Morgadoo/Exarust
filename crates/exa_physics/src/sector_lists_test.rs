//! Tests of the sector's body grid and the Height Query's mesh mode, on
//! synthetic bodies.

use super::*;
use crate::body::RigidPart;

/// A cube of half side `h` about `centre`, fronts facing out.
fn cube(centre: Vec3, h: f32) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let u = axis.any_orthonormal_vector();
        let v = axis.cross(u);
        for n in [axis, -axis] {
            let c = |a: f32, b: f32| centre + (n + u * a + v * b) * h;
            for t in [
                [c(-1.0, -1.0), c(1.0, -1.0), c(1.0, 1.0)],
                [c(-1.0, -1.0), c(1.0, 1.0), c(-1.0, 1.0)],
            ] {
                let front = (t[1] - t[0]).cross(t[2] - t[0]);
                out.push(if front.dot(n) > 0.0 {
                    t
                } else {
                    [t[0], t[2], t[1]]
                });
            }
        }
    }
    out
}

/// A one-part rigid body at `at` whose node box is a 20 cm cube on its
/// origin (raised 20 cm), with that cube as its mesh.
fn crate_at(at: Vec3) -> Ragdoll {
    let mut body = Ragdoll::rigid(&[RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        rest: [
            Vec3::new(-0.1, 0.0, -0.1),
            Vec3::new(0.1, 0.0, -0.1),
            Vec3::new(0.0, 0.0, 0.1),
            Vec3::new(0.0, 0.2, 0.0),
        ],
        point_masses: [4.0; 4],
        parent_world: Affine3A::IDENTITY,
        mobility: 1.0,
        spheres: vec![(Vec3::Y * 0.2, 0.2)],
        origin: Vec3::ZERO,
        links: Vec::new(),
        mesh: Some(sector::Bounds {
            centre: Vec3::Y * 0.2,
            half: Vec3::splat(0.2),
            radius: 0.35,
        }),
        triangles: None,
    }]);
    body.set_part_mesh(0, Arc::new(cube(Vec3::Y * 0.2, 0.2)));
    body
}

fn entity(i: u32) -> Entity {
    Entity::from_raw_u32(i).unwrap()
}

/// Parts are listed in the cell their mesh box's world centre falls in,
/// in walk order; the nine-cell scan finds a neighbour's.
#[test]
fn the_body_grid_lists_parts_by_their_mesh_box_centre() {
    let (a, b, c) = (
        crate_at(Vec3::new(1.0, 0.0, 1.0)),
        crate_at(Vec3::new(7.0, 0.0, 1.0)),
        crate_at(Vec3::new(-0.5, 0.0, 1.0)),
    );
    let mut grid = SectorBodies::default();
    grid.rebuild([(entity(1), &a), (entity(2), &b), (entity(3), &c)].into_iter());
    assert_eq!(grid.len(), 3);
    assert_eq!(
        grid.at((0, 0)).iter().map(|e| e.body).collect::<Vec<_>>(),
        [entity(1)]
    );
    assert_eq!(grid.at((1, 0))[0].body, entity(2));
    assert_eq!(grid.at((-1, 0))[0].body, entity(3));
    let near: Vec<Entity> = grid.near((0, 0)).map(|e| e.body).collect();
    // The own cell first, then the neighbours in the offset table's order.
    assert_eq!(near, [entity(1), entity(3), entity(2)]);
    assert!(grid.at((5, 5)).is_empty());
    // A rebuild starts again.
    grid.rebuild([(entity(2), &b)].into_iter());
    assert_eq!(grid.len(), 1);
    assert!(grid.at((0, 0)).is_empty());
}

/// The Height Query's mesh mode skips a body out of the world and a
/// living character's limb; a dead one's limb is tested.
#[test]
fn the_height_query_skips_living_limbs_and_disabled_bodies() {
    let entry = |limb, disabled| BodyEntry {
        body: entity(1),
        part: 0,
        node: entity(1),
        limb,
        disabled,
    };
    assert!(height_tests(&entry(false, false), |_| false));
    assert!(!height_tests(&entry(false, true), |_| false));
    assert!(!height_tests(&entry(true, false), |_| false));
    assert!(height_tests(&entry(true, false), |_| true));
}

/// Over a body's mesh the Height Query lands on its top; beside it, on
/// the level under it.
#[test]
fn the_mesh_mode_lands_on_a_body_on_the_floor() {
    let floor: Vec<[Vec3; 3]> = vec![
        [
            Vec3::new(-10.0, 0.0, -10.0),
            Vec3::new(10.0, 0.0, 10.0),
            Vec3::new(10.0, 0.0, -10.0),
        ],
        [
            Vec3::new(-10.0, 0.0, -10.0),
            Vec3::new(-10.0, 0.0, 10.0),
            Vec3::new(10.0, 0.0, 10.0),
        ],
    ];
    assert!(
        floor
            .iter()
            .all(|t| (t[1] - t[0]).cross(t[2] - t[0]).y > 0.0)
    );
    let body = crate_at(Vec3::new(1.0, 0.0, 1.0));
    let mut grid = SectorBodies::default();
    grid.rebuild([(entity(1), &body)].into_iter());
    let meshes = height_meshes(&grid, Vec3::new(1.0, 1.0, 1.0), |_| false, |_| Some(&body));
    assert_eq!(meshes.len(), 1);
    assert!(meshes[0].tree.is_some());
    let level = MeshLevel::new(floor.as_slice(), &meshes);
    let over = level
        .height_below(Vec3::new(1.0, 1.0, 1.0), 0.1, 3.0)
        .unwrap();
    assert!((over - 0.4).abs() < 1e-4, "{over}");
    let beside = level
        .height_below(Vec3::new(2.0, 1.0, 1.0), 0.1, 3.0)
        .unwrap();
    assert!(beside.abs() < 1e-4, "{beside}");
    assert_eq!(level.body_hits.get(), 1);
    // Without the body, the level alone.
    let bare = floor
        .as_slice()
        .height_below(Vec3::new(1.0, 1.0, 1.0), 0.1, 3.0)
        .unwrap();
    assert!(bare.abs() < 1e-4);
    // A living limb's mesh is not among the meshes tested.
    let mut grid = SectorBodies::default();
    grid.insert(
        Vec3::new(1.0, 0.2, 1.0),
        BodyEntry {
            body: entity(1),
            part: 0,
            node: entity(1),
            limb: true,
            disabled: false,
        },
    );
    assert!(height_meshes(&grid, Vec3::new(1.0, 1.0, 1.0), |_| false, |_| Some(&body)).is_empty());
}
