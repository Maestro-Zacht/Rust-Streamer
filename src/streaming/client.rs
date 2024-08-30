use byte_slice_cast::*;
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::connection::client::ConnectionClient;
use gstreamer::{self as gst, element_error, glib, prelude::*};
use gstreamer_app as gst_app;
use thiserror::Error;

use chrono::prelude::*;

#[derive(Error, Debug)]
pub enum StreamingClientError {
    #[error("GStreamer init error: {0}")]
    GStreamerInitError(#[from] glib::Error),

    #[error("GStreamer element error: {0}")]
    GStreamerElementCreationError(#[from] glib::BoolError),

    #[error("GStreamer state change error: {0}")]
    GStreamerStateChangeError(#[from] gst::StateChangeError),

    #[error("Websocket error: {0}")]
    WebsocketError(#[from] io::Error),
}

pub struct StreamingClient {
    pipeline: Arc<gst::Pipeline>,
    _connection_client: ConnectionClient,
    connected: Arc<AtomicBool>,
}

impl StreamingClient {
    pub fn new<T: AsRef<str>>(
        ip: T,
        mut image_parser: impl FnMut(&[u8]) + Send + 'static,
        save_stream: bool,
    ) -> Result<Self, StreamingClientError> {
        gst::init()?;

        let mut pipeline_string = "udpsrc port=9001 !
        application/x-rtp, media=video, clock-rate=90000, encoding-name=H264, payload=96 ! rtph264depay ! tee name=t ! queue ! decodebin !
        videoconvert ! jpegenc ! appsink name=s max-buffers=1 caps=image/jpeg".to_string();

        if save_stream {
            pipeline_string.push_str(&format!(
                " t. ! queue ! h264parse ! mp4mux ! filesink location=./stream{}.mp4",
                Local::now().format("%Y%m%d_%H%M%S")
            ));
        }

        let pipeline = gst::parse::launch(&pipeline_string)?
            .dynamic_cast::<gst::Pipeline>()
            .unwrap();

        let sink: gst_app::AppSink = pipeline.by_name("s").unwrap().dynamic_cast().unwrap();

        let pipeline = Arc::new(pipeline);
        let connected = Arc::new(AtomicBool::new(true));

        let pipeline_clone = pipeline.clone();
        let connected_clone = connected.clone();
        let connection_client = ConnectionClient::new(ip, move || {
            pipeline_clone.send_event(gst::event::Eos::new());
            pipeline_clone
                .bus()
                .unwrap()
                .timed_pop_filtered(gst::ClockTime::NONE, &[gst::MessageType::Eos]);
            let _ = pipeline_clone.set_state(gst::State::Null);
            connected_clone.store(false, Ordering::Relaxed);
        })?;

        sink.set_callbacks(
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
            pipeline,
            _connection_client: connection_client,
            connected,
        })
    }

    pub fn start(&self) -> Result<(), StreamingClientError> {
        Ok(self.pipeline.set_state(gst::State::Playing).map(|_| ())?)
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}

impl Drop for StreamingClient {
    fn drop(&mut self) {
        if self.is_connected() {
            self.pipeline.send_event(gst::event::Eos::new());
            self.pipeline
                .bus()
                .unwrap()
                .timed_pop_filtered(gst::ClockTime::NONE, &[gst::MessageType::Eos]);
        }
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
