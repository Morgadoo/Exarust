//! Keyframe tracks of scene nodes, played as the original plays them.
//!
//! A node may carry position tracks (`exa_formats::key_track`): Catmull-Rom
//! curves through keys of time and translation. Nothing plays them until
//! they are started: the original starts every track of an object's nodes
//! when the player uses the object (the one shipped track is the brick of
//! the secret switch, which sinks 5 cm into the wall and comes back out).
//! A started track joins the scene's playing list with the clock's time;
//! every frame its clock turns the time since then into a time on the
//! curve by its play mode, and the node's translation takes the curve's
//! position there. A track that ends (once, reverse) leaves the list and
//! its node where it stopped. Starting a track again plays it the other
//! way: the start swaps once and reverse. The clock is the game's frame
//! clock, which stands still while the game is paused. Evidence:
//! `docs/formats/RFC.md` (Keyframe tracks).

use bevy::prelude::*;
use exa_formats::key_track::{KeyTrack, PlayMode};

/// The debug key that starts every keyframe track of the level (the
/// original starts an object's tracks when it is used, which Exarust cannot
/// do yet).
pub const START_ALL_KEY: KeyCode = KeyCode::F7;

/// Starts, and plays every frame, the scene's keyframe tracks.
pub struct TracksPlugin;

impl Plugin for TracksPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StartTracks>()
            .init_resource::<PlayingTracks>()
            .add_systems(
                Update,
                (start_all_on_key, start_tracks, play_tracks)
                    .chain()
                    .in_set(TrackSystems),
            );
    }
}

/// The systems starting and playing keyframe tracks.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackSystems;

/// A node's keyframe tracks, each with its play mode as the starts so far
/// have swapped it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct NodeTracks(pub Vec<KeyTrack>);

/// Starts every keyframe track of this node and of the nodes under it that
/// is not playing already, as the original does when an object is used.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartTracks(pub Entity);

/// The tracks playing now, in the order they started.
#[derive(Resource, Debug, Default)]
pub struct PlayingTracks(Vec<Playing>);

impl PlayingTracks {
    /// How many tracks are playing.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One playing track: its node, its index among the node's tracks and the
/// clock's time when it started (seconds).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Playing {
    node: Entity,
    track: usize,
    start: f64,
}

/// The mode a start leaves a track in: once and reverse swap, so the next
/// start plays it back the other way; the others stay.
pub fn swapped(mode: PlayMode) -> PlayMode {
    match mode {
        PlayMode::Once => PlayMode::Reverse,
        PlayMode::Reverse => PlayMode::Once,
        other => other,
    }
}

/// The time on the curve `elapsed` seconds after a track started, and
/// whether it goes on playing. Free passes the time on; once clamps it at
/// the duration and ends there; reverse runs from the duration down to 0
/// and ends there; loop wraps it at the duration (with a zero duration it
/// runs on as free, as the original's integer wrap does).
pub fn clock(mode: PlayMode, duration: f32, elapsed: f32) -> (f32, bool) {
    match mode {
        PlayMode::Free => (elapsed, true),
        PlayMode::Once if duration <= elapsed => (duration, false),
        PlayMode::Once => (elapsed, true),
        PlayMode::Reverse => {
            let t = duration - elapsed;
            if t <= 0.0 { (0.0, false) } else { (t, true) }
        }
        PlayMode::Loop if duration == 0.0 => (elapsed, true),
        PlayMode::Loop => (elapsed % duration, true),
    }
}

/// Where a track's curve puts its node at time `t` (seconds): the first
/// key's position up to its time, the last key's from its time on, and in
/// between the cubic Hermite curve of the bracketing keys (the first key
/// whose time is at or past `t` and the one before it). The curve leaves
/// each inner key along the chord of its neighbours, scaled by how much of
/// their span the segment takes; the first segment leaves its first key
/// with no slope, and the last segment arrives along its own chord. `None`
/// for a track without keys or a time no key brackets (a NaN time), which
/// leaves the node as it is, as the original does.
pub fn position_at(track: &KeyTrack, t: f32) -> Option<Vec3> {
    track.position_at(t).map(Vec3::from)
}

/// F7 starts every track of the level.
fn start_all_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    nodes: Query<Entity, With<NodeTracks>>,
    mut out: MessageWriter<StartTracks>,
) {
    if keys.is_some_and(|k| k.just_pressed(START_ALL_KEY)) {
        out.write_batch(nodes.iter().map(StartTracks));
    }
}

fn start_tracks(
    mut starts: MessageReader<StartTracks>,
    time: Res<Time<Virtual>>,
    children: Query<&Children>,
    mut nodes: Query<(&mut NodeTracks, Option<&Name>, Option<&GlobalTransform>)>,
    mut playing: ResMut<PlayingTracks>,
) {
    let now = time.elapsed_secs_f64();
    for &StartTracks(root) in starts.read() {
        let under = std::iter::once(root).chain(children.iter_descendants(root));
        for node in under {
            let Ok((mut tracks, name, at)) = nodes.get_mut(node) else {
                continue;
            };
            for (index, track) in tracks.0.iter_mut().enumerate() {
                if playing.0.iter().any(|p| p.node == node && p.track == index) {
                    continue;
                }
                playing.0.push(Playing {
                    node,
                    track: index,
                    start: now,
                });
                track.mode = swapped(track.mode);
                info!(
                    "keyframe track {index} of {} started: {:?}, {} s, at {:.2?}",
                    name.map_or("a node", |n| n.as_str()),
                    track.mode,
                    track.duration,
                    at.map(|g| g.translation()),
                );
            }
        }
    }
}

fn play_tracks(
    time: Res<Time<Virtual>>,
    mut playing: ResMut<PlayingTracks>,
    mut nodes: Query<(&NodeTracks, &mut Transform, Option<&Name>)>,
) {
    let now = time.elapsed_secs_f64();
    playing.0.retain(|p| {
        let Ok((tracks, mut transform, name)) = nodes.get_mut(p.node) else {
            // The node is gone (the level was left).
            return false;
        };
        let Some(track) = tracks.0.get(p.track) else {
            return false;
        };
        let (t, going) = clock(track.mode, track.duration, (now - p.start) as f32);
        if let Some(at) = position_at(track, t) {
            transform.translation = at;
        }
        if !going {
            debug!(
                "keyframe track {} of {} ended at {:?}",
                p.track,
                name.map_or("a node", |n| n.as_str()),
                transform.translation
            );
        }
        going
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::key_track::PositionKey;

    /// Centimetres to metres, as the file's positions are converted.
    fn cm(v: f32) -> f32 {
        v * 0.01
    }

    /// The shipped track (the secret switch's brick): once, 3 s, keys at
    /// 0, 1, 2 and 3 s pushing 5 cm along −z, holding and coming back.
    fn brick() -> KeyTrack {
        let key = |time, z: f32| PositionKey {
            time,
            position: [0.0, 0.0, cm(z)],
        };
        KeyTrack {
            mode: PlayMode::Once,
            duration: 3.0,
            keys: vec![key(0.0, 0.0), key(1.0, -5.0), key(2.0, -5.0), key(3.0, 0.0)],
        }
    }

    fn z_at(track: &KeyTrack, t: f32) -> f32 {
        position_at(track, t).unwrap().z
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    /// Between keys the curve follows the Hermite basis with the
    /// original's slopes: −2.1875 cm at 0.5 s (no slope leaving the first
    /// key, the chord of keys 0 and 2 halved arriving at key 1), −5.625 cm
    /// at 1.5 s (past the held −5 cm), −2.8125 cm at 2.5 s (the last
    /// segment arrives along its own chord).
    #[test]
    fn curve_between_keys() {
        let b = brick();
        assert!(close(z_at(&b, 0.5), cm(-2.1875)), "{}", z_at(&b, 0.5));
        assert!(close(z_at(&b, 1.5), cm(-5.625)), "{}", z_at(&b, 1.5));
        assert!(close(z_at(&b, 2.5), cm(-2.8125)), "{}", z_at(&b, 2.5));
        // On the keys, the keys' positions.
        for (t, z) in [(1.0, -5.0), (2.0, -5.0)] {
            assert!(close(z_at(&b, t), cm(z)));
        }
        assert_eq!(position_at(&b, 0.0), Some(Vec3::ZERO));
    }

    /// Outside the keys the curve holds the first and the last position;
    /// a NaN time takes the first key, as the original's unordered compare
    /// does; keys out of order bracket by the first key at or past the time.
    #[test]
    fn curve_clamps_outside_the_keys() {
        let mut b = brick();
        b.keys[3].position = [1.0, 2.0, 3.0];
        assert_eq!(position_at(&b, -1.0), Some(Vec3::ZERO));
        assert_eq!(position_at(&b, f32::NAN), Some(Vec3::ZERO));
        assert_eq!(position_at(&b, 3.0), Some(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(position_at(&b, 9.0), Some(Vec3::new(1.0, 2.0, 3.0)));
        let one = KeyTrack {
            keys: vec![b.keys[0]],
            ..brick()
        };
        assert_eq!(position_at(&one, 1.0), Some(Vec3::ZERO));
        let none = KeyTrack {
            keys: Vec::new(),
            ..brick()
        };
        assert_eq!(position_at(&none, 1.0), None);
        // Times 0, 5, 4, 3: 2.5 s falls in the segment ending at 5 s.
        let mut shuffled = brick();
        shuffled.keys[1].time = 5.0;
        shuffled.keys[2].time = 4.0;
        let z = z_at(&shuffled, 2.5);
        assert!(z < 0.0 && z > cm(-5.0), "{z}");
    }

    /// The clock by mode, 3 s duration.
    #[test]
    fn clock_by_mode() {
        assert_eq!(clock(PlayMode::Free, 3.0, 4.5), (4.5, true));
        assert_eq!(clock(PlayMode::Once, 3.0, 1.25), (1.25, true));
        assert_eq!(clock(PlayMode::Once, 3.0, 3.5), (3.0, false));
        assert_eq!(clock(PlayMode::Once, 3.0, 3.0), (3.0, false));
        assert_eq!(clock(PlayMode::Reverse, 3.0, 0.5), (2.5, true));
        assert_eq!(clock(PlayMode::Reverse, 3.0, 3.25), (0.0, false));
        assert_eq!(clock(PlayMode::Loop, 3.0, 4.5), (1.5, true));
        assert_eq!(clock(PlayMode::Loop, 3.0, 7.0).0, 1.0);
        assert_eq!(clock(PlayMode::Loop, 0.0, 4.5), (4.5, true));
    }

    #[test]
    fn starts_swap_once_and_reverse() {
        assert_eq!(swapped(PlayMode::Once), PlayMode::Reverse);
        assert_eq!(swapped(PlayMode::Reverse), PlayMode::Once);
        assert_eq!(swapped(PlayMode::Loop), PlayMode::Loop);
        assert_eq!(swapped(PlayMode::Free), PlayMode::Free);
    }

    fn app() -> App {
        use bevy::time::TimeUpdateStrategy;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TracksPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(250),
            ));
        // Let the clock's first frame pass.
        app.update();
        app
    }

    fn z(app: &App, e: Entity) -> f32 {
        app.world().get::<Transform>(e).unwrap().translation.z
    }

    /// Using the switch starts the brick's track under the object: the
    /// first start plays it in reverse (from 3 s down), a start while it
    /// plays does nothing, it ends after 3 s where it began, and the next
    /// start plays it forward.
    #[test]
    fn a_started_track_plays_on_its_node() {
        let mut app = app();
        let object = app.world_mut().spawn(Transform::default()).id();
        let brick = app
            .world_mut()
            .spawn((
                Transform::default(),
                NodeTracks(vec![brick()]),
                ChildOf(object),
            ))
            .id();
        app.update();
        // Nothing plays before a start.
        assert!(app.world().resource::<PlayingTracks>().is_empty());
        app.world_mut().write_message(StartTracks(object));
        app.update();
        let mode = |app: &App| app.world().get::<NodeTracks>(brick).unwrap().0[0].mode;
        assert_eq!(mode(&app), PlayMode::Reverse);
        assert_eq!(app.world().resource::<PlayingTracks>().len(), 1);
        // Started this frame: 3 s on the reversed curve, the rest position.
        assert!(close(z(&app, brick), 0.0));
        // Two frames on, 0.5 s in: the curve at 2.5 s.
        app.update();
        app.update();
        assert!(close(z(&app, brick), cm(-2.8125)), "{}", z(&app, brick));
        // A start while it plays changes nothing.
        app.world_mut().write_message(StartTracks(object));
        app.update();
        assert_eq!(mode(&app), PlayMode::Reverse);
        assert_eq!(app.world().resource::<PlayingTracks>().len(), 1);
        // 1.5 s in: the middle of the hold.
        for _ in 0..3 {
            app.update();
        }
        assert!(close(z(&app, brick), cm(-5.625)), "{}", z(&app, brick));
        // 3 s in: ended, back at the start, off the list.
        for _ in 0..6 {
            app.update();
        }
        assert!(app.world().resource::<PlayingTracks>().is_empty());
        assert!(close(z(&app, brick), 0.0));
        // The next start plays forward: 0.5 s in, the curve at 0.5 s.
        app.world_mut().write_message(StartTracks(brick));
        app.update();
        assert_eq!(mode(&app), PlayMode::Once);
        app.update();
        app.update();
        assert!(close(z(&app, brick), cm(-2.1875)), "{}", z(&app, brick));
    }

    /// The clock stands still while the game is paused: the brick holds
    /// where it was and goes on from there once the game resumes.
    #[test]
    fn a_paused_game_holds_the_track() {
        let mut app = app();
        let node = app
            .world_mut()
            .spawn((Transform::default(), NodeTracks(vec![brick()])))
            .id();
        app.world_mut().write_message(StartTracks(node));
        app.update();
        app.update();
        app.update();
        // 0.5 s in (reversed): the curve at 2.5 s.
        assert!(close(z(&app, node), cm(-2.8125)), "{}", z(&app, node));
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        for _ in 0..8 {
            app.update();
        }
        assert!(close(z(&app, node), cm(-2.8125)), "{}", z(&app, node));
        assert_eq!(app.world().resource::<PlayingTracks>().len(), 1);
        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        app.update();
        app.update();
        // 1 s of game clock in: the curve at 2 s, the held key.
        assert!(close(z(&app, node), cm(-5.0)), "{}", z(&app, node));
    }

    /// F7 starts every track of the level once: a node's own message and
    /// its parent's both reach it, and its mode swaps only once.
    #[test]
    fn the_debug_key_starts_every_track() {
        let mut app = app();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(START_ALL_KEY);
        app.insert_resource(keys);
        let parent = app
            .world_mut()
            .spawn((Transform::default(), NodeTracks(vec![brick()])))
            .id();
        let child = app
            .world_mut()
            .spawn((
                Transform::default(),
                NodeTracks(vec![brick(), brick()]),
                ChildOf(parent),
            ))
            .id();
        app.update();
        assert_eq!(app.world().resource::<PlayingTracks>().len(), 3);
        for (node, tracks) in [(parent, 1), (child, 2)] {
            let modes: Vec<_> = app
                .world()
                .get::<NodeTracks>(node)
                .unwrap()
                .0
                .iter()
                .map(|t| t.mode)
                .collect();
            assert_eq!(modes, vec![PlayMode::Reverse; tracks]);
        }
    }

    /// A node that goes away leaves the playing list.
    #[test]
    fn a_despawned_node_stops_playing() {
        let mut app = app();
        let node = app
            .world_mut()
            .spawn((Transform::default(), NodeTracks(vec![brick()])))
            .id();
        app.world_mut().write_message(StartTracks(node));
        app.update();
        assert_eq!(app.world().resource::<PlayingTracks>().len(), 1);
        app.world_mut().despawn(node);
        app.update();
        assert!(app.world().resource::<PlayingTracks>().is_empty());
    }
}
