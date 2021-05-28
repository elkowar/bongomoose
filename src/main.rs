use bevy::{
    input::{keyboard::KeyboardInput, ElementState},
    prelude::*,
};
use bevy_kira_audio::{Audio, AudioChannel, AudioPlugin, AudioSource};

// const ARBITRARY_OSU_TIME_OFFSET: i32 = 60;
const ARBITRARY_OSU_TIME_OFFSET: i32 = 55;

const HIT_ACCURACY: i64 = 100;

// const OSU_FILE: &str = "songs/aotd/DragonForce - Ashes of the Dawn (Nao Tomori) [Futsuu].osu";
// const OGG_FILE: &str = "songs/aotd/audio.ogg";
// const OSU_FILE: &str = "songs/osu/Polyphia - O.D. (Melwoine) [Insane].osu";
// const OGG_FILE: &str = "songs/od.ogg";
const OSU_FILE: &str = "songs/insight/Haywyre - Insight (Twiggykun) [Normal].osu";
const OGG_FILE: &str = "songs/insight/audio.ogg";

#[allow(unused)]
macro_rules! some_or_return {
    ($x:expr) => {
        match $x {
            Some(x) => x,
            None => return,
        }
    };
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    Loading,
    Playing,
}

pub struct Block;

pub struct Timed(i32);

pub struct BongoMap {
    data: Vec<i32>,
}

pub struct AudioChannels {
    bong: AudioChannel,
    music: AudioChannel,
}

#[derive(Default)]
pub struct SongStartedAt(u128);

#[derive(Default)]
pub struct TimeInSong {
    millis: u128,
}

pub struct SongStarted;

fn hit_object_time(obj: &osuparse::HitObject) -> i32 {
    match obj {
        osuparse::HitObject::HitCircle(x) => x.time,
        osuparse::HitObject::Slider(x) => x.time,
        osuparse::HitObject::Spinner(x) => x.time,
        osuparse::HitObject::HoldNote(x) => x.time,
    }
}

fn slider_length(px_len: f32, slider_mult: f32, ms_per_beat: f32) -> f32 {
    px_len / (slider_mult * 100.0) * ms_per_beat
}

impl FromWorld for BongoMap {
    fn from_world(_world: &mut World) -> Self {
        let data = std::fs::read_to_string(format!("assets/{}", OSU_FILE)).unwrap();
        let beatmap = osuparse::parse_beatmap(&data).unwrap();
        let slider_mult = beatmap.difficulty.slider_multiplier;

        dbg!(beatmap.general.countdown);
        dbg!(beatmap.general.preview_time);
        dbg!(beatmap.general.audio_lead_in);

        let mut times = Vec::new();

        let mut timing_points = beatmap.timing_points.iter().peekable();
        let mut non_negative_ms_per_beat = timing_points.next().unwrap().ms_per_beat;
        let mut actual_ms_per_beat = non_negative_ms_per_beat;

        for object in beatmap.hit_objects {
            let time = hit_object_time(&object);
            if let Some(next_timing_point) = timing_points.peek() {
                if next_timing_point.offset <= time as f32 {
                    if next_timing_point.ms_per_beat >= 0.0 {
                        non_negative_ms_per_beat = next_timing_point.ms_per_beat;
                        actual_ms_per_beat = next_timing_point.ms_per_beat;
                    } else {
                        actual_ms_per_beat = (-next_timing_point.ms_per_beat as f32 / 100.0) * non_negative_ms_per_beat;
                    };
                    let _ = timing_points.next();
                }
            }
            times.push(time);
            match object {
                osuparse::HitObject::Slider(slider) => {
                    let slider_end = time + slider_length(slider.pixel_length, slider_mult, actual_ms_per_beat) as i32;
                    times.push(slider_end);
                }
                osuparse::HitObject::Spinner(spinner) => times.push(spinner.end_time),
                osuparse::HitObject::HoldNote(hold) => times.push(hold.end_time),
                _ => {}
            }
        }

        BongoMap { data: times.iter().map(|x| x + ARBITRARY_OSU_TIME_OFFSET).collect() }
    }
}

fn load_assets(asset_server: Res<AssetServer>) {
    let _: Handle<AudioSource> = asset_server.load(OGG_FILE);
    let _: Handle<AudioSource> = asset_server.load("sfx/bong.ogg");
}

fn handle_song_loading(
    mut commands: Commands,
    mut game_state: ResMut<State<GameState>>,
    asset_server: Res<AssetServer>,
    audio_channels: Res<AudioChannels>,
    time: Res<Time>,
    audio: Res<Audio>,
    mut song_started_at: ResMut<SongStartedAt>,
) {
    let handle = asset_server.get_handle(OGG_FILE);
    match asset_server.get_load_state(&handle) {
        bevy::asset::LoadState::Loading => {
            commands.insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)));
        }
        bevy::asset::LoadState::Loaded => {
            commands.insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)));
            audio.play_in_channel(handle, &audio_channels.music);
            song_started_at.0 = time.time_since_startup().as_millis();
            dbg!(song_started_at.0);
            game_state.set(GameState::Playing).unwrap();
        }
        _ => {}
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    let texture_handle = asset_server.load("sprites/hit_square.png");
    let mat_handle = materials.add(texture_handle.into());
    commands.spawn_bundle(SpriteBundle { material: mat_handle.clone(), ..Default::default() });
}

fn setup_audio(mut commands: Commands, audio: Res<Audio>) {
    let bong = AudioChannel::new("bong".to_string());
    let music = AudioChannel::new("music".to_string());
    audio.set_volume_in_channel(0.1, &music);
    audio.set_volume_in_channel(0.3, &bong);
    commands.insert_resource(AudioChannels { bong, music });
}

fn epic_block_time(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut bongo_map: ResMut<BongoMap>,
    time_in_song: Res<TimeInSong>,
) {
    let mut count = 0;
    let texture_handle = asset_server.load("sprites/smol_square.png");
    let mat_handle = materials.add(texture_handle.into());
    for time in bongo_map.data.iter().take_while(|x| **x as i64 - (time_in_song.millis as i64) < 2000) {
        count += 1;
        commands
            .spawn_bundle(SpriteBundle {
                material: mat_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(*time as f32 - time_in_song.millis as f32, 0.0, 0.0),
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

fn kill_blocks(mut commands: Commands, time_in_song: Res<TimeInSong>, query: Query<(Entity, &Timed), With<Block>>) {
    for (entity, timed) in query.iter() {
        if (timed.0 as i64) - (time_in_song.millis as i64) < -1000 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn keyboard_events(
    mut key_evr: EventReader<KeyboardInput>,
    time_in_song: Res<TimeInSong>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    audio_channels: Res<AudioChannels>,
    audio: Res<Audio>,
    query: Query<(Entity, &Timed)>,
) {
    for _ in key_evr.iter().filter(|x| x.state == ElementState::Pressed) {
        let closest_hit = query
            .iter()
            .map(|(entity, timed)| (entity, i64::abs(timed.0 as i64 - (time_in_song.millis as i64))))
            .min_by_key(|(_, diff)| *diff)
            .filter(|(_, diff)| *diff < HIT_ACCURACY);
        if let Some((entity, ..)) = closest_hit {
            commands.entity(entity).despawn_recursive();
            let bong_handle = asset_server.load("sfx/bong.ogg");
            audio.play_in_channel(bong_handle, &audio_channels.bong);
        }
    }
}

fn move_blocks(time_in_song: Res<TimeInSong>, mut query: Query<(&mut Transform, &Timed), With<Block>>) {
    for (mut transform, timed) in query.iter_mut() {
        transform.translation.x = timed.0 as f32 - time_in_song.millis as f32;
    }
}

fn update_time(time: ResMut<Time>, song_started_at: Res<SongStartedAt>, mut time_in_song: ResMut<TimeInSong>) {
    let started_at = song_started_at.0;
    let since_start = time.time_since_startup().as_millis() - started_at;
    time_in_song.millis = since_start;
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(AudioPlugin)
        .add_state(GameState::Loading)
        .init_resource::<BongoMap>()
        .init_resource::<SongStartedAt>()
        .init_resource::<TimeInSong>()
        .add_startup_system(setup.system())
        .add_startup_system(setup_audio.system())
        .add_startup_system(load_assets.system())
        .add_system_set(SystemSet::on_update(GameState::Loading).with_system(handle_song_loading.system()))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(epic_block_time.system())
                .with_system(update_time.system())
                .with_system(move_blocks.system())
                .with_system(kill_blocks.system())
                .with_system(keyboard_events.system()),
        )
        .run();
}
