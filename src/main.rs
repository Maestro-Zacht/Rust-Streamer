use std::sync::{Arc, Mutex};

use image::ImageFormat;
use rust_streamer::streaming::Streaming;

use clap::{Args, Parser, Subcommand};
use eframe::egui::{self, Color32};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Send,
    Recv(ServerArgs),
}

#[derive(Args, Debug)]
struct ServerArgs {
    ip: String,
}

struct MyApp {
    _streaming: Streaming,
    current_image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<egui::TextureHandle>,
}

impl MyApp {
    fn new() -> Self {
        // TODO not use a fake streaming
        let current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
            [200, 200],
            Color32::BLACK,
        ))));
        let image_clone = current_image.clone();
        let streaming = Streaming::new_server(move |bytes| {
            let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                .unwrap()
                .to_rgba8();

            let image = image::RgbaImage::from_raw(image.width(), image.height(), image.into_raw())
                .unwrap();

            let size = [image.width() as usize, image.height() as usize];
            let mut pixels = Vec::with_capacity(size[0] * size[1]);
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let pixel = image.get_pixel(x as u32, y as u32);
                    pixels.push(egui::Color32::from_rgba_premultiplied(
                        pixel[0], pixel[1], pixel[2], pixel[3],
                    ));
                }
            }

            println!("Received image with size {:?}", size);

            *image_clone.lock().unwrap() = Some(egui::ColorImage { pixels, size });
        })
        .unwrap();
        streaming.start().unwrap();
        Self {
            _streaming: streaming,
            current_image,
            texture: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            ui.label("This is an example app");

            let mut data = self.current_image.lock().unwrap();
            if let Some(image) = data.take() {
                self.texture = Some(ui.ctx().load_texture("image", image, Default::default()));
            }
            drop(data);

            if let Some(texture) = &self.texture {
                ui.image(texture);
            }
        });
    }
}

fn main() {
    // let cli = Cli::parse();

    // let streaming = match cli.command {
    //     Commands::Send => Streaming::new_server(|_| {}).unwrap(),
    //     Commands::Recv(ServerArgs { ip }) => Streaming::new_client(ip, |_| {}).unwrap(),
    // };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new()))
        }),
    )
    .unwrap();
    println!("Finished");
}
