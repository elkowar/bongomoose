#![feature(iter_advance_by)]
use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioChannel, AudioPlugin, AudioSource};

const OSU_FILE: &str = "songs/osu/od.osu";
const OGG_FILE: &str = "songs/od.ogg";

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
pub struct SongStartedAt(Option<u128>);

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
                    let _ = timing_points.advance_by(1);
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
        BongoMap { data: times }
    }
}

fn load_assets(asset_server: Res<AssetServer>) {
    let _: Handle<AudioSource> = asset_server.load(OGG_FILE);
    let _: Handle<AudioSource> = asset_server.load("sfx/bong.ogg");
}

fn handle_song_loading(
    asset_server: Res<AssetServer>,
    channels: Res<AudioChannels>,
    time: Res<Time>,
    audio: Res<Audio>,
    mut song_started_at: ResMut<SongStartedAt>,
) {
    if song_started_at.0.is_some() {
        return;
    }
    let handle = asset_server.get_handle(OGG_FILE);
    match asset_server.get_load_state(&handle) {
        bevy::asset::LoadState::NotLoaded => {}
        bevy::asset::LoadState::Loading => {}
        bevy::asset::LoadState::Failed => {}
        bevy::asset::LoadState::Loaded => {
            song_started_at.0 = Some(time.time_since_startup().as_millis());
            audio.play_in_channel(handle, &channels.music);
            // writer.send(SongStarted);
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
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
    time: Res<Time>,
    mut bongo_map: ResMut<BongoMap>,
    song_started_at: Res<SongStartedAt>,
) {
    let song_started_at = match song_started_at.0 {
        Some(time) => time,
        None => return,
    };

    let since_start = time.time_since_startup().as_millis() - song_started_at;
    let mut count = 0;
    let texture_handle = asset_server.load("sprites/smol_square.png");
    let mat_handle = materials.add(texture_handle.into());
    for time in bongo_map.data.iter().take_while(|x| **x as i64 - (since_start as i64) < 1000) {
        count += 1;
        commands
            .spawn_bundle(SpriteBundle {
                material: mat_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(*time as f32 - since_start as f32, 0.0, 0.0),
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
fn kill_blocks(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    audio_channels: Res<AudioChannels>,
    time: Res<Time>,
    song_started_at: Res<SongStartedAt>,
    audio: Res<Audio>,
    query: Query<(Entity, &Timed), With<Block>>,
) {
    let song_started_at = match song_started_at.0 {
        Some(time) => time,
        None => return,
    };

    let since_start = time.time_since_startup().as_millis() - song_started_at;
    let mut did_kill = false;
    for (entity, timed) in query.iter() {
        if (timed.0 as u128) < since_start {
            commands.entity(entity).despawn();
            did_kill = true;
        }
    }
    if did_kill {
        let bong_handle = asset_server.load("sfx/bong.ogg");
        audio.play_in_channel(bong_handle, &audio_channels.bong);
    }
}

fn move_blocks(time: Res<Time>, song_started_at: Res<SongStartedAt>, mut query: Query<(&mut Transform, &Timed), With<Block>>) {
    let song_started_at = match song_started_at.0 {
        Some(time) => time,
        None => return,
    };

    let since_start = time.time_since_startup().as_millis() - song_started_at;
    for (mut transform, timed) in query.iter_mut() {
        transform.translation.x = timed.0 as f32 - since_start as f32;
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(AudioPlugin)
        .init_resource::<BongoMap>()
        .init_resource::<SongStartedAt>()
        .add_startup_system(setup.system())
        .add_startup_system(setup_audio.system())
        .add_startup_system(load_assets.system())
        .add_system(epic_block_time.system())
        .add_system(move_blocks.system())
        .add_system(kill_blocks.system())
        .add_system(handle_song_loading.system())
        .run();
}
