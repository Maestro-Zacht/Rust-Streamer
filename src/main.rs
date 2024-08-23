use std::sync::{Arc, Mutex};

use image::ImageFormat;
use rust_streamer::streaming::Streaming;

use clap::{Args, Parser, Subcommand};
use eframe::egui::{self, Color32, Key};

use std::net::Ipv4Addr;

fn is_valid_ipv4(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>().is_ok()
}

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

#[derive(Clone, Copy, PartialEq)]  // Aggiunto PartialEq per l'enum Mode
enum Mode {
    Caster,
    Receiver,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Caster
    }
}

#[derive(PartialEq)]
enum TransmissionStatus {
    Idle,
    Casting,
    Receiving,
}

impl Default for TransmissionStatus {
    fn default() -> Self {
        TransmissionStatus::Idle
    }
}

#[derive(PartialEq, Clone)]
struct ScreenArea {
    startx: u32,
    starty: u32,
    endx: u32,
    endy: u32,
}

struct MyApp {
    _streaming: Option<Streaming>,
    current_image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<egui::TextureHandle>,
    mode: Mode,
    caster_address: String,
    selected_screen_area: Option<ScreenArea>,
    transmission_status: TransmissionStatus,
    pause: bool,
    wrong_ip: bool,
    blanking_screen: bool
}

impl MyApp {
    fn new() -> Self {
        // TODO not use a fake streaming
        let current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
            [200, 200],
            Color32::BLACK,
        ))));
        
        

        Self {
            _streaming: None,
            current_image,
            texture: None,
            mode: Mode::default(),
            caster_address: String::default(),
            selected_screen_area: None,
            transmission_status: TransmissionStatus::default(),
            pause: false,
            wrong_ip: false,
            blanking_screen: false
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Screen-Caster");

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.add_enabled_ui(self.transmission_status == TransmissionStatus::Idle, |ui| {
                    if ui.radio(self.mode == Mode::Caster, "Caster").clicked() {
                        self.mode = Mode::Caster;
                    }
                });
                ui.add_enabled_ui(self.transmission_status == TransmissionStatus::Idle, |ui| {
                    if ui.radio(self.mode == Mode::Receiver, "Receiver").clicked() {
                        self.mode = Mode::Receiver;
                    }
                });
            });

            ui.separator();

            match self.mode {
                Mode::Caster => {
                    ui.label("Select screen area:");
                    ui.horizontal(|ui| {
                        if ui.selectable_value(&mut None, self.selected_screen_area.clone(), "Total screen").clicked(){
                            self.selected_screen_area = None;
                            if let Some(s) = &self._streaming{
                                if let Streaming::Server(ss) = &s{
                                    ss.capture_fullscreen();
                                }
                            }
                        }
                        if ui.selectable_value(&mut true, self.selected_screen_area.is_some(), "Personalized area").clicked(){
                            self.selected_screen_area = todo!();
                            if let Some(s) = &self._streaming{
                                if let Streaming::Server(ss) = &s{
                                    ss.capture_resize(self.selected_screen_area.clone().unwrap().startx, self.selected_screen_area.clone().unwrap().starty, self.selected_screen_area.clone().unwrap().endx, self.selected_screen_area.clone().unwrap().endy)
                                }
                            }
                        }
                    });

                    //ui.separator();

                    // Mock screen area selection (replace with actual logic)
                    //ui.add_space(8.0);
                }
                Mode::Receiver => {
                    if self.wrong_ip{
                        ui.colored_label(egui::Color32::RED, "Please insert a valid IP address!");
                    }
                    ui.label("Enter caster's address:");

                    ui.add_enabled(self.transmission_status == TransmissionStatus::Idle, |ui: &mut egui::Ui|{
                        ui.text_edit_singleline(&mut self.caster_address)
                    });
                }
            }

            ui.separator();

            match &self.transmission_status {
                TransmissionStatus::Idle => {
                    match self.mode {
                        Mode::Caster => {
                            if ui.button("Start trasmission").clicked() {
                                if let Some(s) = &self._streaming{
                                    match s {
                                        Streaming::Client(_) => {
                                            let image_clone = self.current_image.clone();
                                            let streaming = Streaming::new_server(move |bytes| {
                                                let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                    .unwrap()
                                                    .to_rgba8();
                        
                                                let size = [image.width() as usize, image.height() as usize];
                                                let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                        
                                                // println!("Received image with size {:?}", size);
                        
                                                *image_clone.lock().unwrap() = Some(image);
                                            })
                                            .unwrap();
                                            self._streaming = Some(streaming);
                                        }
                                        Streaming::Server(_) => { /* Nothing to do because it is already a streaming server */ }
                                    }
            
                                }
                                else{
                                    let image_clone = self.current_image.clone();
                                    let streaming = Streaming::new_server(move |bytes| {
                                        let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                            .unwrap()
                                            .to_rgba8();
            
                                        let size = [image.width() as usize, image.height() as usize];
                                        let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
            
                                        // println!("Received image with size {:?}", size);
            
                                        *image_clone.lock().unwrap() = Some(image);
                                    })
                                    .unwrap();
                                    self._streaming = Some(streaming);
                                }
                                if let Some(s) = &self._streaming{
                                    self.pause = false;
                                    self.blanking_screen = false;
                                    s.start().unwrap();
                                }
                                self.transmission_status = TransmissionStatus::Casting;
                            }
                        }
                        Mode::Receiver => {
                            if ui.button("Start reception").clicked() {
                                if is_valid_ipv4(&self.caster_address){
                                    self.wrong_ip = false;
                                    self.transmission_status = TransmissionStatus::Receiving;
                                    if let Some(s) = &self._streaming{
                                        match s {
                                            Streaming::Client(_) => {}
                                            Streaming::Server(_) => {
                                                let image_clone = self.current_image.clone();
                                                let streaming = Streaming::new_client(self.caster_address.clone(), move |bytes| {
                                                    let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                        .unwrap()
                                                        .to_rgba8();
                            
                                                    let size = [image.width() as usize, image.height() as usize];
                                                    let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                            
                                                    // println!("Received image with size {:?}", size);
                            
                                                    *image_clone.lock().unwrap() = Some(image);
                                                })
                                                .unwrap();
                                                self._streaming = Some(streaming);
                                            }
                                        }
                
                                    }
                                    else{
                                        let image_clone = self.current_image.clone();
                                        let streaming = Streaming::new_client(self.caster_address.clone(), move |bytes| {
                                            let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                .unwrap()
                                                .to_rgba8();
                    
                                            let size = [image.width() as usize, image.height() as usize];
                                            let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                    
                                            // println!("Received image with size {:?}", size);
                    
                                            *image_clone.lock().unwrap() = Some(image);
                                        })
                                        .unwrap();
                                        self._streaming = Some(streaming);
                                    }
                                    if let Some(s) = &self._streaming{
                                        self.caster_address = String::default();
                                        self.wrong_ip = false;
                                        s.start().unwrap();
                                    }
                                }
                                else{
                                    self.wrong_ip = true;
                                }
                            }
                        }
                    }
                }
                TransmissionStatus::Casting => {
                    let input = ctx.input(|i| i.clone());
                    if !self.pause{
                        ui.label("Casting...");
                    }
                    else{
                        ui.colored_label(egui::Color32::LIGHT_RED, "Pause...");
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Stop transmission").clicked() || input.key_pressed(Key::T) && input.modifiers.ctrl{
                            self._streaming.take();
                            self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                                [200, 200],
                                Color32::BLACK))));
                            self.transmission_status = TransmissionStatus::Idle;
                        }

                        if ui.add_enabled(!self.pause, egui::Button::new("Pause")).clicked() || input.key_pressed(Key::P) && input.modifiers.ctrl{
                            self.pause = true;
                            //TODO: aggiornare stato pipeline server
                        }
                        if ui.add_enabled(self.pause, egui::Button::new("Resume")).clicked() || input.key_pressed(Key::R) && input.modifiers.ctrl{
                            self.pause = false;
                            //TODO: aggiornare stato pipeline server
                        }
                        if ui.selectable_value(&mut self.blanking_screen.clone(), true, "Blanking screen").clicked(){
                            self.blanking_screen = !self.blanking_screen;
                        }
                    });

                }
                TransmissionStatus::Receiving => {
                    ui.label("Receiving...");
                    if ui.button("Stop reception").clicked() {
                        self._streaming.take();
                        self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                            [200, 200],
                            Color32::BLACK))));
                        self.transmission_status = TransmissionStatus::Idle;
                    }
                }
            }

            let mut data = self.current_image.lock().unwrap();
            if let Some(image) = data.take() {
                self.texture = Some(ui.ctx().load_texture("image", image, Default::default()));
            }
            drop(data);

            if let Some(texture) = &self.texture {
                ui.add(egui::Image::from_texture(texture).shrink_to_fit());
            }
        });
    }

    /*fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            ui.label("This is an example app");

            let mut data = self.current_image.lock().unwrap();
            if let Some(image) = data.take() {
                self.texture = Some(ui.ctx().load_texture("image", image, Default::default()));
            }
            drop(data);

            if let Some(texture) = &self.texture {
                ui.add(egui::Image::from_texture(texture).shrink_to_fit());
            }
        });
    }*/
}

fn main() {
    // let cli = Cli::parse();

    // let streaming = match cli.command {
    //     Commands::Send => Streaming::new_server(|_| {}).unwrap(),
    //     Commands::Recv(ServerArgs { ip }) => Streaming::new_client(ip, |_| {}).unwrap(),
    // };

    let options = Default::default();
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
