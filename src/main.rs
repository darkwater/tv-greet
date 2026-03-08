use eframe::egui;

use crate::greetd::Greetd;

use self::gamepad::Gamepad;

mod gamepad;
mod greetd;

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|panic_info| {
        std::process::Command::new("hermes")
            .arg("send")
            .arg(format!("tv-greet panic: {}", panic_info))
            .spawn()
            .ok();

        std::process::exit(1);
    }));

    // get a connection to greetd before we start eframe
    let greetd = Greetd::new();

    let native_options = eframe::NativeOptions {
        window_builder: Some(Box::new(|vb| vb.with_fullscreen(true))),
        ..Default::default()
    };

    eframe::run_native(
        "tv-greet",
        native_options,
        Box::new(|cc| Ok(Box::new(GreeterApp::new(cc, greetd)))),
    )
}

struct GreeterApp {
    greetd: Greetd,
    log: Vec<String>,
    // session_starting: bool,
}

impl GreeterApp {
    fn new(cc: &eframe::CreationContext<'_>, greetd: Greetd) -> Self {
        cc.egui_ctx.set_pixels_per_point(2.5);

        cc.egui_ctx.style_mut(|s| {
            s.visuals.panel_fill = egui::Color32::from_rgb(12, 18, 24);
        });

        cc.egui_ctx.add_plugin(Gamepad::new());

        Self {
            greetd,
            log: Vec::new(),
            // session_starting: false,
        }
    }

    fn start_session(&mut self, username: &str, session: &[&str]) {
        // self.session_starting = true;
        self.greetd.create_session(username);
        self.greetd.start_session(session);
        std::process::exit(0);
    }
}

impl eframe::App for GreeterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(res) = self.greetd.recv() {
            self.log.push(format!("Received response: {res:?}"));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // if self.session_starting {
            //     egui::Spinner::new().paint_at(
            //         ui,
            //         Rect::from_center_size(ui.clip_rect().center(), Vec2::splat(128.)),
            //     );

            //     return;
            // }

            if ui.button("Hyprland").clicked() {
                self.start_session("dark", &["hyprland"]);
            }

            if ui.button("Steam").clicked() {
                self.start_session("dark", &["gamescope-steam"]);
            }
        });
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}
