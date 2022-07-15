/// General information about the beatmap
pub struct GeneralSection {
    /// Location of the audio file relative to the current folder
    pub audio_filename: String,
    /// Milliseconds of silence before the audio starts playing
    pub audio_lead_in: i32,
    /// Deprecated
    pub audio_hash: String,
    /// Time in milliseconds when the audio preview should start
    pub preview_time: i32,
    /// Speed of the countdown before the first hit object
    /// - 0 = no countdown
    /// - 1 = normal
    /// - 2 = half
    /// - 3 = double
    pub countdown: i32,
    /// Sample set that will be used if timing points do not override it (Normal, Soft, Drum)
    pub sample_set: String,
    /// Multiplier for the threshold in time where hit objects placed close together stack (0–1)
    pub stack_leniency: f64,
    /// - 0 = osu!
    /// - 1 = osu!taiko
    /// - 2 = osu!catch
    /// - 3 = osu!mania
    pub mode: u8,
    /// Whether or not breaks have a letterboxing effect
    pub letterbox_in_breaks: bool,
    /// Deprecated
    pub story_fire_in_front: bool,
    /// Whether or not the storyboard can use the user's skin images
    pub use_skin_sprites: bool,
    /// Deprecated
    pub always_show_playfield: bool,
    /// Draw order of hit circle overlays compared to hit numbers
    /// - NoChange = use skin setting,
    /// - Below = draw overlays under numbers
    /// - Above = draw overlays on top of numbers)
    pub overlay_position: String,
    /// Preferred skin to use during gameplay
    pub skin_preference: String,
    /// Whether or not a warning about flashing colours should be shown at the beginning of the map
    pub epilepsy_warning: bool,
    /// Time in beats that the countdown starts before the first hit object
    pub countdown_offset: i32,
    /// Whether or not the "N+1" style key layout is used for osu!mania
    pub special_style: bool,
    /// Whether or not the storyboard allows widescreen viewing
    pub widescreen_storyboard: bool,
    /// Whether or not sound samples will change rate when playing with speed-changing mods
    pub samples_match_playback_rate: bool,
}

/// Saved settings for the beatmap editor
pub struct EditorSection {
    /// Time in milliseconds of bookmarks
    pub bookmarks: Vec<i32>,
    /// Distance snap multiplier
    pub distance_spacing: f64,
    /// Beat snap divisor
    pub beat_divisor: f64,
    /// Grid size
    pub grid_size: i32,
    /// Scale factor for the object timeline
    pub timeline_zoom: f64,
}

/// Information used to identify the beatmap
pub struct MetadataSection {
    /// Romanised song title
    pub title: String,
    /// Song title
    pub title_unicode: String,
    /// Romanised song artist
    pub artist: String,
    /// Song artist
    pub artist_unicode: String,
    /// Beatmap creator
    pub creator: String,
    /// Difficulty name
    pub version: String,
    /// Original media the song was produced for
    pub source: String,
    /// Search terms
    pub tags: Vec<String>,
    /// Difficulty ID
    pub beatmap_id: u32,
    /// Beatmap ID
    pub beatmap_set_id: u32,
}

/// Difficulty settings
pub struct DifficultySection {
    /// HP setting (0–10)
    pub hp_drain_rate: f32,
    /// CS setting (0–10)
    pub circle_size: f32,
    /// OD setting (0–10)
    pub overall_difficulty: f32,
    /// AR setting (0–10)
    pub approach_rate: f32,
    /// Base slider velocity in hundreds of osu! pixels per beat
    pub slider_multiplier: f32,
    /// Amount of slider ticks per beat
    pub slider_tick_rate: f32,
}

pub enum EventParams {
    Background {
        /// Location of the background image relative to the beatmap directory.
        /// Double quotes are usually included surrounding the filename, but they are not required.
        filename: String,
        /// Offset in osu! pixels from the centre of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the centre of the screen.
        /// If the offset is `0,0`, writing it is optional.
        x_offset: i32,
        /// Offset in osu! pixels from the centre of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the centre of the screen.
        /// If the offset is `0,0`, writing it is optional.
        y_offset: i32,
    },
    Video {
        /// Location of the video relative to the beatmap directory.
        /// Double quotes are usually included surrounding the filename, but they are not required.
        filename: String,
        /// Offset in osu! pixels from the centre of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the centre of the screen.
        /// If the offset is `0,0`, writing it is optional.
        x_offset: i32,
        /// Offset in osu! pixels from the centre of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the centre of the screen.
        /// If the offset is `0,0`, writing it is optional.
        y_offset: i32,
    },
    Break {
        end_time: usize,
    },
}

/// Beatmap and storyboard graphic event
pub struct Event {
    start_time: usize,
    params: EventParams,
}

/// Timing and control points
pub struct TimingPoint {
    time: usize,
    beat_length: f64,
    meter: u32,
    sample_set: u8,
    sample_index: u8,
    volume: u8,
    uninherited: bool,
    effects: u32,
}

pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

/// Combo and skin colors
pub struct ColorsSection {
    combo_colors: Vec<Color>,
    slider_track_override: Color,
    slider_border: Color,
}

pub struct HitObjectParams {
    // TODO
}

pub struct HitSample {
    normal_set: u32,
    addition_set: u32,
    index: u32,
    volume: u8,
    filename: Option<String>,
}

/// Hit object
pub struct HitObject {
    x: i32,
    y: i32,
    time: usize,
    object_type: u32,
    hit_sound: u32,
    object_params: HitObjectParams,
    hit_sample: HitSample
}

pub struct BeatmapFile {
    osu_file_format: u32,
    general: GeneralSection,
    editor: EditorSection,
    metadata: MetadataSection,
    difficulty: DifficultySection,
    events: Vec<Event>,
    timing_points: Vec<TimingPoint>,
    colors: ColorsSection,
    hit_objects: Vec<HitObject>,
}
