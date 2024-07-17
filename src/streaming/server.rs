use std::io;
use std::sync::Arc;

use gst::glib;
use gst::prelude::*;
use gstreamer as gst;
use thiserror::Error;

use crate::connection::server::ConnectionServer;

#[derive(Error, Debug)]
pub enum StreamingServerError {
    #[error("GStreamer init error: {0}")]
    GStreamerInitError(#[from] glib::Error),

    #[error("GStreamer element error: {0}")]
    GStreamerElementCreationError(#[from] glib::BoolError),

    #[error("Websocket error: {0}")]
    WebsocketError(#[from] io::Error),
}

pub struct StreamingServer {
    source: gst::Element,
    pipeline: gst::Pipeline,
    connection_server: ConnectionServer,
}

impl StreamingServer {
    pub fn new() -> Result<Self, StreamingServerError> {
        gst::init()?;

        let source = gst::ElementFactory::make("ximagesrc")
            .property("use-damage", false)
            .build()?;

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw")
                    .field("framerate", &gst::Fraction::new(30, 1))
                    .build(),
            )
            .build()?;

        let videoconvert = gst::ElementFactory::make("videoconvert").build()?;

        let enc = gst::ElementFactory::make("x264enc")
            .property_from_str("tune", "zerolatency")
            .build()?;

        let pay = gst::ElementFactory::make("rtph264pay").build()?;

        let multiudpsink = gst::ElementFactory::make("multiudpsink").build()?;

        let tee = gst::ElementFactory::make("tee").build()?;

        let queue1 = gst::ElementFactory::make("queue").build()?;
        let queue2 = gst::ElementFactory::make("queue").build()?;

        let videoconvert2 = gst::ElementFactory::make("videoconvert").build()?;
        let videosink = gst::ElementFactory::make("autovideosink").build()?;

        let pipeline = gst::Pipeline::with_name("send-pipeline");

        pipeline.add_many(&[
            &source,
            &capsfilter,
            &tee,
            &queue1,
            &queue2,
            &videoconvert,
            &enc,
            &pay,
            &multiudpsink,
            &videoconvert2,
            &videosink,
        ])?;

        gst::Element::link_many(&[
            &source,
            &capsfilter,
            &tee,
            &queue1,
            &videoconvert,
            &enc,
            &pay,
            &multiudpsink,
        ])?;

        gst::Element::link_many(&[&tee, &queue2, &videoconvert2, &videosink])?;

        let multiudpsink = Arc::new(multiudpsink);
        let multiudpsink2 = multiudpsink.clone();
        let connection_server = ConnectionServer::new(
            move |ip| {
                multiudpsink.emit_by_name_with_values("add", &[ip.into(), 9001.into()]);
                println!("Connected: {}", ip);
            },
            move |ip| {
                multiudpsink2.emit_by_name_with_values("remove", &[ip.into(), 9001.into()]);
                println!("Disconnected: {}", ip);
            },
        )?;

        Ok(Self {
            source,
            pipeline,
            connection_server,
        })
    }

    pub fn start(&self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Playing).map(|_| ())
    }

    pub fn pause(&self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Paused).map(|_| ())
    }

    pub fn close(self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Null)?;
        self.connection_server.stop();
        Ok(())
    }

    pub fn capture_resize(&self, startx: u32, starty: u32, endx: u32, endy: u32) {
        self.source.set_property("startx", startx);
        self.source.set_property("starty", starty);
        self.source.set_property("endx", endx);
        self.source.set_property("endy", endy);
    }

    pub fn capture_fullscreen(&self) {
        self.capture_resize(0, 0, 0, 0);
    }
}
