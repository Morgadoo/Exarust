//! Tests of the search activity.
//! (Included from search.rs: `#[path = "search_test.rs"] mod tests;`)

use super::*;

/// A searcher whose brain answers as set and counts what is asked of it.
struct Fake {
    now: u32,
    roll: u32,
    root: Vec3,
    centre: Vec3,
    attention: Attention,
    walking: bool,
    plans: Vec<Goal>,
    accept: bool,
    attends: bool,
    alert: bool,
    gaze: Gaze,
    pack: Pack,
    searched: Vec<u32>,
    waits: u32,
    settles: u32,
}

impl Fake {
    fn new() -> Self {
        Self {
            now: 1000,
            roll: 0,
            root: Vec3::ZERO,
            centre: Vec3::ZERO,
            attention: Attention::Goal,
            walking: false,
            plans: Vec::new(),
            accept: true,
            attends: false,
            alert: false,
            gaze: Gaze {
                body_yaw: 0.0,
                head: Vec3::Z,
                look_yaw: 0.0,
                look_pitch: 0.0,
                turn_to: 0.0,
                look_until: 0,
            },
            pack: Pack::default(),
            searched: Vec::new(),
            waits: 0,
            settles: 0,
        }
    }
}

impl Searcher for Fake {
    fn now_ms(&self) -> u32 {
        self.now
    }
    fn roll(&mut self) -> &mut u32 {
        &mut self.roll
    }
    fn root(&self) -> Vec3 {
        self.root
    }
    fn centre(&self) -> Vec3 {
        self.centre
    }
    fn reachable(&self, _from: Vec3, _to: Vec3) -> bool {
        true
    }
    fn alert_look(&mut self) -> bool {
        self.alert
    }
    fn attend(&mut self, _level: f32) -> bool {
        self.attends
    }
    fn wait(&mut self) {
        self.waits += 1;
    }
    fn settle(&mut self) {
        self.settles += 1;
    }
    fn idle_look(&mut self) {}
    fn attention(&self) -> Attention {
        self.attention
    }
    fn plan(&mut self, goal: Goal) -> bool {
        self.plans.push(goal);
        self.accept
    }
    fn walking(&self) -> bool {
        self.walking
    }
    fn calm_walk(&mut self) {}
    fn clear_path_mark(&mut self) {}
    fn gaze(&mut self) -> &mut Gaze {
        &mut self.gaze
    }
    fn pack(&mut self) -> &mut Pack {
        &mut self.pack
    }
    fn pack_id(&self) -> u32 {
        77
    }
    fn searched(&mut self) -> &mut Vec<u32> {
        &mut self.searched
    }
    fn now_s(&self) -> u32 {
        self.now / 1000
    }
}

/// Apparel covering one bone with the given coverage and protection.
fn piece(bone: usize, coverage: u32, protection: u8, occupancy: u64) -> Apparel {
    let mut c = [0; 20];
    c[bone] = coverage;
    let mut o = [0; 20];
    o[0] = occupancy;
    Apparel {
        coverage: c,
        protection: [protection; 4],
        occupancy: o,
        special: 0,
        model_mark: 0,
    }
}

fn blade(flags: u32, tier: u8) -> Arm {
    Arm {
        flags,
        quality: 255,
        damage: 0,
        thrust: 140,
        tier,
        special: 0,
    }
}

fn apparel_loot(piece: Apparel) -> Loot {
    Loot::Apparel(Box::new(piece))
}

fn carried(ids: &[u32]) -> Vec<Carried> {
    ids.iter().map(|&item| Carried { item, flags: 0 }).collect()
}

/// Bone 1 fully covered at protection 10 each: each bone value is
/// 100 · 0.01 · 0.005 · 40 = 0.2, its bulk 40 · 0.005 = 0.2. The second
/// bone weighs 1, the fourth 0.5, the tenth 4.
#[test]
fn apparel_coverage_and_bulk_follow_the_bone_weights() {
    let p = piece(0, 100, 10, 1);
    let (sum, most) = p.coverage();
    assert!((sum - 0.2).abs() < 1e-6 && (most - 0.2).abs() < 1e-6);
    assert!((p.bulk() - 0.2).abs() < 1e-6);
    let (sum, _) = piece(3, 100, 10, 1).coverage();
    assert!((sum - 0.1).abs() < 1e-6);
    let (sum, most) = piece(9, 25, 10, 1).coverage();
    assert!((sum - 0.2).abs() < 1e-6 && (most - 0.05).abs() < 1e-6);
    assert!((piece(9, 25, 10, 1).bulk() - 0.4).abs() < 1e-6);
}

/// A piece over nothing is worth (1.2 + 1.2)/2 = 1.2; over an equal piece
/// (carried or worn) 1; a piece it does not overlap does not count.
#[test]
fn apparel_is_worth_its_gain_over_what_it_overlaps() {
    let mut items = BTreeMap::new();
    items.insert(1, apparel_loot(piece(0, 100, 10, 1)));
    items.insert(2, apparel_loot(piece(0, 100, 10, 1)));
    items.insert(3, apparel_loot(piece(0, 100, 10, 2)));
    let empty = Pack::default();
    assert!((apparel_worth(&items, &empty, 1) - 1.2).abs() < 1e-6);
    let worn = Pack {
        worn: vec![2],
        ..Pack::default()
    };
    assert!((apparel_worth(&items, &worn, 1) - 1.0).abs() < 1e-6);
    let held = Pack {
        carried: carried(&[2]),
        ..Pack::default()
    };
    assert!((apparel_worth(&items, &held, 1) - 1.0).abs() < 1e-6);
    let aside = Pack {
        worn: vec![3],
        ..Pack::default()
    };
    assert!((apparel_worth(&items, &aside, 1) - 1.2).abs() < 1e-6);
    assert_eq!(apparel_worth(&items, &empty, 9), 0.0);
}

/// Tier 2 at full quality, no damage, thrust 140: 1 + 2 = 3; the `0x10`
/// hand bits make it 2.4; tier 0 is −10. Classes by the hand bits and the
/// low nibble.
#[test]
fn a_weapon_is_valued_by_tier_quality_and_wear() {
    assert_eq!(blade(0x21, 2).value(), 3.0);
    assert!((blade(0x11, 2).value() - 2.4).abs() < 1e-6);
    assert_eq!(blade(0x21, 0).value(), -10.0);
    let worn = Arm {
        damage: 255,
        quality: 0,
        ..blade(0x20, 2)
    };
    assert!((worn.value() - (1.0 + 2.0 * 0.5 * 0.7)).abs() < 1e-6);
    let classes: Vec<_> = [0x11, 0x10, 0x21, 0x20]
        .iter()
        .map(|f| blade(*f, 1).class())
        .collect();
    assert_eq!(classes, vec![0, 1, 2, 3]);
}

/// A weapon's worth is (v + 1) over the best (v + 1) of its class carried:
/// tier 4 (value 5) over a carried tier 2 (value 3) is 6/4 = 1.5; with no
/// weapon of its class, 6.
#[test]
fn a_weapon_is_worth_its_gain_over_its_class() {
    let mut items = BTreeMap::new();
    items.insert(1, Loot::Weapon(blade(0x20, 4)));
    items.insert(2, Loot::Weapon(blade(0x20, 2)));
    items.insert(3, Loot::Weapon(blade(0x21, 2)));
    let pack = Pack {
        carried: carried(&[2]),
        ..Pack::default()
    };
    assert_eq!(weapon_worth(&items, &pack, 1), 1.5);
    let other = Pack {
        carried: carried(&[3]),
        ..Pack::default()
    };
    assert_eq!(weapon_worth(&items, &other, 1), 6.0);
}

/// The wear room is the piece's bulk less the bulk of the worn pieces it
/// overlaps, and 0 when one of them has a model byte above 50.
#[test]
fn the_wear_room_is_what_the_piece_adds() {
    let mut items = BTreeMap::new();
    let big = piece(9, 100, 10, 1);
    items.insert(2, apparel_loot(piece(0, 100, 10, 1)));
    let mut marked = piece(0, 100, 10, 1);
    marked.model_mark = 51;
    items.insert(3, apparel_loot(marked));
    let pack = Pack {
        worn: vec![2],
        ..Pack::default()
    };
    assert!((wear_room(&items, &pack, &big) - (0.8 - 0.2)).abs() < 1e-6);
    let pack = Pack {
        worn: vec![3],
        ..Pack::default()
    };
    assert_eq!(wear_room(&items, &pack, &big), 0.0);
}

fn weigh_one(
    me: &mut Fake,
    items: BTreeMap<u32, Loot>,
    leader: Option<&Pack>,
    item: u32,
) -> (u8, Vec<Deed>) {
    let scene = Scene {
        items,
        ..Scene::default()
    };
    let mut search = Search::new();
    let mut phrases = Phrases::default();
    let mut deeds = Vec::new();
    let names = |_: u32| String::from("Helmet");
    let r = search.weigh(
        me,
        &scene,
        &mut phrases,
        leader,
        &names,
        0,
        Source::Container,
        item,
        &mut deeds,
    );
    (r, deeds)
}

/// Apparel worth 1.2 is worn when it adds more than 15 % of its bulk, and
/// carried when an equal piece already covers it; worth 1 is left.
#[test]
fn apparel_is_worn_or_carried_by_its_room() {
    let mut items = BTreeMap::new();
    items.insert(1, apparel_loot(piece(0, 100, 10, 1)));
    items.insert(2, apparel_loot(piece(0, 100, 10, 1)));
    let mut me = Fake::new();
    let (r, deeds) = weigh_one(&mut me, items.clone(), None, 1);
    assert_eq!(r, 1);
    assert_eq!(
        deeds,
        vec![Deed::Wear {
            object: 0,
            source: Source::Container,
            item: 1
        }]
    );
    let mut me = Fake::new();
    me.pack.carried = carried(&[2]);
    let (r, _) = weigh_one(&mut me, items.clone(), None, 1);
    assert_eq!(r, 0);
    let mut big = items.clone();
    big.insert(1, apparel_loot(piece(9, 100, 10, 1)));
    let mut me = Fake::new();
    me.pack.worn = vec![2];
    let mut marked = piece(0, 100, 10, 1);
    marked.model_mark = 60;
    big.insert(2, apparel_loot(marked));
    let (r, deeds) = weigh_one(&mut me, big, None, 1);
    assert_eq!(r, 1);
    assert_eq!(deeds[1], Deed::Carry { item: 1 });
    assert_eq!(me.pack.carried, carried(&[1]));
}

/// A weapon worth more than 1.05 is carried and the hands re-armed; one of
/// the `0x06000000` kinds is not weighed.
#[test]
fn a_better_weapon_is_carried_and_wielded() {
    let mut items = BTreeMap::new();
    items.insert(1, Loot::Weapon(blade(0x20, 4)));
    items.insert(2, Loot::Weapon(blade(0x20, 2)));
    items.insert(3, Loot::Weapon(blade(0x0200_0020, 9)));
    let mut me = Fake::new();
    me.pack.carried = carried(&[2]);
    me.pack.hands[0] = 2;
    let (r, deeds) = weigh_one(&mut me, items.clone(), None, 1);
    assert_eq!(r, 1);
    assert_eq!(deeds[1], Deed::Carry { item: 1 });
    assert_eq!(deeds[2], Deed::Wield { item: 1 });
    assert_eq!(deeds[3], Deed::Stow { item: 2 });
    let (r, deeds) = weigh_one(&mut Fake::new(), items, None, 3);
    assert_eq!((r, deeds.len()), (0, 0));
}

/// With the leader in sight a special piece, or one worth more to the
/// leader past its threshold, is commented on instead of taken.
#[test]
fn the_leader_hears_about_what_suits_it_better() {
    let mut items = BTreeMap::new();
    items.insert(1, apparel_loot(piece(0, 100, 100, 1)));
    items.insert(2, apparel_loot(piece(0, 100, 100, 1)));
    let mut special = piece(0, 100, 10, 4);
    special.special = 1;
    items.insert(3, apparel_loot(special));
    let leader = Pack::default();
    let mut me = Fake::new();
    me.pack.worn = vec![2];
    let (r, deeds) = weigh_one(&mut me, items.clone(), Some(&leader), 1);
    assert_eq!(r, 2);
    assert!(matches!(deeds[0], Deed::Comment { .. }));
    let (r, _) = weigh_one(&mut Fake::new(), items.clone(), Some(&leader), 3);
    assert_eq!(r, 2);
    // Theirs 1.2 is under max(0.55 − 0.2 · 0.05, 0) + 0.9 = 1.44: taken.
    let mut small = items;
    small.insert(1, apparel_loot(piece(0, 100, 10, 1)));
    let (r, deeds) = weigh_one(&mut Fake::new(), small, Some(&leader), 1);
    assert_eq!(r, 1);
    assert!(matches!(deeds[0], Deed::Wear { .. }));
}

/// The first comment draws from the first table, the second (another name
/// within 10 s) from the further one; "a"/"an" by the first letter, the
/// last letter left as it was, and a capital start.
#[test]
fn comments_follow_the_phrase_rules() {
    let mut phrases = Phrases::default();
    let mut remarks = Remarks::default();
    let mut roll = 0;
    let first = comment(&mut phrases, &mut remarks, "Helmet", 5, &mut roll);
    assert_eq!(first, "I found a helmet.");
    assert_eq!(phrases.first_one.last, [0, 8]);
    assert_eq!(phrases.first_pair.last, [0, 8]);
    let second = comment(&mut phrases, &mut remarks, "AXE", 15, &mut roll);
    assert_eq!(second, "And an axE.");
    assert_eq!(remarks.names, vec!["axE".to_string(), "helmet".into()]);
    let mut late = remarks.clone();
    let mut r = 0;
    // After 10 s the window starts again; the first table's last draw (0)
    // is drawn twice more before 4 comes.
    let again = comment(&mut phrases, &mut late, "Helmet", 26, &mut r);
    assert_eq!(again, "A helmet here, if you want it.");
    assert_eq!(late.names, vec!["helmet".to_string()]);
    let mut table = Phrases::default();
    table.first_one.templates[0] = "[a], should you want it.".into();
    let mut fresh = Remarks::default();
    let mut r = 0;
    assert_eq!(
        comment(&mut table, &mut fresh, "Owl", 0, &mut r),
        "An owl, should you want it."
    );
    let mut pairs = Phrases::default();
    let mut r = 0;
    let said = comment(
        &mut pairs,
        &mut Remarks::default(),
        "pair of boots",
        0,
        &mut r,
    );
    assert_eq!(said, "I found some pair of boots.");
    assert_eq!(lower_but_last("HELM"), "helM");
}

/// A draw equal to one of the last two is drawn again.
#[test]
fn a_comment_does_not_repeat_the_last_two() {
    let mut phrases = Phrases::default();
    phrases.first_one.last = [0, 1];
    let mut roll = 0;
    let said = comment(&mut phrases, &mut Remarks::default(), "Cup", 0, &mut roll);
    assert_eq!(phrases.first_one.last, [4, 0]);
    assert_eq!(said, "A cup here, if you want it.");
}

#[test]
fn the_quick_arcsine_meets_its_ends() {
    assert_eq!(quick_asin(0.0), 0.0);
    assert!((quick_asin(1.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert!((quick_asin(-1.0) + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert!((quick_asin(0.5) - 0.5f32.asin()).abs() < 0.01);
}

/// The sweep starts to one side (the generator's top bit clear: −90°),
/// turns the look by 1.5 % of the gap plus 0.001, goes to the other side
/// within 0.1 rad, then turns the body 162°; a target within 0.4 rad of
/// the head stops it.
#[test]
fn the_sweep_looks_both_ways_then_turns() {
    let mut s = Sweep::default();
    let mut gaze = Fake::new().gaze;
    let (mut timer, mut roll) = (9000, 0);
    assert!(!sweep(
        &mut s,
        &mut gaze,
        Vec3::ZERO,
        None,
        1000,
        &mut timer,
        &mut roll
    ));
    assert_eq!(s.phase, 1);
    assert_eq!(s.heading, -HALF_PI);
    let expected = -0.001 - HALF_PI * 0.015;
    assert!((gaze.look_yaw - expected).abs() < 1e-6);
    gaze.look_yaw = -HALF_PI + 0.05;
    sweep(
        &mut s,
        &mut gaze,
        Vec3::ZERO,
        None,
        1000,
        &mut timer,
        &mut roll,
    );
    assert_eq!((s.phase, s.heading), (3, HALF_PI));
    gaze.look_yaw = HALF_PI - 0.05;
    sweep(
        &mut s,
        &mut gaze,
        Vec3::ZERO,
        None,
        1000,
        &mut timer,
        &mut roll,
    );
    assert_eq!(s.phase, 5);
    assert_eq!(gaze.turn_to, TURN);
    let target = Vec3::new(10.0, 0.0, 100.0);
    assert!(sweep(
        &mut s,
        &mut gaze,
        Vec3::ZERO,
        Some(target),
        1234,
        &mut timer,
        &mut roll
    ));
    assert_eq!((s.phase, timer, gaze.look_until), (7, 1234, 1234));
    assert!((gaze.turn_to - 0.1f32.atan()).abs() < 1e-6);
}

/// A 9 × 9 grid with the world origin at cell (4, 4).
fn grid() -> RoomGrid {
    RoomGrid {
        width: 9,
        height: 9,
        origin: (4, 4),
        flags: vec![CONNECTED; 81],
        characters: vec![Vec::new(); 81],
        objects: vec![Vec::new(); 81],
    }
}

fn chest(id: u32, at: Vec3) -> SceneObject {
    SceneObject {
        id,
        kind: object::CONTAINER_KIND | object::PRESENT,
        searchable: true,
        fixed: false,
        at,
        half: Vec3::new(30.0, 30.0, 30.0),
        reserved: 0,
        corpse: None,
    }
}

/// The flood reaches connected cells within √10 cells inside the border:
/// (3, 1) away is in, (3, 2) is not, nor a cell that is not connected.
/// Fixed, searched, unsearchable and record-less objects and living
/// characters are left; the rest are reserved.
#[test]
fn gathering_floods_the_nearby_rooms() {
    let mut scene = Scene {
        grid: grid(),
        ..Scene::default()
    };
    let cell = |x: i32, y: i32| (y * 9 + x) as usize;
    let add = |scene: &mut Scene, x: i32, y: i32, o: SceneObject| {
        scene.objects.push(o);
        let i = scene.objects.len() - 1;
        scene.grid.objects[cell(x, y)].push(i);
        i
    };
    let near = add(&mut scene, 4, 4, chest(10, Vec3::ZERO));
    let far = add(&mut scene, 7, 5, chest(11, Vec3::ZERO));
    add(&mut scene, 7, 6, chest(12, Vec3::ZERO));
    let mut fixed = chest(13, Vec3::ZERO);
    fixed.fixed = true;
    add(&mut scene, 4, 4, fixed);
    add(&mut scene, 4, 4, chest(14, Vec3::ZERO));
    let mut plain = chest(15, Vec3::ZERO);
    plain.searchable = false;
    add(&mut scene, 4, 4, plain);
    add(&mut scene, 4, 4, chest(99, Vec3::ZERO));
    for id in [10, 11, 12, 13, 14, 15] {
        scene.items.insert(id, Loot::Other);
    }
    scene.grid.flags[cell(5, 3)] = 0;
    add(&mut scene, 5, 3, chest(16, Vec3::ZERO));
    scene.items.insert(16, Loot::Other);
    let body = add(&mut scene, 4, 5, chest(0, Vec3::ZERO));
    scene.objects[body].kind = object::PRESENT;
    for (dead, pack_id) in [(true, 5), (false, 6)] {
        scene.corpses.push(Corpse {
            character: true,
            dead,
            pack_id,
            pack: Pack::default(),
            object: body,
        });
        let c = scene.corpses.len() - 1;
        scene.grid.characters[cell(4, 5)].push(c);
    }
    let mut s = Search::new();
    s.gather(&mut scene, &[14]);
    assert!(s.candidates.contains(&near));
    assert!(s.candidates.contains(&far));
    assert!(s.candidates.contains(&body));
    assert_eq!(s.candidates.len(), 3);
    assert_eq!(scene.objects[near].reserved, 1);
    assert_eq!(scene.objects[body].reserved, 1);
    let mut again = Search::new();
    again.gather(&mut scene, &[5 | CORPSE_TAG]);
    assert!(!again.candidates.contains(&body));
}

/// A character of the cell whose object is container-like is weighed as an
/// object, before the dead-character test: a living one with a chest of a
/// record is gathered, a dead one whose chest is searched is not, even with
/// its pack unsearched.
#[test]
fn a_character_with_a_container_object_is_weighed_as_one() {
    let mut scene = Scene {
        grid: grid(),
        ..Scene::default()
    };
    let cell = 4 * 9 + 4;
    for (id, dead) in [(20, false), (21, true)] {
        scene.objects.push(chest(id, Vec3::ZERO));
        scene.items.insert(id, Loot::Other);
        scene.corpses.push(Corpse {
            character: true,
            dead,
            pack_id: id + 100,
            pack: Pack::default(),
            object: scene.objects.len() - 1,
        });
        scene.grid.characters[cell].push(scene.corpses.len() - 1);
    }
    let mut s = Search::new();
    s.gather(&mut scene, &[21]);
    assert_eq!(s.candidates, vec![0]);
    assert_eq!(
        (scene.objects[0].reserved, scene.objects[1].reserved),
        (1, 0)
    );
}

/// One container holding one piece of apparel, 150 cm ahead of a searcher
/// that walked there.
fn looted_scene() -> Scene {
    let mut scene = Scene {
        grid: grid(),
        ..Scene::default()
    };
    scene.objects.push(chest(40, Vec3::new(0.0, 0.0, 150.0)));
    scene.grid.objects[4 * 9 + 4].push(0);
    scene.containers.insert(
        40,
        Container {
            items: vec![41],
            max_item: 0,
        },
    );
    scene.items.insert(41, apparel_loot(piece(0, 100, 10, 1)));
    scene
}

fn step(s: &mut Search, me: &mut Fake, scene: &mut Scene, anchor: Vec3) -> (bool, Vec<Deed>) {
    let mut phrases = Phrases::default();
    let names = |_: u32| String::from("Coat");
    s.tick(me, scene, &mut phrases, anchor, None, &names)
}

/// The whole cycle: standing until the anchor is 100 cm off, gathering,
/// picking (arrival 30 · 0.999 + 250 cm, 8 s timer), looking, the forced
/// path 100 cm under the chest, arriving, kneeling 1000 + draw(1000) ms,
/// taking the coat, and once the chest is empty remembering it.
#[test]
fn a_search_loots_a_chest() {
    let mut scene = looted_scene();
    let mut me = Fake::new();
    let mut s = Search::new();
    let (busy, _) = step(&mut s, &mut me, &mut scene, Vec3::new(50.0, 0.0, 0.0));
    assert!(!busy);
    assert_eq!(s.state, State::Stand);
    let anchor = Vec3::new(100.0, 0.0, 0.0);
    let (busy, _) = step(&mut s, &mut me, &mut scene, anchor);
    assert!(busy);
    assert_eq!(
        (s.state, s.origin, s.candidates.clone()),
        (State::Pick, anchor, vec![0])
    );
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(s.state, State::Look);
    assert_eq!(s.timer_ms, 9000);
    assert!((s.arrival_cm - (90.0 * 0.333 + 250.0)).abs() < 1e-4);
    assert!(s.busy);
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(s.state, State::Path);
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(s.state, State::Approach);
    assert_eq!(
        me.plans[0],
        Goal {
            at: Vec3::new(0.0, -100.0, 150.0),
            stop_cm: 50,
            radius_cm: 100.0,
            mode: 7
        }
    );
    me.centre = Vec3::new(0.0, 0.0, 20.0);
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!((s.state, s.source), (State::Kneel, Source::Container));
    assert_eq!(me.settles, 1);
    let kneel = s.timer_ms;
    assert!((2000..3000).contains(&kneel));
    me.now = kneel;
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(s.state, State::Kneel);
    me.now = kneel + 1;
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(s.state, State::Loot);
    let (_, deeds) = step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(
        deeds,
        vec![Deed::Wear {
            object: 0,
            source: Source::Container,
            item: 41
        }]
    );
    assert_eq!(s.index, 0);
    assert!((me.now + 1000..me.now + 1500).contains(&s.timer_ms));
    scene.containers.get_mut(&40).unwrap().items.clear();
    me.now = s.timer_ms;
    let (busy, deeds) = step(&mut s, &mut me, &mut scene, anchor);
    assert!(busy && deeds.is_empty());
    assert_eq!(me.searched, vec![40]);
    assert_eq!((s.state, s.target, s.busy), (State::Pick, None, false));
    assert_eq!(scene.objects[0].reserved, 0);
    let (busy, _) = step(&mut s, &mut me, &mut scene, anchor);
    assert!(!busy);
    assert_eq!(s.state, State::Stand);
}

/// Short of the chest after the walk it paths again; the third time it lets
/// the chest go. Its own pack open in the window pauses it, not busy.
#[test]
fn three_short_walks_give_up() {
    let mut scene = looted_scene();
    let mut me = Fake::new();
    let mut s = Search::new();
    let anchor = Vec3::new(100.0, 0.0, 0.0);
    for _ in 0..4 {
        step(&mut s, &mut me, &mut scene, anchor);
    }
    assert_eq!(s.state, State::Approach);
    me.centre = Vec3::new(0.0, 0.0, -400.0);
    scene.open.push(77 | CORPSE_TAG);
    let (busy, _) = step(&mut s, &mut me, &mut scene, anchor);
    assert!(!busy);
    assert_eq!(s.state, State::Approach);
    scene.open.clear();
    for tries in 1..3 {
        step(&mut s, &mut me, &mut scene, anchor);
        assert_eq!((s.state, s.tries), (State::Path, tries));
        step(&mut s, &mut me, &mut scene, anchor);
    }
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!((s.state, s.target), (State::Pick, None));
    assert_eq!(scene.objects[0].reserved, 0);
}

/// Someone else's attention restarts the look and its 8 s timer; no one's
/// lets the chest go; the chest beyond 1100 cm sends it back to picking,
/// still reserved.
#[test]
fn attention_and_reach_steer_the_look() {
    let mut scene = looted_scene();
    let mut me = Fake::new();
    let mut s = Search::new();
    let anchor = Vec3::new(100.0, 0.0, 0.0);
    for _ in 0..2 {
        step(&mut s, &mut me, &mut scene, anchor);
    }
    assert_eq!(s.state, State::Look);
    me.attention = Attention::Other;
    me.now = 5000;
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(
        (s.state, s.timer_ms, s.sweep.phase),
        (State::Look, 13_000, 0)
    );
    me.attention = Attention::Nobody;
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!((s.state, s.target), (State::Pick, None));
    let mut s = Search::new();
    let mut scene = looted_scene();
    me.attention = Attention::Goal;
    for _ in 0..2 {
        step(&mut s, &mut me, &mut scene, anchor);
    }
    let far = Vec3::new(0.0, 0.0, 1300.0);
    s.origin = far;
    step(&mut s, &mut me, &mut scene, far);
    assert_eq!(s.state, State::Stand);
    assert_eq!(scene.objects[0].reserved, 1);
    me.alert = true;
    assert!(!step(&mut s, &mut me, &mut scene, far).0);
}

/// A dead character is looted worn items first, then its pack, skipping
/// entries flagged 0x100, and remembered by its object id tagged; its pack
/// open in the window sends it back to picking.
#[test]
fn a_corpse_is_looted_worn_then_carried() {
    let mut scene = looted_scene();
    scene.containers.clear();
    let mut body = chest(50, Vec3::new(0.0, 0.0, 150.0));
    body.kind = object::PRESENT;
    body.corpse = Some(0);
    scene.objects[0] = body;
    scene.corpses.push(Corpse {
        character: true,
        dead: true,
        pack_id: 60,
        pack: Pack {
            worn: vec![41],
            carried: vec![
                Carried {
                    item: 42,
                    flags: SKIP_FLAG,
                },
                Carried { item: 43, flags: 0 },
            ],
            ..Pack::default()
        },
        object: 0,
    });
    scene.grid.objects[4 * 9 + 4].clear();
    scene.grid.characters[4 * 9 + 4].push(0);
    scene.items.insert(43, Loot::Weapon(blade(0x20, 3)));
    let mut me = Fake::new();
    let mut s = Search::new();
    let anchor = Vec3::new(100.0, 0.0, 0.0);
    me.centre = Vec3::new(0.0, 0.0, 150.0);
    for _ in 0..6 {
        step(&mut s, &mut me, &mut scene, anchor);
    }
    assert_eq!(
        (s.state, s.source, s.corpse_pack),
        (State::Kneel, Source::Worn, 60 | CORPSE_TAG)
    );
    me.now = s.timer_ms + 1;
    step(&mut s, &mut me, &mut scene, anchor);
    let (_, deeds) = step(&mut s, &mut me, &mut scene, anchor);
    assert!(matches!(
        deeds[0],
        Deed::Wear {
            source: Source::Worn,
            item: 41,
            ..
        }
    ));
    scene.corpses[0].pack.worn.clear();
    me.now = s.timer_ms;
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!((s.source, s.index), (Source::Carried, 0));
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(s.index, 1);
    let (_, deeds) = step(&mut s, &mut me, &mut scene, anchor);
    assert!(matches!(
        deeds[0],
        Deed::Take {
            source: Source::Carried,
            item: 43,
            ..
        }
    ));
    scene.corpses[0].pack.carried.truncate(1);
    me.now = s.timer_ms;
    step(&mut s, &mut me, &mut scene, anchor);
    assert_eq!(me.searched, vec![50 | CORPSE_TAG]);
    let mut s = Search::new();
    let mut scene2 = scene.clone();
    scene2.objects[0].reserved = 0;
    me.searched.clear();
    scene2.open.push(60 | CORPSE_TAG);
    for _ in 0..5 {
        step(&mut s, &mut me, &mut scene2, anchor);
    }
    assert_eq!((s.state, s.picked), (State::Pick, 0));
}

/// The best weapon goes to the main hand; the next best is stowed, one of
/// the same hand group counting 0.6. With a second hand held, the better
/// of the first two classes.
#[test]
fn rearming_takes_the_best_and_stows_the_next() {
    let mut items = BTreeMap::new();
    items.insert(1, Loot::Weapon(blade(0x11, 2)));
    items.insert(2, Loot::Weapon(blade(0x21, 3)));
    items.insert(3, Loot::Weapon(blade(0x10, 1)));
    let pack = Pack {
        carried: carried(&[1, 2]),
        ..Pack::default()
    };
    assert_eq!(
        rearm(&items, &pack),
        vec![Deed::Wield { item: 2 }, Deed::Stow { item: 1 }]
    );
    let two = Pack {
        carried: carried(&[1, 2, 3]),
        hands: [1, 3, 0, 0],
        ..Pack::default()
    };
    assert_eq!(rearm(&items, &two), vec![Deed::Stow { item: 2 }]);
    assert!(rearm(&items, &Pack::default()).is_empty());
}

/// Tidying drops the weapon under 0.8 of its class's best (3/4) and a
/// carried piece a worn one makes redundant (half of it: 0.5 + 0.5 = 1),
/// keeping a special piece.
#[test]
fn tidying_leaves_the_redundant_gear() {
    let mut items = BTreeMap::new();
    items.insert(1, Loot::Weapon(blade(0x20, 3)));
    items.insert(2, Loot::Weapon(blade(0x20, 2)));
    items.insert(3, apparel_loot(piece(0, 100, 10, 1)));
    items.insert(4, apparel_loot(piece(0, 100, 20, 1)));
    items.insert(5, apparel_loot(piece(0, 100, 20, 1)));
    let mut special = piece(0, 50, 10, 1);
    special.special = 2;
    items.insert(6, apparel_loot(special));
    let pack = Pack {
        carried: carried(&[1, 2, 3, 6]),
        worn: vec![4, 5],
        ..Pack::default()
    };
    let deeds = tidy(&items, &pack, 99);
    assert_eq!(
        deeds,
        vec![
            Deed::Drop {
                item: 2,
                container: 99
            },
            Deed::Drop {
                item: 3,
                container: 99
            }
        ]
    );
}

#[test]
fn the_leader_is_a_living_player_in_sight() {
    let c = |attention, dead, player| Contact {
        attention,
        character: true,
        dead,
        player,
    };
    let contacts = [
        c(49.0, false, true),
        c(60.0, true, true),
        c(50.0, false, true),
    ];
    assert_eq!(find_leader(&contacts), Some(2));
    assert_eq!(find_leader(&[c(80.0, false, false)]), None);
}

fn scene_object(id: u32, kind: u32, corpse: Option<usize>) -> SceneObject {
    SceneObject {
        id,
        kind,
        searchable: true,
        fixed: false,
        at: Vec3::ZERO,
        half: Vec3::ONE,
        reserved: 0,
        corpse,
    }
}

/// A container, a corpse and a loose item, all in one room cell.
fn executor_scene() -> Scene {
    let mut scene = Scene::default();
    scene.grid.width = 1;
    scene.grid.height = 1;
    scene.grid.objects = vec![vec![0, 1, 2]];
    scene.objects.push(scene_object(
        40,
        object::CONTAINER_KIND | object::PRESENT,
        None,
    ));
    scene
        .objects
        .push(scene_object(60, object::PRESENT, Some(0)));
    scene.objects.push(scene_object(70, object::PRESENT, None));
    scene.containers.insert(
        40,
        Container {
            items: vec![7, 8],
            max_item: 4,
        },
    );
    scene.corpses.push(Corpse {
        character: true,
        dead: true,
        pack_id: 60,
        pack: Pack {
            carried: vec![
                Carried { item: 10, flags: 0 },
                Carried { item: 11, flags: 0 },
            ],
            worn: vec![9],
            hands: [10, 0, 0, 0],
            locked: false,
        },
        object: 1,
    });
    scene.items.insert(8, Loot::Other);
    scene
}

/// A take removes the first matching entry from its container, the worn
/// list, or the carried list; a worn take of the item in the corpse's grip
/// releases the hand and its carried entry instead; a loose object is
/// despawned from the scene and its room cell.
#[test]
fn the_executors_take_from_each_source() {
    let mut scene = executor_scene();
    let mut pack = Pack::default();
    let take = |object, source, item| Deed::Take {
        object,
        source,
        item,
    };
    assert!(apply_deed(
        &mut scene,
        &mut pack,
        &take(0, Source::Container, 7)
    ));
    assert_eq!(scene.containers[&40].items, vec![8]);
    assert!(!apply_deed(
        &mut scene,
        &mut pack,
        &take(0, Source::Container, 7)
    ));

    assert!(apply_deed(&mut scene, &mut pack, &take(1, Source::Worn, 9)));
    assert!(scene.corpses[0].pack.worn.is_empty());
    assert_eq!(scene.corpses[0].pack.hands[0], 10);
    assert!(apply_deed(
        &mut scene,
        &mut pack,
        &take(1, Source::Worn, 10)
    ));
    assert_eq!(scene.corpses[0].pack.hands[0], 0);
    assert_eq!(
        scene.corpses[0].pack.carried,
        vec![Carried { item: 11, flags: 0 }]
    );
    assert!(!apply_deed(
        &mut scene,
        &mut pack,
        &take(1, Source::Worn, 10)
    ));
    assert!(apply_deed(
        &mut scene,
        &mut pack,
        &take(1, Source::Carried, 11)
    ));
    assert!(scene.corpses[0].pack.carried.is_empty());

    assert!(apply_deed(
        &mut scene,
        &mut pack,
        &take(2, Source::Item, 70)
    ));
    assert_eq!(scene.objects[2].kind & object::PRESENT, 0);
    assert_eq!(scene.grid.objects[0], vec![0, 1]);
}

/// Wearing takes an item out of its source and adds it to the worn list;
/// carrying adds an entry; wielding fills the main hand; stowing the first
/// stowed slot.
#[test]
fn the_executors_carry_wear_wield_and_stow() {
    let mut scene = executor_scene();
    let mut pack = Pack::default();
    assert!(apply_deed(
        &mut scene,
        &mut pack,
        &Deed::Wear {
            object: 0,
            source: Source::Container,
            item: 8,
        }
    ));
    assert_eq!(scene.containers[&40].items, vec![7]);
    assert_eq!(pack.worn, vec![8]);
    assert!(apply_deed(&mut scene, &mut pack, &Deed::Carry { item: 11 }));
    assert_eq!(pack.carried.len(), 1);
    assert!(apply_deed(&mut scene, &mut pack, &Deed::Wield { item: 11 }));
    assert_eq!(pack.hands[0], 11);
    assert!(pack.carried.is_empty());
    assert!(apply_deed(&mut scene, &mut pack, &Deed::Stow { item: 12 }));
    assert_eq!(pack.hands[2], 12);
    scene.sizes.insert(12, 4);
    assert!(apply_deed(
        &mut scene,
        &mut pack,
        &Deed::Drop {
            item: 12,
            container: 40,
        }
    ));
    assert_eq!(scene.containers[&40].items, vec![7, 12]);
    assert_eq!(pack.hands[2], 0);
}

/// The object edits the deferred jobs ask for: an item's first container
/// entry goes and the list compacts; a worn item leaves the worn list, or,
/// when the corpse grips it in the main hand, the hand and its carried
/// entry; a carried item leaves its entry; a container takes an item only
/// when its size byte fits.
#[test]
fn the_small_object_edits_work_as_the_jobs_ask() {
    let mut container = Container {
        items: vec![3, 4, 5, 4],
        max_item: 9,
    };
    assert!(take_from_container(&mut container, 4));
    assert_eq!(container.items, vec![3, 5, 4]);
    assert!(!take_from_container(&mut container, 7));

    let mut pack = Pack {
        carried: vec![Carried { item: 1, flags: 0 }, Carried { item: 2, flags: 0 }],
        worn: vec![8, 9],
        hands: [2, 0, 0, 0],
        locked: false,
    };
    assert!(take_worn(&mut pack, 2));
    assert_eq!(pack.hands[0], 0);
    assert_eq!(pack.carried, vec![Carried { item: 1, flags: 0 }]);
    assert!(take_worn(&mut pack, 9));
    assert_eq!(pack.worn, vec![8]);
    assert!(!take_worn(&mut pack, 9));
    assert!(take_carried(&mut pack, 1));
    assert!(pack.carried.is_empty());
    assert!(!take_carried(&mut pack, 1));

    let mut small = Container {
        items: vec![],
        max_item: 5,
    };
    assert!(add_to_container(&mut small, 11, Some(5)));
    assert!(!add_to_container(&mut small, 12, Some(6)));
    assert!(!add_to_container(&mut small, 13, None));
    assert_eq!(small.items, vec![11]);
}

/// Dropping waits on a locked pack, needs the item in the pack and its
/// size no larger than the container's largest item; the item then leaves
/// the carried list for the container.
#[test]
fn dropping_into_a_container_needs_a_small_enough_item() {
    let mut scene = executor_scene();
    scene.containers.get_mut(&40).unwrap().items.clear();
    scene.sizes.insert(1, 4);
    scene.sizes.insert(2, 5);
    let mut pack = Pack {
        carried: vec![Carried { item: 1, flags: 0 }, Carried { item: 2, flags: 0 }],
        ..Pack::default()
    };
    let drop = |item| Deed::Drop {
        item,
        container: 40,
    };
    pack.locked = true;
    assert!(!apply_deed(&mut scene, &mut pack, &drop(1)));
    pack.locked = false;
    assert!(apply_deed(&mut scene, &mut pack, &drop(1)));
    assert!(!apply_deed(&mut scene, &mut pack, &drop(2)));
    assert!(!apply_deed(&mut scene, &mut pack, &drop(3)));
    assert_eq!(scene.containers[&40].items, vec![1]);
    assert_eq!(pack.carried, vec![Carried { item: 2, flags: 0 }]);
    assert_eq!(
        apply_deeds(
            &mut scene,
            &mut pack,
            &[Deed::Comment {
                phrase: "Look.".into()
            }]
        ),
        1
    );
}

/// One record section: its tag, its size counting the header, its body.
fn record_section(tag: u32, body: &[u8]) -> Vec<u8> {
    let mut out = tag.to_le_bytes().to_vec();
    out.extend((body.len() as u32 + 8).to_le_bytes());
    out.extend(body);
    out
}

/// An item record is read as the weighing reads it: the apparel body's
/// occupancy, coverage, protection and special bytes and the model
/// section's model byte; else the weapon section's flags, quality, damage,
/// thrust, tier byte and special word; else an operative or container part
/// makes it worth gathering; a record with none of them is not.
#[test]
fn an_item_record_reads_as_the_weighing_reads_it() {
    let mut apparel = vec![0u8; 0x218];
    apparel[0x10 + 8 * 3..0x10 + 8 * 4].copy_from_slice(&0x0102u64.to_le_bytes());
    apparel[0x150 + 4 * 9..0x150 + 4 * 10].copy_from_slice(&75u32.to_le_bytes());
    apparel[0x150 + 4 * 19..0x150 + 4 * 20].copy_from_slice(&120u32.to_le_bytes());
    apparel[0x1A0..0x1A4].copy_from_slice(&[10, 20, 30, 40]);
    apparel[0x214] = 2;
    let mut model = vec![0u8; 0x20];
    model[0x1C] = 51;
    let mut payload = record_section(section::MODEL, &model);
    payload.extend(record_section(section::APPAREL, &apparel));
    payload.extend(record_section(section::WEAPON, &[0; 0x50]));
    let Some(Loot::Apparel(piece)) = Loot::of_payload(&payload) else {
        panic!("apparel first");
    };
    assert_eq!(piece.occupancy[3], 0x0102);
    assert_eq!((piece.coverage[9], piece.coverage[19]), (75, 120));
    assert_eq!(piece.protection, [10, 20, 30, 40]);
    assert_eq!((piece.special, piece.model_mark), (2, 51));

    let mut weapon = vec![0u8; 0x50];
    weapon[..4].copy_from_slice(&0x11u32.to_le_bytes());
    weapon[0x0C] = 200;
    weapon[0x0D] = 5;
    weapon[0x33] = 70;
    weapon[0x42] = 0xE3;
    weapon[0x44..0x48].copy_from_slice(&7u32.to_le_bytes());
    let arm = Loot::of_payload(&record_section(section::WEAPON, &weapon));
    let expected = Arm {
        flags: 0x11,
        quality: 200,
        damage: 5,
        thrust: 70,
        tier: 0xE3,
        special: 7,
    };
    assert_eq!(arm, Some(Loot::Weapon(expected)));
    // Only the tier's low five bits count: 3 points.
    assert_eq!(expected.value(), blade_value(0x11, 200, 5, 70, 3));

    let short = Loot::of_payload(&record_section(section::WEAPON, &[1, 0, 0, 0]));
    assert_eq!(
        short,
        Some(Loot::Weapon(Arm {
            flags: 1,
            quality: 0,
            damage: 0,
            thrust: 0,
            tier: 0,
            special: 0,
        }))
    );
    assert_eq!(
        Loot::of_payload(&record_section(section::CONTAINER, &[0; 24])),
        Some(Loot::Other)
    );
    assert_eq!(
        Loot::of_payload(&record_section(section::OPERATIVE, &[0; 8])),
        Some(Loot::Other)
    );
    assert_eq!(
        Loot::of_payload(&record_section(section::MODEL, &model)),
        None
    );
    assert_eq!(Loot::of_payload(&[]), None);
}

/// The weapon value the item description routine gives, written out.
fn blade_value(flags: u32, quality: u8, damage: u8, thrust: u8, points: u8) -> f32 {
    let q = f32::from(quality) / 255.0;
    let d = f32::from(damage) / 255.0;
    let v = f32::from(thrust) / 140.0 + f32::from(points) * (q * q * 0.5 + 0.5) * (1.0 - d * 0.3);
    if flags & 0x30 == 0x10 { v * 0.8 } else { v }
}
