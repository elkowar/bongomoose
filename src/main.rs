use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioPlugin};

pub struct Block;

pub struct Timed(i32);

pub struct BongoMap {
    data: Vec<i32>,
}

impl FromWorld for BongoMap {
    fn from_world(_world: &mut World) -> Self {
        let data = std::fs::read_to_string("assets/songs/osu/Bleed.osu").unwrap();
        let beatmap = osuparse::parse_beatmap(&data).unwrap();
        // dbg!(beatmap.timing_points.iter().map(|x| x.ms_per_beat).collect::<Vec<f32>>());

        let mut times = Vec::new();

        for object in beatmap.hit_objects {
            match object {
                osuparse::HitObject::HitCircle(x) => times.push(x.time),
                osuparse::HitObject::Slider(x) => times.push(x.time),
                osuparse::HitObject::Spinner(x) => times.push(x.time),
                osuparse::HitObject::HoldNote(x) => {
                    times.push(x.time);
                    times.push(x.end_time);
                }
            }
        }
        BongoMap { data: times }
    }
}

// fn setup_osu(mut commands: Commands, asset_server: Res<AssetServer>) {}

fn setup_audio(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    audio.play(asset_server.load("songs/bleed.ogg"));
    audio.set_volume(0.2);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    // let texture_handle = asset_server.load("sprites/square.png");
    // commands.spawn_bundle(SpriteBundle { material: materials.add(texture_handle.into()), ..Default::default() }).insert(Block);
}

fn epic_block_time(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    time: Res<Time>,
    mut bongo_map: ResMut<BongoMap>,
) {
    let since_start = time.time_since_startup().as_millis();
    let mut count = 0;
    let texture_handle = asset_server.load("sprites/smol_square.png");
    let mat_handle = materials.add(texture_handle.into());
    for time in bongo_map.data.iter().take_while(|x| **x as u128 - since_start < 1000) {
        count += 1;
        commands
            .spawn_bundle(SpriteBundle {
                material: mat_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(since_start as f32 - *time as f32, 0.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Block)
            .insert(Timed(*time));
    }
    for n in 0..count {
        bongo_map.data.remove(n);
    }
}

// TODO get world thingy for this
fn kill_blocks(mut commands: Commands, time: Res<Time>, mut query: Query<(&Transform, &Timed), With<Block>>) {
    let since_start = time.time_since_startup().as_millis();
    query.iter().filter(|(trans, timed)| (timed.0 as u128) < since_start).for_each(|(trans, time)| {});
}

fn move_blocks(time: Res<Time>, mut query: Query<(&mut Transform, &Timed), With<Block>>) {
    let since_start = time.time_since_startup().as_millis();
    for (mut transform, timed) in query.iter_mut() {
        transform.translation.x = timed.0 as f32 - since_start as f32;
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(AudioPlugin)
        .init_resource::<BongoMap>()
        .add_startup_system(setup.system())
        .add_startup_system(setup_audio.system())
        //.add_startup_system(setup_osu.system())
        .add_system(epic_block_time.system())
        .add_system(move_blocks.system())
        .run();
}

// fn setup_midi(mut commands: Commands, asset_server: Res<AssetServer>) {
// let mut bytes = Vec::new();
// std::fs::File::open("assets/songs/bleed.mid").unwrap().read_to_end(&mut bytes).unwrap();
// let smf = Smf::parse(&bytes).unwrap();
// let track = smf.tracks.first().unwrap();

// let ticks_per_beat = match smf.header.timing {
// midly::Timing::Metrical(x) => x,
// midly::Timing::Timecode(..) => {
// panic!("not implemented");
//};

// for event in track.iter() {
// match event.kind {
// midly::TrackEventKind::Midi { channel, message } => {}
// midly::TrackEventKind::SysEx(_) => {}
// midly::TrackEventKind::Escape(_) => {}
// midly::TrackEventKind::Meta(meta) => {
// dbg!(meta);
//}
