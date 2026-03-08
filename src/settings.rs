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
                ("A", Key::A),
                ("B", Key::B),
                ("Start", Key::Z),
                ("Select", Key::N),
                ("Up", Key::ArrowUp),
                ("Down", Key::ArrowDown),
                ("Left", Key::ArrowLeft),
                ("Right", Key::ArrowRight),
            ]),
            application: HashMap::from([("Save State", Key::F5), ("Load State", Key::F6)]),
        }
    }
}

#[derive(Default)]
pub struct Settings {
    pub keybindings: Keybindings,
}
