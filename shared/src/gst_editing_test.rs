use gst::prelude::*;

use ges::prelude::{ElementExt, *};

use std::{env, sync::mpsc};

use crate::{constants::projects_location, global::uniq_id};

/*

Don't use GESPipeline!

This only allows for one output pretty much, since it's only there for convenience.

To be able to utilise the caching algos mentioned, we need to use a standard GST Pipeline.


The GESTimeline is a GSTElement, hence we can use it in a standard GST pipeline.



Wait...
we can literally just put LOADs of tracks (outputs) on the timeline, and
have each thing we want to cache output to separate tracks, which can then be written to files for caching or whatever.









Look here: https://gitlab.freedesktop.org/gstreamer/gst-editing-services/-/blob/master/ges/ges-project.c
- use subprojects. Bit clunky, have to save to file, then load that file, but might be able to use custom ges protocol?






*/

fn configure_pipeline(pipeline: &ges::Pipeline, output_name: &str) {
    // Every audiostream piped into the encodebin should be encoded using opus.
    let audio_profile =
        gst_pbutils::EncodingAudioProfile::builder(&gst::Caps::builder("audio/x-opus").build())
            .build();

    // Every videostream piped into the encodebin should be encoded using vp8.
    let video_profile =
        gst_pbutils::EncodingVideoProfile::builder(&gst::Caps::builder("video/x-vp8").build())
            .build();

    // All streams are then finally combined into a webm container.
    let container_profile =
        gst_pbutils::EncodingContainerProfile::builder(&gst::Caps::builder("video/webm").build())
            .name("container")
            .add_profile(&video_profile)
            .add_profile(&audio_profile)
            .build();

    // Apply the EncodingProfile to the pipeline, and set it to render mode
    let output_uri = format!("{}.webm", output_name);
    pipeline
        .set_render_settings(&output_uri, &container_profile)
        .expect("Failed to set render settings");
    pipeline
        .set_mode(ges::PipelineFlags::RENDER)
        .expect("Failed to set pipeline to render mode");
}

pub fn main_loop(uri: &str) -> Result<(), glib::BoolError> {
    ges::init()?;

    let project_loc = format!("file:///{}\\test-project.xges", projects_location());
    let project = ges::Project::new(None);

    let timeline: ges::Timeline = project.extract().unwrap().dynamic_cast().unwrap();

    let clip_location = "file:///D:\\Data\\Libraries\\Videos\\Samples\\sample1.mp4";
    let layer = timeline.append_layer();

    let clip = ges::UriClip::new(clip_location).expect("Failed to create clip");

    layer.add_clip(&clip)?;

    let loc = format!("file:///{}\\test-timeline.xges", projects_location());

    timeline
        .save_to_uri(loc.as_str(), None as Option<&ges::Asset>, true)
        .unwrap();

    // project
    //     .save(
    //         &timeline,
    //         project_loc.as_str(),
    //         None as Option<&ges::Asset>,
    //         true,
    //     )
    //     .unwrap();

    let timeline = ges::Timeline::new_audio_video();
    let clip = ges::UriClip::new(project_loc.as_str()).unwrap();

    let layer = timeline.append_layer();
    layer.add_clip(&clip).unwrap();

    let loc = format!("file:///{}\\test-timeline2.xges", projects_location());

    timeline
        .save_to_uri(loc.as_str(), None as Option<&ges::Asset>, true)
        .unwrap();
    let timeline = ges::Timeline::from_uri(loc.as_str()).unwrap();

    println!("timeline: {:?}", timeline);

    return Ok(());
    let project_location = format!("{}\\test-project", projects_location());
    let project_location = format!("{}", project_location.replace("/", "\\"));

    println!("Project location: {}", project_location);

    let uri = format!("test-project.xges");
    let project = ges::Project::new(Some(uri.as_str()));

    let (tx, rx) = mpsc::channel();
    project.connect_loaded(move |project, timeline| {
        tx.send(());
    });
    /* Now extract a timeline from it */
    let timeline: ges::Timeline = project.extract().unwrap().dynamic_cast().unwrap();

    rx.recv().unwrap();

    let project_location = format!("test-project-tmp.xges");

    project
        .save(
            &timeline,
            project_location.as_str(),
            None as Option<&ges::Asset>,
            true,
        )
        .unwrap();

    // Create a new layer that will contain our timed clips.
    // let layer = timeline.append_layer();
    // // Load a clip from the given uri and add it to the layer.

    // let clip = ges::UriClip::new(uri).expect("Failed to create clip");

    // layer.add_clip(&clip)?;
    // // Add an effect to the clip's video stream.
    // let effect = ges::Effect::new("agingtv").expect("Failed to create effect");

    // clip.add(&effect)?;

    // println!(
    //     "Agingtv scratch-lines: {}",
    //     clip.child_property("scratch-lines")
    //         .unwrap()
    //         .serialize()
    //         .unwrap()
    // );

    // Retrieve the asset that was automatically used behind the scenes, to
    // extract the clip from.
    let x: Option<&ges::Asset> = None;
    project
        .save(&timeline, project_location.as_str(), x, true)
        .unwrap();
    Ok(())
}
