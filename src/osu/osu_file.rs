/// General information about the beatmap
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub enum EventParams {
    Background {
        /// Location of the background image relative to the beatmap directory.
        /// Double quotes are usually included surrounding the filename, but they are not required.
        filename: String,
        /// Offset in osu! pixels from the center of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the center of the screen.
        /// If the offset is `0,0`, writing it is optional.
        x_offset: i32,
        /// Offset in osu! pixels from the center of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the center of the screen.
        /// If the offset is `0,0`, writing it is optional.
        y_offset: i32,
    },
    Video {
        /// Location of the video relative to the beatmap directory.
        /// Double quotes are usually included surrounding the filename, but they are not required.
        filename: String,
        /// Offset in osu! pixels from the center of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the center of the screen.
        /// If the offset is `0,0`, writing it is optional.
        x_offset: i32,
        /// Offset in osu! pixels from the center of the screen.
        /// For example, an offset of `50,100` would have the
        /// background shown 50 osu! pixels to the right and
        /// 100 osu! pixels down from the center of the screen.
        /// If the offset is `0,0`, writing it is optional.
        y_offset: i32,
    },
    Break {
        end_time: usize,
    },
}

/// Beatmap and storyboard graphic event
#[derive(Clone, Debug)]
pub struct Event {
    /// Type of the event. Some events may be referred to by either a name or a number.
    pub event_type: String,
    /// Start time of the event, in milliseconds from the beginning of the beatmap's audio.
    /// For events that do not use a start time, the default is `0`.
    pub start_time: usize,
    /// Extra parameters specific to the event's type.
    pub params: EventParams,
}

/// Timing and control points
#[derive(Clone, Debug)]
pub struct TimingPoint {
    /// Start time of the timing section, in milliseconds from the beginning of the beatmap's audio.
    /// The end of the timing section is the next timing point's time (or never, if this is the last timing point).
    pub time: usize,
    /// This property has two meanings:
    /// - For uninherited timing points, the duration of a beat, in milliseconds.
    /// - For inherited timing points, a negative inverse slider velocity multiplier, as a percentage.
    ///   For example, `-50` would make all sliders in this timing section twice as fast as `slider_multiplier`.
    pub beat_length: f64,
    /// Amount of beats in a measure. Inherited timing points ignore this property.
    pub meter: u32,
    /// Default sample set for hit objects (0 = beatmap default, 1 = normal, 2 = soft, 3 = drum).
    pub sample_set: u8,
    /// Custom sample index for hit objects. `0` indicates osu!'s default hitsounds.
    pub sample_index: u8,
    /// Volume percentage for hit objects.
    pub volume: u8,
    /// Whether or not the timing point is uninherited.
    pub uninherited: bool,
    /// Bit flags that give the timing point extra effects.
    pub effects: u32,
}

#[derive(Clone, Debug)]
pub struct Color {
    /// Red value in range `[0, 255]`.
    pub r: u8,
    /// Green value in range `[0, 255]`.
    pub g: u8,
    /// Blue value in range `[0, 255]`.
    pub b: u8,
    /// Alpha value in range `[0, 255]`.
    pub a: u8,
}

/// Combo and skin colors
#[derive(Clone, Debug)]
pub struct ColorsSection {
    /// Additive combo colors
    pub combo_colors: Vec<Color>,
    /// Additive slider track color
    pub slider_track_override: Color,
    /// Slider border color
    pub slider_border: Color,
}

#[derive(Clone, Debug)]
pub struct HitSampleSet {
    /// Sample set of the normal sound.
    pub normal_set: u8,
    /// Sample set of the whistle, finish, and clap sounds.
    pub addition_set: u8,
}

/// Anchor point used to construct a slider.
#[derive(Clone, Debug)]
pub struct SliderPoint {
    /// Type of curve used to construct this slider
    /// (B = bézier, C = centripetal catmull-rom, L = linear, P = perfect circle)
    /// If there is none, the point inherits the previous one.
    pub curve_type: Option<char>,
    /// Horizontal coordinate of the slider point.
    pub x: i32,
    /// Vertical coordinate of the slider point.
    pub y: i32,
}

/// Extra parameters specific to the object's type.
#[derive(Clone, Debug)]
pub enum HitObjectParams {
    HitCircle,
    Slider {
        /// Anchor points used to construct the slider. Each point is in the format `x:y`.
        /// 
        /// Note: the curve type is in this case individual to each point as Lazer allows
        /// sliders to have multiple points of different curve types shile Stable doesn't.
        /// This also seems to be completely bacwards-compatible, so no information is lost.
        /// 
        /// ## Example of slider curve points
        /// 
        /// ```
        /// P|213:282|P|257:269|234:254|P|158:283|129:306|B|39:234|L|57:105|68:173
        /// ```
        curve_points: Vec<SliderPoint>,
        /// Amount of times the player has to follow the slider's curve back-and-forth before
        /// the slider is complete. It can also be interpreted as the repeat count plus one.
        slides: u32,
        /// Visual length in osu! pixels of the slider.
        length: f64,
        /// Hitsounds that play when hitting edges of the slider's curve.
        /// The first sound is the one that plays when the slider is first clicked,
        /// and the last sound is the one that plays when the slider's end is hit.
        edge_hitsounds: Vec<u8>,
        /// Sample sets used for the edge hitounds.
        /// Each set is in the format `normal_set:addition_set`, with the same meaning as in the hitsounds section.
        edge_samplesets: Vec<HitSampleSet>,
    },
    /// Note: `x` and `y` do not affect spinners. They default to the center of the playfield, `256,192`.
    Spinner {
        /// End time of the spinner, in milliseconds from the beginning of the beatmap's audio.
        end_time: usize,
    },
    /// (osu!mania only)
    /// 
    /// Note: `x` determines the index of the column that the hold will be in.
    /// It is computed by `floor(x * column_count / 512)` and clamped between `0` and `column_count - 1`.
    /// 
    /// `y` does not affect holds. It defaults to the center of the playfield, `192`.
    Hold {
        /// End time of the hold, in milliseconds from the beginning of the beatmap's audio.
        end_time: usize,
    },
}

#[derive(Clone, Debug)]
pub struct HitSample {
    /// Sample set of the normal sound.
    pub normal_set: u8,
    /// Sample set of the whistle, finish, and clap sounds.
    pub addition_set: u8,
    /// Index of the sample. If this is `0`, the timing point's sample index will be used instead.
    pub index: u32,
    /// Volume of the sample from 1 to 100. If this is `0`, the timing point's volume will be used instead.
    pub volume: u8,
    /// Custom filename of the addition sound.
    pub filename: Option<String>,
}

/// Hit object
#[derive(Clone, Debug)]
pub struct HitObject {
    /// Horizontal position in osu! pixels of the object.
    pub x: i32,
    /// Vertical position in osu! pixels of the object.
    pub y: i32,
    /// Time when the object is to be hit, in milliseconds from the beginning of the beatmap's audio.
    pub time: usize,
    /// Bit flags indicating the type of the object.
    pub object_type: u8,
    /// Bit flags indicating the hitsound applied to the object.
    pub hit_sound: u8,
    /// Extra parameters specific to the object's type.
    pub object_params: HitObjectParams,
    /// Information about which samples are played when the object is hit.
    /// It is closely related to `hit_sound`.
    /// If it is not written, it defaults to `0:0:0:0:`.
    pub hit_sample: HitSample,
}

/// `.osu` is a human-readable file format containing information about a beatmap.
#[derive(Clone, Debug)]
pub struct OsuBeatmapFile {
    /// The first line of the file which specifies the file format version.
    /// For example, `osu file format v14` is the latest *stable* version.
    /// `osu file format v128` is the current version that osu!lazer uses.
    pub osu_file_format: u32,
    /// General information about the beatmap
    pub general: GeneralSection,
    /// Saved settings for the beatmap editor
    pub editor: EditorSection,
    /// Information used to identify the beatmap
    pub metadata: MetadataSection,
    /// Difficulty settings
    pub difficulty: DifficultySection,
    /// Beatmap and storyboard graphic events
    pub events: Vec<Event>,
    /// Timing and control points
    pub timing_points: Vec<TimingPoint>,
    /// Combo and skin colors
    pub colors: ColorsSection,
    /// Hit objects
    pub hit_objects: Vec<HitObject>,
}

