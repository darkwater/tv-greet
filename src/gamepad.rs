use egui::{FocusDirection, Modifiers};
use gilrs::{Button, EventType, Filter, Gilrs, ev::filter::axis_dpad_to_button};
use tokio::sync::{mpsc, watch};

#[derive(Debug)]
pub enum GamepadEvent {
    Move(FocusDirection),
    Select,
}

pub struct Gamepad {
    rx: mpsc::UnboundedReceiver<GamepadEvent>,
    ctx_tx: watch::Sender<Option<egui::Context>>,
}

impl Gamepad {
    pub fn new() -> Self {
        let mut gilrs = Gilrs::new().unwrap();

        for (id, gamepad) in gilrs.gamepads() {
            println!("Connected gamepad {}: {}", id, gamepad.name());
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let (ctx_tx, ctx_rx) = watch::channel::<Option<egui::Context>>(None);

        std::thread::spawn(move || {
            loop {
                let Some(event) = gilrs
                    .next_event_blocking(None)
                    .filter_ev(&axis_dpad_to_button, &mut gilrs)
                else {
                    continue;
                };

                gilrs.update(&event);

                let EventType::ButtonPressed(button, _) = event.event else {
                    continue;
                };

                let ev = match button {
                    Button::DPadUp => GamepadEvent::Move(FocusDirection::Up),
                    Button::DPadDown => GamepadEvent::Move(FocusDirection::Down),
                    Button::DPadRight => GamepadEvent::Move(FocusDirection::Right),
                    Button::DPadLeft => GamepadEvent::Move(FocusDirection::Left),

                    Button::North | Button::Select => GamepadEvent::Move(FocusDirection::Next),
                    Button::West => GamepadEvent::Move(FocusDirection::Previous),

                    // there's nothing to go "back" to atm so this is how we support both western and
                    // eastern button layouts :thumbsup:
                    Button::East | Button::South | Button::Start => GamepadEvent::Select,

                    _ => continue,
                };

                match tx.send(ev) {
                    Ok(()) => {
                        if let Some(ctx) = ctx_rx.borrow().as_ref() {
                            ctx.request_repaint()
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self { rx, ctx_tx }
    }
}

impl egui::Plugin for Gamepad {
    fn debug_name(&self) -> &'static str {
        "Gamepad"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        self.ctx_tx.send(Some(ctx.clone())).unwrap();
    }

    fn on_begin_pass(&mut self, ctx: &egui::Context) {
        let mut events = Vec::new();

        while let Ok(ev) = self.rx.try_recv() {
            events.push(format!("{ev:?}"));

            match ev {
                GamepadEvent::Move(dir) => ctx.memory_mut(|m| m.move_focus(dir)),
                GamepadEvent::Select => ctx.input_mut(|i| {
                    i.events.push(egui::Event::Key {
                        key: egui::Key::Enter,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::default(),
                    })
                }),
            }
        }
    }
}
