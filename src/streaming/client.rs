use std::{io, sync::Arc};

use crate::connection::client::ConnectionClient;
use gstreamer::{self as gst, glib, prelude::*};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamingClientError {
    #[error("GStreamer init error: {0}")]
    GStreamerInitError(#[from] glib::Error),

    #[error("GStreamer element error: {0}")]
    GStreamerElementCreationError(#[from] glib::BoolError),

    #[error("Websocket error: {0}")]
    WebsocketError(#[from] io::Error),
}

pub struct StreamingClient {
    pipeline: Arc<gst::Pipeline>,
    connection_client: ConnectionClient,
}

impl StreamingClient {
    pub fn new<T: AsRef<str>>(ip: T) -> Result<Self, StreamingClientError> {
        gst::init()?;

        let source = gst::ElementFactory::make("udpsrc")
            .property("port", 9001)
            .build()?;

        let depay = gst::ElementFactory::make("rtph264depay").build()?;
        let decode = gst::ElementFactory::make("decodebin").build()?;
        let convert = gst::ElementFactory::make("videoconvert").build()?;
        let sink = gst::ElementFactory::make("autovideosink").build()?;

        let pipeline = gst::Pipeline::with_name("recv-pipeline");

        pipeline.add_many(&[&source, &depay, &decode, &convert, &sink])?;

        source.link_filtered(
            &depay,
            &gst::Caps::builder("application/x-rtp")
                .field("media", "video")
                .field("clock-rate", 90000)
                .field("encoding-name", "H264")
                .field("payload", 96)
                .build(),
        )?;
        depay.link(&decode)?;

        let convert_weak = convert.downgrade();
        decode.connect_pad_added(move |_, src_pad| {
            let sink_pad = match convert_weak.upgrade() {
                None => return,
                Some(s) => s.static_pad("sink").unwrap(),
            };
            src_pad.link(&sink_pad).unwrap();
        });

        convert.link(&sink)?;

        let pipeline = Arc::new(pipeline);

        let pipeline_clone = pipeline.clone();
        let connection_client = ConnectionClient::new(ip, move || {
            pipeline_clone.set_state(gst::State::Null).unwrap();
            // TODO better close connection
        })?;

        Ok(Self {
            pipeline,
            connection_client,
        })
    }

    pub fn start(&self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Playing).map(|_| ())
    }

    pub fn stop(self) -> Result<(), gst::StateChangeError> {
        self.connection_client.stop();
        self.pipeline.set_state(gst::State::Null)?;
        println!("Streaming client stopped");
        Ok(())
    }
}
