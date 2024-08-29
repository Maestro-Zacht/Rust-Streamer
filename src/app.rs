use std::sync::{Arc, Mutex};

use image::ImageFormat;

use eframe::egui::{self, Color32, Key};

use std::net::Ipv4Addr;

use crate::streaming::Streaming;

fn is_valid_ipv4(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>().is_ok()
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

pub struct MyApp {
    _streaming: Option<Streaming>,
    current_image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<egui::TextureHandle>,
    mode: Mode,
    caster_address: String,
    selected_screen_area: Option<ScreenArea>,
    transmission_status: TransmissionStatus,
    pause: bool,
    error_msg: Option<String>,
    blanking_screen: bool
}

impl MyApp {
    pub fn new() -> Self {
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
            error_msg: None,
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
                        self.error_msg.take();
                        self.mode = Mode::Caster;
                    }
                });
                ui.add_enabled_ui(self.transmission_status == TransmissionStatus::Idle, |ui| {
                    if ui.radio(self.mode == Mode::Receiver, "Receiver").clicked() {
                        self.error_msg.take();
                        self.mode = Mode::Receiver;
                    }
                });
            });

            match self.error_msg.clone() {
                Some(msg) => {
                    ui.colored_label(egui::Color32::RED, msg);
                }
                _ => {}
            }

            ui.separator();

            match self.mode {
                Mode::Caster => {
                    ui.label("Select screen area:");
                    ui.horizontal(|ui| {
                        if ui.selectable_value(&mut None, self.selected_screen_area.clone(), "Total screen").clicked(){
                            self.selected_screen_area = None;
                            if let Some(s) = &self._streaming {
                                if let Streaming::Server(ss) = &s{
                                    ss.capture_fullscreen();
                                }
                            }
                        }
                        if ui.selectable_value(&mut true, self.selected_screen_area.is_some(), "Personalized area").clicked(){
                            self.selected_screen_area = todo!();
                            if let Some(Streaming::Server(ss)) = &self._streaming {
                                ss.capture_resize((&self.selected_screen_area).unwrap().startx, (&self.selected_screen_area).unwrap().starty, (&self.selected_screen_area).unwrap().endx, (&self.selected_screen_area).unwrap().endy)
                            }
                        }
                    });
                }
                Mode::Receiver => {
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
                                            match Streaming::new_server(move |bytes| {
                                                let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                    .unwrap()
                                                    .to_rgba8();
                        
                                                let size = [image.width() as usize, image.height() as usize];
                                                let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                        
                                                *image_clone.lock().unwrap() = Some(image);
                                            }) {
                                                Ok(s) => {
                                                    self._streaming = Some(s);
                                                }
                                                Err(e) => {
                                                    self.error_msg = Some(e.to_string());
                                                }
                                            }
                                        }
                                        Streaming::Server(_) => { /* Nothing to do because it is already a streaming server */ }
                                    }
            
                                }
                                else{
                                    let image_clone = self.current_image.clone();
                                    match Streaming::new_server(move |bytes| {
                                        let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                            .unwrap()
                                            .to_rgba8();
                
                                        let size = [image.width() as usize, image.height() as usize];
                                        let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                
                                        *image_clone.lock().unwrap() = Some(image);
                                    }) {
                                        Ok(s) => {
                                            self._streaming = Some(s);
                                        }
                                        Err(e) => {
                                            self.error_msg = Some(e.to_string());
                                        }
                                    }
                                }
                                if let Some(s) = &self._streaming{
                                    self.pause = false;
                                    self.blanking_screen = false;
                                    self.error_msg.take();
                                    match s.start(){
                                        Ok(_) => {
                                            self.transmission_status = TransmissionStatus::Casting;
                                        }
                                        Err(e) => {
                                            self.error_msg = Some(e.to_string());
                                        }
                                    }
                                    
                                }
                            }
                        }
                        Mode::Receiver => {
                            if ui.button("Start reception").clicked() {
                                if is_valid_ipv4(&self.caster_address){
                                    if let Some(s) = &self._streaming{
                                        match s {
                                            Streaming::Client(_) => { /* Nothing to do because it is already a streaming client */ },
                                            Streaming::Server(_) => {
                                                let image_clone = self.current_image.clone();
                                                match Streaming::new_client(self.caster_address.clone(), move |bytes| {
                                                    let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                        .unwrap()
                                                        .to_rgba8();
                            
                                                    let size = [image.width() as usize, image.height() as usize];
                                                    let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                            
                                                    *image_clone.lock().unwrap() = Some(image);
                                                }) {
                                                    Ok(s) => {
                                                        self._streaming = Some(s);
                                                    }
                                                    Err(e) => {
                                                        self.error_msg = Some(e.to_string());
                                                    }
                                                }
                                            }
                                        }
                
                                    }
                                    else{
                                        let image_clone = self.current_image.clone();
                                        match Streaming::new_client(self.caster_address.clone(), move |bytes| {
                                            let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                .unwrap()
                                                .to_rgba8();
                    
                                            let size = [image.width() as usize, image.height() as usize];
                                            let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                    
                                            *image_clone.lock().unwrap() = Some(image);
                                        }) {
                                            Ok(s) => {
                                                self._streaming = Some(s);
                                            }
                                            Err(e) => {
                                                self.error_msg = Some(e.to_string());
                                            }
                                        }
                                    }
                                    if let Some(s) = &self._streaming{
                                        self.error_msg.take();
                                        match s.start(){
                                            Ok(_) => {
                                                self.transmission_status = TransmissionStatus::Receiving;
                                            }
                                            Err(e) => {
                                                self.error_msg = Some(e.to_string());
                                            }
                                        }
                                    }
                                }
                                else{
                                    self.error_msg = Some("Please insert a valid IP address!".to_string());
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
                        if ui.button("Stop transmission").on_hover_text("Ctrl + T").clicked() || input.key_pressed(Key::T) && input.modifiers.ctrl{
                            self._streaming.take();
                            self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                                [200, 200],
                                Color32::BLACK))));
                            self.transmission_status = TransmissionStatus::Idle;
                        }

                        if ui.add_enabled(!self.pause, egui::Button::new("Pause")).on_hover_text("Ctrl + P").clicked() || input.key_pressed(Key::P) && input.modifiers.ctrl{
                            self.pause = true;
                            if let Some(Streaming::Server(s)) = &self._streaming{
                                match s.pause(){
                                    Ok(_) => {}
                                    Err(e) => {
                                        self.error_msg = Some(e.to_string());
                                        self._streaming.take();
                                        self.transmission_status = TransmissionStatus::Idle;
                                    }
                                }
                            }
                        }
                        if ui.add_enabled(self.pause, egui::Button::new("Resume")).on_hover_text("Ctrl + R").clicked() || input.key_pressed(Key::R) && input.modifiers.ctrl{
                            self.pause = false;
                            if let Some(Streaming::Server(s)) = &self._streaming{
                                match s.start(){
                                    Ok(_) => {}
                                    Err(e) => {
                                        self.error_msg = Some(e.to_string());
                                        self._streaming.take();
                                        self.transmission_status = TransmissionStatus::Idle;
                                    }
                                }
                            }
                        }
                        if ui.selectable_value(&mut self.blanking_screen.clone(), true, "Blanking screen").on_hover_text("Ctrl + B").clicked() || input.key_pressed(Key::B) && input.modifiers.ctrl {
                            self.blanking_screen = !self.blanking_screen;
                            if let Some(Streaming::Server(s)) = &self._streaming {
                                if self.blanking_screen {
                                    s.blank_screen();
                                } else {
                                    s.restore_screen();
                                }
                            }
                        }
                    });

                }
                TransmissionStatus::Receiving => {
                    ui.label(format!("Receiving..."));
                    if ui.button("Stop reception").clicked() {
                        self._streaming.take();
                        self.caster_address = String::default();
                        self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                            [200, 200],
                            Color32::BLACK))));
                        self.transmission_status = TransmissionStatus::Idle;
                    }
                    if let Some(Streaming::Client(s)) = &self._streaming {
                        if !s.is_connected() {
                            self._streaming.take();
                            self.caster_address = String::default();
                            self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                                [200, 200],
                                Color32::BLACK))));
                            self.transmission_status = TransmissionStatus::Idle;
                        }
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
}