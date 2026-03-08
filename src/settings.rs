use egui::Key;
use linked_hash_map::LinkedHashMap;

#[derive(Clone)]
pub struct Keybindings {
    pub in_game: LinkedHashMap<&'static str, Key>,
    pub application: LinkedHashMap<&'static str, Key>,
}

impl Default for Keybindings {
    fn default() -> Self {
        let mut in_game = LinkedHashMap::new();
        in_game.insert("A", Key::A);
        in_game.insert("B", Key::B);
        in_game.insert("Start", Key::Z);
        in_game.insert("Select", Key::N);
        in_game.insert("Up", Key::ArrowUp);
        in_game.insert("Down", Key::ArrowDown);
        in_game.insert("Left", Key::ArrowLeft);
        in_game.insert("Right", Key::ArrowRight);

        let mut application = LinkedHashMap::new();
        application.insert("Save State", Key::F5);
        application.insert("Load State", Key::F6);

        Self {
            in_game,
            application,
        }
    }
}

#[derive(Default)]
pub struct Settings {
    pub keybindings: Keybindings,
}
