use byte_slice_cast::*;
use std::io;
use std::sync::Arc;

use gst::prelude::*;
use gst::{element_error, glib};
use gstreamer as gst;
use gstreamer_app as gst_app;
use thiserror::Error;

use crate::connection::server::ConnectionServer;

#[derive(Error, Debug)]
pub enum StreamingServerError {
    #[error("GStreamer init error: {0}")]
    GStreamerInitError(#[from] glib::Error),

    #[error("GStreamer element error: {0}")]
    GStreamerElementCreationError(#[from] glib::BoolError),

    #[error("GStreamer state change error: {0}")]
    GStreamerStateChangeError(#[from] gst::StateChangeError),

    #[error("Websocket error: {0}")]
    WebsocketError(#[from] io::Error),
}

pub struct StreamingServer {
    source: gst::Element,
    pipeline: gst::Pipeline,
    _connection_server: ConnectionServer,
}

impl StreamingServer {
    pub fn new(
        mut image_parser: impl FnMut(&[u8]) + Send + 'static,
    ) -> Result<Self, StreamingServerError> {
        gst::init()?;

        let source = if cfg!(target_os = "windows") {
            gst::ElementFactory::make("d3d11screencapturesrc")
                .property("show-cursor", true)
                .build()?
        } else if cfg!(target_os = "linux") {
            gst::ElementFactory::make("ximagesrc")
                .property("use-damage", false)
                .build()?
        } else {
            todo!()
        };

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
        let jpegenc = gst::ElementFactory::make("jpegenc").build()?;
        let videosink = gst_app::AppSink::builder()
            .max_buffers(1)
            .caps(&gst::Caps::builder("image/jpeg").build())
            .build();

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
            &jpegenc,
            videosink.upcast_ref(),
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

        gst::Element::link_many(&[
            &tee,
            &queue2,
            &videoconvert2,
            &jpegenc,
            videosink.upcast_ref(),
        ])?;

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

        videosink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or_else(|| {
                        element_error!(
                            appsink,
                            gst::ResourceError::Failed,
                            ("Failed to get buffer from appsink")
                        );

                        gst::FlowError::Error
                    })?;

                    let map = buffer.map_readable().map_err(|_| {
                        element_error!(
                            appsink,
                            gst::ResourceError::Failed,
                            ("Failed to map buffer readable")
                        );

                        gst::FlowError::Error
                    })?;

                    let samples = map.as_slice_of::<u8>().map_err(|_| {
                        element_error!(
                            appsink,
                            gst::ResourceError::Failed,
                            ("Failed to interpret buffer as bytes")
                        );

                        gst::FlowError::Error
                    })?;

                    image_parser(samples);

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        Ok(Self {
            source,
            pipeline,
            _connection_server: connection_server,
        })
    }

    pub fn start(&self) -> Result<(), StreamingServerError> {
        Ok(self.pipeline.set_state(gst::State::Playing).map(|_| ())?)
    }

    pub fn pause(&self) -> Result<(), StreamingServerError> {
        Ok(self.pipeline.set_state(gst::State::Paused).map(|_| ())?)
    }

    #[cfg(target_os = "linux")]
    pub fn capture_resize(&self, startx: u32, starty: u32, endx: u32, endy: u32) {
        self.source.set_property("startx", startx);
        self.source.set_property("starty", starty);
        self.source.set_property("endx", endx);
        self.source.set_property("endy", endy);
    }

    #[cfg(target_os = "windows")]
    pub fn capture_resize(&self, startx: u32, starty: u32, endx: u32, endy: u32) {
        self.source.set_property("crop-x", startx);
        self.source.set_property("crop-y", starty);
        self.source.set_property("crop-width", endx - startx);
        self.source.set_property("crop-height", endy - starty);
    }

    pub fn capture_fullscreen(&self) {
        self.capture_resize(0, 0, 0, 0);
    }
}

impl Drop for StreamingServer {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
