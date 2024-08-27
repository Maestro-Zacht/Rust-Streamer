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
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    source: gst::Element,

    pipeline: gst::Pipeline,

    #[cfg(target_os = "macos")]
    crop: gst::Element,

    selector: gst::Element,

    _connection_server: ConnectionServer,
}

impl StreamingServer {
    pub fn new(
        mut image_parser: impl FnMut(&[u8]) + Send + 'static,
    ) -> Result<Self, StreamingServerError> {
        gst::init()?;

        let pipeline_string = if cfg!(target_os = "windows") {
            "input-selector name=i ! tee name=t ! queue ! videoconvert ! x264enc tune=zerolatency ! rtph264pay ! multiudpsink name=s t. ! queue ! videoconvert ! jpegenc ! appsink max-buffers=1 caps=image/jpeg name=videosink d3d11screencapturesrc show-cursor=true name=src ! video/x-raw,framerate=30/1 ! i.sink_0 videotestsrc pattern=white ! video/x-raw,framerate=30/1 ! i.sink_1"
        } else if cfg!(target_os = "linux") {
            "input-selector name=i ! tee name=t ! queue ! videoconvert ! x264enc tune=zerolatency ! rtph264pay ! multiudpsink name=s t. ! queue ! videoconvert ! jpegenc ! appsink max-buffers=1 caps=image/jpeg name=videosink ximagesrc use-damage=false name=src ! video/x-raw,framerate=30/1 ! videoconvert ! i.sink_0 videotestsrc pattern=white ! video/x-raw,framerate=30/1 ! i.sink_1"
        } else {
            "input-selector name=i ! tee name=t ! queue ! videoconvert ! x264enc tune=zerolatency ! rtph264pay ! multiudpsink name=s t. ! queue ! videoconvert ! jpegenc ! appsink max-buffers=1 caps=image/jpeg name=videosink avfvideosrc capture-screen=1 capture-screen-cursor=1 name=src ! video/x-raw,framerate=30/1 ! videocrop name=crop ! videoconvert ! i.sink_0 videotestsrc pattern=white ! video/x-raw,framerate=30/1 ! videoconvert ! i.sink_1"
        };

        // can't panic after pipeline is created correctly
        let pipeline = gst::parse::launch(&pipeline_string)?
            .dynamic_cast::<gst::Pipeline>()
            .unwrap();
        let multiudpsink = pipeline.by_name("s").unwrap();
        let videosink = pipeline
            .by_name("videosink")
            .unwrap()
            .dynamic_cast::<gst_app::AppSink>()
            .unwrap();

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        let source = pipeline.by_name("src").unwrap();

        #[cfg(target_os = "macos")]
        let crop = pipeline.by_name("crop").unwrap();

        let selector = pipeline.by_name("i").unwrap();

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
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            source,

            pipeline,

            #[cfg(target_os = "macos")]
            crop,

            selector,

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
    /// startx, starty are the top left corner of the rectangle, endx, endy are the bottom right corner of the rectangle
    pub fn capture_resize(&self, startx: u32, starty: u32, endx: u32, endy: u32) {
        self.source.set_property("startx", startx);
        self.source.set_property("starty", starty);
        self.source.set_property("endx", endx);
        self.source.set_property("endy", endy);
    }

    #[cfg(target_os = "windows")]
    /// startx, starty are the top left corner of the rectangle, endx, endy are the bottom right corner of the rectangle
    pub fn capture_resize(&self, startx: u32, starty: u32, endx: u32, endy: u32) {
        self.source.set_property("crop-x", startx);
        self.source.set_property("crop-y", starty);
        self.source.set_property("crop-width", endx - startx);
        self.source.set_property("crop-height", endy - starty);
    }

    #[cfg(target_os = "macos")]
    /// the parameters are the number of pixels to remove from the left, top, right and bottom of the screen
    pub fn capture_resize(&self, left: u32, top: u32, right: u32, bottom: u32) {
        self.crop.set_property("left", left);
        self.crop.set_property("top", top);
        self.crop.set_property("right", right);
        self.crop.set_property("bottom", bottom);
    }

    pub fn capture_fullscreen(&self) {
        self.capture_resize(0, 0, 0, 0);
    }

    pub fn blank_screen(&self) {
        self.selector
            .set_property("active-pad", &self.selector.static_pad("sink_1").unwrap());
    }

    pub fn restore_screen(&self) {
        self.selector
            .set_property("active-pad", &self.selector.static_pad("sink_0").unwrap());
    }
}

impl Drop for StreamingServer {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
