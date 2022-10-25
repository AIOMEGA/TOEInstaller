use std::{fs, io, path, sync::mpsc, thread};

use anyhow::{Context, Error, Result};
use eframe::egui;
use ureq::serde_json;

#[derive(Default)]
enum State {
    #[default]
    Start,
    Found,
    Downloading(mpsc::Receiver<Result<()>>),
    Installing(mpsc::Receiver<Result<()>>),
    Done,
    Error(Error),
}

#[derive(Default)]
struct App {
    state: State,
    path: path::PathBuf,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.);
                ui.heading("TOE Installer");
                ui.add_space(10.);

                match &self.state {
                    State::Start => {
                        if ui.button("Find AmongUs").clicked() {
                            match find_among_us() {
                                Ok(path) => {
                                    self.path = path;

                                    self.state = State::Found
                                }
                                Err(e) => self.state = State::Error(e),
                            }
                        }
                    }
                    State::Found => {
                        ui.label(format!("Found: {:?}", self.path));
                        ui.add_space(10.);
                        if ui.button("Install").clicked() {
                            let (s, r) = mpsc::channel();

                            thread::spawn(move || s.send(download_mod()));

                            self.state = State::Downloading(r)
                        }
                    }
                    State::Downloading(receiver) => match receiver.try_recv() {
                        Ok(Ok(())) => {
                            let (s, r) = mpsc::channel();

                            let path = self.path.clone();
                            thread::spawn(move || s.send(install(&path)));

                            self.state = State::Installing(r);
                        }
                        Ok(Err(e)) => {
                            let _ = fs::remove_file("mod.zip");

                            self.state = State::Error(e)
                        }
                        Err(_) => {
                            ui.label("Downloading");
                            ui.add_space(10.);
                            ui.spinner();
                        }
                    },
                    State::Installing(receiver) => match receiver.try_recv() {
                        Ok(Ok(())) => self.state = State::Done,
                        Ok(Err(e)) => {
                            let _ = fs::remove_file("mod.zip");

                            self.state = State::Error(e)
                        }
                        Err(_) => {
                            ui.label("Installing");
                            ui.add_space(10.);
                            ui.spinner();
                        }
                    },
                    State::Done => {
                        ui.label("Finished");
                        ui.add_space(10.);
                        if ui.button("Exit").clicked() {
                            frame.close();
                        }
                    }
                    State::Error(e) => {
                        ui.label(format!("Error: {}", e));
                    }
                }

                ctx.request_repaint();
            });
        });
    }
}

fn find_among_us() -> Result<path::PathBuf> {
    let mut steam = steamlocate::SteamDir::locate().context("Couldn't find steam")?;

    let among_us = steam.app(&945360).context("Couldn't find AmongUs")?;

    Ok(among_us.path.clone())
}

fn download_mod() -> Result<()> {
    let download_url =
        ureq::get("https://api.github.com/repos/AIOMEGA/TownOfEmpath/releases/latest")
            .call()?
            .into_json::<serde_json::Value>()?["assets"][0]["browser_download_url"]
            .as_str()
            .context("?")?
            .to_string();

    let res = ureq::get(&download_url).call()?;
    let mut file = fs::File::create("mod.zip")?;
    io::copy(&mut res.into_reader(), &mut file)?;

    Ok(())
}

fn install(path: &path::PathBuf) -> Result<()> {
    let _ = fs::remove_dir_all(path.join("BepInEx"));
    let _ = fs::remove_dir_all(path.join("mono"));

    let _ = fs::remove_file(path.join("changelog.txt"));
    let _ = fs::remove_file(path.join("doorstop_config.ini"));
    let _ = fs::remove_file(path.join("winhttp.dll"));

    let mut archive = zip::ZipArchive::new(fs::File::open("mod.zip")?)?;
    archive.extract(path)?;

    let _ = fs::remove_file("mod.zip");

    Ok(())
}

fn main() -> Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "TOE Installer",
        native_options,
        Box::new(|_| Box::<App>::default()),
    );

    Ok(())
}
