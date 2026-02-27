use egui::Key;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Keybindings {
    pub in_game: HashMap<&'static str, Key>,
    pub application: HashMap<&'static str, Key>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            in_game: HashMap::from([
                ("a", Key::A),
                ("b", Key::B),
                ("start", Key::Z),
                ("select", Key::N),
                ("up", Key::ArrowUp),
                ("down", Key::ArrowDown),
                ("left", Key::ArrowLeft),
                ("right", Key::ArrowRight),
            ]),
            application: HashMap::from([("save_state", Key::F5)]),
        }
    }
}

#[derive(Default)]
pub struct Settings {
    pub keybindings: Keybindings,
}
