use gstreamer::prelude::Cast;
use gstreamer_pbutils::{
  Discoverer, DiscovererAudioInfo, DiscovererInfo, DiscovererSubtitleInfo, DiscovererVideoInfo,
};
use serde_json::Value;

use crate::classes::node::PipeableType;

use super::{
  node::{PipeableStreamType, Type},
  pipeline::Pipeline,
  ID,
};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SourceClip {
  pub id: ID,
  pub name: String,
  pub file_location: String,
  pub thumbnail_location: Option<String>,
  pub info: Option<ClipInfo>,
}

impl SourceClip {
  pub fn get_clip_type(&self) -> PipeableType {
    if self.info.is_some() {
      let info = self.info.clone().unwrap();
      return PipeableType {
        video: info.video_streams.len() as i32,
        audio: info.audio_streams.len() as i32,
        subtitles: info.subtitle_streams.len() as i32,
      };
    }
    return PipeableType {
      video: -1,
      audio: -1,
      subtitles: -1,
    };
  }

  pub fn get_file_info(filename: String) -> Result<ClipInfo, String> {
    let file_location = format!("file:///{}", filename.replace("\\", "/"));

    let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(10)).unwrap();

    let info = discoverer.discover_uri(&file_location);
    if info.is_err() {
      return Err(format!(
        "Error occurred when finding info!: {}",
        info.unwrap_err()
      ));
    }
    let info = info.unwrap();

    let duration = info.duration().unwrap().nseconds();
    let exact_duration = (duration as f64) / (1000000000 as f64);
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

        let total_frames = exact_duration * fps_num / fps_den; // not 100% accurate
        let total_frames = total_frames.round() as i64;

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
          .unwrap_or(gstreamer::glib::GString::from("und".to_string()))
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
          .unwrap_or(gstreamer::glib::GString::from("und".to_string()))
          .to_string();
        let subtitle_info = SubtitleStreamInfo { language };
        subtitle_streams_vec.push(subtitle_info);
      }
    }

    return Ok(ClipInfo {
      duration,
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
}
