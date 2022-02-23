use gst::prelude::Cast;
use gst_pbutils::{Discoverer, DiscovererAudioInfo, DiscovererSubtitleInfo, DiscovererVideoInfo};

use crate::{
    constants::{
        composited_clips_projects_location, is_server, media_output_location,
        source_files_location, CHUNK_FILENAME_NUMBER_LENGTH,
    },
    node::PipeableType,
};

use super::{node::PipeableStreamType, ID};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ClipType {
    Source,
    Composited,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipIdentifier {
    pub id: ID,
    pub clip_type: ClipType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoStreamInfo {
    pub width: u32,
    pub height: u32,
    pub framerate: f64,
    pub bitrate: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioStreamInfo {
    pub sample_rate: u32,
    pub number_of_channels: u32,
    pub bitrate: u32,
    pub language: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubtitleStreamInfo {
    pub language: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipInfo {
    pub duration: u64,
    pub video_streams: Vec<VideoStreamInfo>,
    pub audio_streams: Vec<AudioStreamInfo>,
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
}

impl ClipInfo {
    pub fn to_pipeable_type(&self) -> PipeableType {
        PipeableType {
            video: self.video_streams.len() as i32,
            audio: self.audio_streams.len() as i32,
            subtitles: self.subtitle_streams.len() as i32,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SourceClipServerStatus {
    NeedsNewID,
    LocalOnly,
    Uploading,
    Uploaded,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SourceClip {
    // shared:
    pub id: ID,
    pub name: String,
    pub status: SourceClipServerStatus,
    pub info: Option<ClipInfo>,

    // sometimes shared:
    pub original_file_location: Option<String>,

    // server only
    pub file_location: Option<String>,
    pub original_device_id: Option<ID>,
    pub thumbnail_location: Option<String>,
}

impl SourceClip {
    pub fn get_clip_type(&self) -> PipeableType {
        if self.info.is_some() {
            let info = self.info.clone().unwrap();
            return info.to_pipeable_type();
        }
        return PipeableType {
            video: -1,
            audio: -1,
            subtitles: -1,
        };
    }

    pub fn get_file_info(filename: String) -> Result<ClipInfo, String> {
        println!("Filename: {}", filename);
        let file_location = format!("file:///{}", filename.replace("\\", "/"));
        println!("Location: {}", file_location);

        let discoverer = Discoverer::new(gst::ClockTime::from_seconds(10)).unwrap();

        let info = discoverer.discover_uri(&file_location);
        if info.is_err() {
            return Err(format!(
                "Error occurred when finding info!: {}",
                info.unwrap_err()
            ));
        }
        let info = info.unwrap();

        let duration = info.duration().unwrap().nseconds();
        // let exact_duration = (duration as f64) / (1000000000 as f64);
        let mut video_streams_vec = Vec::new();
        let video_streams = info.video_streams();
        for video_stream in video_streams {
            let video_info = video_stream.clone().downcast::<DiscovererVideoInfo>();
            if let Ok(video_info) = video_info {
                let width = video_info.width();
                let height = video_info.height();
                let (fps_num, fps_den): (i32, i32) = video_info.framerate().into();
                let (fps_num, fps_den): (f64, f64) = (fps_num.into(), fps_den.into());
                let fps = fps_num / fps_den;

                // let total_frames = exact_duration * fps_num / fps_den; // not 100% accurate
                // let total_frames = total_frames.round() as i64;

                let bitrate = video_info.bitrate();
                let video_stream = VideoStreamInfo {
                    width,
                    height,
                    bitrate,
                    framerate: fps,
                };
                video_streams_vec.push(video_stream);
            }
        }
        let mut audio_streams_vec = Vec::new();
        let audio_streams = info.audio_streams();
        for audio_stream in audio_streams {
            let audio_info = audio_stream.clone().downcast::<DiscovererAudioInfo>();
            if let Ok(audio_info) = audio_info {
                let bitrate = audio_info.bitrate();
                let sample_rate = audio_info.sample_rate();
                let language = audio_info
                    .language()
                    .unwrap_or(gst::glib::GString::from("und".to_string()))
                    .to_string();

                let num_channels = audio_info.channels();

                let audio_stream = AudioStreamInfo {
                    bitrate,
                    sample_rate,
                    language,
                    number_of_channels: num_channels,
                };
                audio_streams_vec.push(audio_stream);
            } else {
                println!("Could not cast to audio info");
            }
        }

        let subtitle_streams = info.subtitle_streams();
        let mut subtitle_streams_vec = Vec::new();
        for subtitle_stream in subtitle_streams {
            let subtitle_info = subtitle_stream.clone().downcast::<DiscovererSubtitleInfo>();
            if let Ok(subtitle_info) = subtitle_info {
                let language = subtitle_info
                    .language()
                    .unwrap_or(gst::glib::GString::from("und".to_string()))
                    .to_string();
                let subtitle_info = SubtitleStreamInfo { language };
                subtitle_streams_vec.push(subtitle_info);
            }
        }

        return Ok(ClipInfo {
            duration: duration / 1000000,
            audio_streams: audio_streams_vec,
            video_streams: video_streams_vec,
            subtitle_streams: subtitle_streams_vec,
        });
    }

    pub fn get_gstreamer_id(&self, stream_type: &PipeableStreamType, index: i32) -> String {
        format!(
            "source-clip-{}-{}-{}",
            self.id,
            stream_type.to_string(),
            index,
        )
    }

    pub fn get_server_url(&self) -> String {
        if is_server() {
            format!("file:///{}/{}", source_files_location(), self.id)
        } else {
            format!("file:///{}", self.file_location.as_ref().unwrap().clone())
        }
        .replace("\\", "/")
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompositedClip {
    pub id: ID,
    pub name: String,
}
impl CompositedClip {
    pub fn get_gstreamer_id(&self, stream_type: &PipeableStreamType, index: i32) -> String {
        format!(
            "composited-clip-{}-{}-{}",
            self.id,
            stream_type.to_string(),
            index,
        )
    }
    pub fn get_output_location(&self) -> String {
        format!("{}/composited-clip-{}", media_output_location(), self.id).replace("\\", "/")
    }

    pub fn get_output_location_template(&self) -> String {
        format!(
            "{}/segment%0{}d.mp4",
            self.get_output_location(),
            CHUNK_FILENAME_NUMBER_LENGTH
        )
    }

    pub fn get_location(&self) -> String {
        if is_server() {
            format!(
                "file:///{}/{}.xges",
                composited_clips_projects_location(),
                self.id
            )
            .replace("\\", "/")
        } else {
            panic!("Cannot get location of composited clip when not on server")
        }
    }
}
