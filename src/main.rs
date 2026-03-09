use core::convert::Infallible;

use eframe::egui;
use egui::ahash::HashMap;
use serde::Deserialize;

use crate::greetd::Greetd;

use self::gamepad::Gamepad;

mod gamepad;
mod greetd;

const SELECTED_USER_KEY: &str = "selected_user";

#[derive(Deserialize)]
struct Config {
    users: Vec<String>,
    sessions: Vec<ConfigSession>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConfigSession {
    name: String,
    exec: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

fn main() -> eframe::Result<Infallible> {
    std::panic::set_hook(Box::new(|panic_info| {
        std::process::Command::new("hermes")
            .arg("send")
            .arg(format!("tv-greet panic: {}", panic_info))
            .spawn()
            .ok();

        std::process::exit(1);
    }));

    tracing_subscriber::fmt::init();

    tracing::info!("Starting tv-greet");

    let config = config::Config::builder()
        .add_source(config::File::with_name("/etc/tv-greet/config.toml"))
        .build()
        .unwrap()
        .try_deserialize::<Config>()
        .unwrap();

    tracing::info!("Config users: {:?}", config.users);
    for session in &config.sessions {
        tracing::info!("Config session: {session:?}");
    }

    // get a connection to greetd before we start eframe
    tracing::info!("Connecting to greetd");
    let greetd = Greetd::new();

    let app = GreeterApp {
        config,
        greetd,
        selected_user: None,
        // session_starting: false,
    };

    tracing::info!("Starting eframe");

    let native_options = eframe::NativeOptions {
        window_builder: Some(Box::new(|vb| vb.with_fullscreen(true))),
        ..Default::default()
    };

    eframe::run_native(
        "tv-greet",
        native_options,
        Box::new(|cc| Ok(Box::new(app.setup(cc)))),
    )?;

    unreachable!();
}

struct GreeterApp {
    config: Config,
    greetd: Greetd,
    selected_user: Option<String>,
    // session_starting: bool,
}

impl GreeterApp {
    fn setup(mut self, cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(2.5);

        cc.egui_ctx.style_mut(|s| {
            s.visuals.panel_fill = egui::Color32::from_rgb(12, 18, 24);
        });

        cc.egui_ctx.add_plugin(Gamepad::new());

        if cc.storage.is_some() {
            tracing::info!("Persistence available, loading selected user");
        }

        self.selected_user = cc
            .storage
            .and_then(|s| eframe::get_value(s, SELECTED_USER_KEY))
            .unwrap_or(self.config.users.first().cloned());

        self
    }
}

impl eframe::App for GreeterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // if self.session_starting {
            //     egui::Spinner::new().paint_at(
            //         ui,
            //         Rect::from_center_size(ui.clip_rect().center(), Vec2::splat(128.)),
            //     );

            //     return;
            // }

            ui.group(|ui| {
                for user in &self.config.users {
                    ui.selectable_value(&mut self.selected_user, Some(user.clone()), user);
                }
            });

            ui.group(|ui| {
                if self.selected_user.is_none() {
                    ui.disable();
                }

                for session in &self.config.sessions {
                    if ui.button(&session.name).clicked() {
                        tracing::info!("Starting session: {}", session.name);
                        tracing::info!("Session exec: {:?}", session.exec);
                        tracing::info!("Session env: {:?}", session.env);

                        self.greetd
                            .create_session(self.selected_user.as_ref().unwrap());

                        self.greetd.start_session(&session.exec, &session.env);

                        std::thread::sleep(std::time::Duration::from_secs(1));

                        std::process::exit(0);
                    }
                }
            })
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SELECTED_USER_KEY, &self.selected_user);
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}
