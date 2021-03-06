use bevy::{
    input::{keyboard::KeyboardInput, ElementState},
    prelude::*,
};
use bevy_kira_audio::{Audio, AudioChannel, AudioPlugin, AudioSource};
use circular_queue::CircularQueue;

mod utils;

// const ARBITRARY_OSU_TIME_OFFSET: i32 = 60;
const ARBITRARY_OSU_TIME_OFFSET: i32 = 55;

const HIT_ACCURACY: i64 = 200;

const OSU_FILE: &str = "songs/aotd/DragonForce - Ashes of the Dawn (Nao Tomori) [Futsuu].osu";
const OGG_FILE: &str = "songs/aotd/audio.ogg";
// const OSU_FILE: &str = "songs/osu/Polyphia - O.D. (Melwoine) [Insane].osu";
// const OGG_FILE: &str = "songs/od.ogg";
// const OSU_FILE: &str = "songs/insight/Haywyre - Insight (Twiggykun) [Normal].osu";
// const OGG_FILE: &str = "songs/insight/audio.ogg";
// const OSU_FILE: &str = "songs/osu_bleed/Bleed.osu";
// const OGG_FILE: &str = "songs/bleed.ogg";

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

pub struct Block(bool);
pub struct AccuracyIndicator;
pub struct TotalErrorIndicator;

pub struct Timed(i32);

#[derive(Debug, Clone, Copy)]
pub struct HitBlock {
    time: i32,
    strong: bool,
}

impl HitBlock {
    fn at_time(time: i32) -> Self {
        HitBlock { time, strong: false }
    }
}

pub struct BongoMap {
    data: Vec<HitBlock>,
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

#[derive(Debug)]
pub struct Accuracy {
    last_hits: CircularQueue<i32>,
    total_error: i32,
    missed: i32,
}

impl Default for Accuracy {
    fn default() -> Self {
        Accuracy { last_hits: CircularQueue::with_capacity(10), total_error: 0, missed: 0 }
    }
}

impl Accuracy {
    fn average_accuracy(&self) -> i32 {
        if self.last_hits.is_empty() {
            0
        } else {
            self.last_hits.iter().sum::<i32>() / self.last_hits.len() as i32
        }
    }

    fn add_time(&mut self, time: i32) {
        self.last_hits.push(time);
        self.total_error = self.total_error + time.abs();
    }
}

fn hit_object_to_block(obj: &osuparse::HitObject) -> HitBlock {
    match obj {
        osuparse::HitObject::HitCircle(x) => HitBlock { time: x.time, strong: x.hitsound != 0 },
        osuparse::HitObject::Slider(x) => HitBlock { time: x.time, strong: false },
        osuparse::HitObject::Spinner(x) => HitBlock { time: x.time, strong: false },
        osuparse::HitObject::HoldNote(x) => HitBlock { time: x.time, strong: false },
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

        let mut hit_blocks = Vec::new();

        let mut timing_points = beatmap.timing_points.iter().peekable();
        let mut non_negative_ms_per_beat = timing_points.next().unwrap().ms_per_beat;
        let mut actual_ms_per_beat = non_negative_ms_per_beat;

        for object in beatmap.hit_objects {
            let hit_block = hit_object_to_block(&object);
            if let Some(next_timing_point) = timing_points.peek() {
                if next_timing_point.offset <= hit_block.time as f32 {
                    if next_timing_point.ms_per_beat >= 0.0 {
                        non_negative_ms_per_beat = next_timing_point.ms_per_beat;
                        actual_ms_per_beat = next_timing_point.ms_per_beat;
                    } else {
                        actual_ms_per_beat = (-next_timing_point.ms_per_beat as f32 / 100.0) * non_negative_ms_per_beat;
                    };
                    let _ = timing_points.next();
                }
            }
            hit_blocks.push(hit_block);
            match object {
                osuparse::HitObject::Slider(slider) => {
                    let slider_end = hit_block.time + slider_length(slider.pixel_length, slider_mult, actual_ms_per_beat) as i32;
                    hit_blocks.push(HitBlock::at_time(slider_end));
                }
                osuparse::HitObject::Spinner(spinner) => hit_blocks.push(HitBlock::at_time(spinner.end_time)),
                osuparse::HitObject::HoldNote(hold) => hit_blocks.push(HitBlock::at_time(hold.end_time)),
                _ => {}
            }
        }

        hit_blocks.iter_mut().for_each(|x| x.time = x.time + ARBITRARY_OSU_TIME_OFFSET);

        BongoMap { data: hit_blocks }
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

    let texture_handle = asset_server.load("sprites/smol_square.png");
    let mat_handle = materials.add(texture_handle.into());
    commands
        .spawn_bundle(SpriteBundle {
            material: mat_handle.clone(),
            transform: Transform { translation: Vec3::new(0.0, 150.0, 0.0), ..Default::default() },
            ..Default::default()
        })
        .insert(AccuracyIndicator);

    let font_handle = asset_server.load("fonts/DejaVuSans.ttf");

    commands
        .spawn_bundle(Text2dBundle {
            transform: Transform { translation: Vec3::new(0.0, 250.0, 0.0), ..Default::default() },
            text: Text {
                sections: vec![TextSection {
                    value: String::new(),
                    style: TextStyle { font: font_handle, ..Default::default() },
                }],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(TotalErrorIndicator);
}

fn setup_audio(mut commands: Commands, audio: Res<Audio>) {
    let bong = AudioChannel::new("bong".to_string());
    let music = AudioChannel::new("music".to_string());
    audio.set_volume_in_channel(0.1, &music);
    audio.set_volume_in_channel(0.3, &bong);
    commands.insert_resource(AudioChannels { bong, music });
}

fn render_accuracy(
    accuracy: Res<Accuracy>,
    mut accuracy_indicator_query: Query<(&mut Transform, &AccuracyIndicator)>,
    mut total_error_indicator_query: Query<(&mut Text, &TotalErrorIndicator)>,
) {
    for (mut trans, _) in accuracy_indicator_query.iter_mut() {
        trans.translation.x = accuracy.average_accuracy() as f32;
    }
    for (mut text, _) in total_error_indicator_query.iter_mut() {
        text.sections[0].value = format!("{} / {}", accuracy.missed, accuracy.total_error);
    }
}

fn spawn_blocks(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut bongo_map: ResMut<BongoMap>,
    time_in_song: Res<TimeInSong>,
) {
    let mut count = 0;
    let texture_handle = asset_server.load("sprites/smol_square.png");
    let mat_handle = materials.add(texture_handle.into());
    for block in bongo_map.data.iter().take_while(|x| x.time as i64 - (time_in_song.millis as i64) < 2000) {
        count += 1;
        commands
            .spawn_bundle(SpriteBundle {
                material: mat_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(block.time as f32 - time_in_song.millis as f32, 0.0, 0.0),
                    scale: if block.strong { Vec3::new(2.0, 2.0, 1.0) } else { Vec3::new(1.0, 1.0, 1.0) },
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Block(block.strong))
            .insert(Timed(block.time));
    }
    for n in 0..count {
        bongo_map.data.remove(n);
    }
}

fn kill_blocks(
    mut commands: Commands,
    time_in_song: Res<TimeInSong>,
    mut accuracy: ResMut<Accuracy>,
    query: Query<(Entity, &Timed)>,
) {
    for (entity, timed) in query.iter() {
        if (timed.0 as i64) - (time_in_song.millis as i64) < -HIT_ACCURACY {
            commands.entity(entity).despawn_recursive();
            accuracy.missed = accuracy.missed + 1;
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
    mut accuracy: ResMut<Accuracy>,
) {
    for _ in key_evr.iter().filter(|x| x.state == ElementState::Pressed) {
        let closest_hit = query
            .iter()
            .min_by_key(|(_, timed)| i64::abs(timed.0 as i64 - (time_in_song.millis as i64)))
            .filter(|(_, timed)| i64::abs(timed.0 as i64 - (time_in_song.millis as i64)) < HIT_ACCURACY);
        if let Some((entity, timed)) = closest_hit {
            accuracy.add_time((timed.0 as i64 - (time_in_song.millis as i64)) as i32);
            commands.entity(entity).despawn_recursive();
            // let bong_handle = asset_server.load("sfx/bong.ogg");
            // audio.play_in_channel(bong_handle, &audio_channels.bong);
        }
    }
}

fn move_blocks(time_in_song: Res<TimeInSong>, mut query: Query<(&mut Transform, &Timed)>) {
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
        .init_resource::<Accuracy>()
        .add_startup_system(setup.system())
        .add_startup_system(setup_audio.system())
        .add_startup_system(load_assets.system())
        .add_system_set(SystemSet::on_update(GameState::Loading).with_system(handle_song_loading.system()))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(spawn_blocks.system())
                .with_system(update_time.system())
                .with_system(move_blocks.system())
                .with_system(kill_blocks.system())
                .with_system(keyboard_events.system())
                .with_system(render_accuracy.system()),
        )
        .run();
}
