use egui::{Key, KeyboardShortcut, Modifiers};
use linked_hash_map::LinkedHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // In-game
    A,
    B,
    Start,
    Select,
    Up,
    Down,
    Left,
    Right,

    // Application
    PauseResume,
    Step,
    SaveState,
    LoadState,
    #[cfg(not(target_arch = "wasm32"))]
    TakeScreenshot,
    OpenRom,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Keybinding {
    pub name: &'static str,
    pub shortcut: KeyboardShortcut,
    default_shortcut: KeyboardShortcut,
}

impl Keybinding {
    pub fn new(name: &'static str, default_key: KeyboardShortcut) -> Keybinding {
        Keybinding {
            name,
            shortcut: default_key,
            default_shortcut: default_key,
        }
    }

    pub fn default_key(&self) -> KeyboardShortcut {
        self.default_shortcut
    }

    pub fn reset(&mut self) {
        self.shortcut = self.default_shortcut;
    }
}

#[derive(Clone)]
pub struct Keybindings {
    pub in_game: LinkedHashMap<Action, Keybinding>,
    pub application: LinkedHashMap<Action, Keybinding>,
}

impl Default for Keybindings {
    fn default() -> Self {
        let mut in_game = LinkedHashMap::new();
        in_game.insert(
            Action::A,
            Keybinding::new("A", KeyboardShortcut::new(Modifiers::NONE, Key::A)),
        );
        in_game.insert(
            Action::B,
            Keybinding::new("B", KeyboardShortcut::new(Modifiers::NONE, Key::B)),
        );
        in_game.insert(
            Action::Start,
            Keybinding::new("Start", KeyboardShortcut::new(Modifiers::NONE, Key::Z)),
        );
        in_game.insert(
            Action::Select,
            Keybinding::new("Select", KeyboardShortcut::new(Modifiers::NONE, Key::N)),
        );
        in_game.insert(
            Action::Up,
            Keybinding::new("Up", KeyboardShortcut::new(Modifiers::NONE, Key::ArrowUp)),
        );
        in_game.insert(
            Action::Down,
            Keybinding::new(
                "Down",
                KeyboardShortcut::new(Modifiers::NONE, Key::ArrowDown),
            ),
        );
        in_game.insert(
            Action::Left,
            Keybinding::new(
                "Left",
                KeyboardShortcut::new(Modifiers::NONE, Key::ArrowLeft),
            ),
        );
        in_game.insert(
            Action::Right,
            Keybinding::new(
                "Right",
                KeyboardShortcut::new(Modifiers::NONE, Key::ArrowRight),
            ),
        );

        let mut application = LinkedHashMap::new();
        application.insert(
            Action::SaveState,
            Keybinding::new(
                "Save State",
                KeyboardShortcut::new(Modifiers::NONE, Key::F5),
            ),
        );
        application.insert(
            Action::LoadState,
            Keybinding::new(
                "Load State",
                KeyboardShortcut::new(Modifiers::NONE, Key::F6),
            ),
        );
        application.insert(
            Action::PauseResume,
            Keybinding::new(
                "Pause / Resume",
                KeyboardShortcut::new(Modifiers::NONE, Key::Space),
            ),
        );
        application.insert(
            Action::Step,
            Keybinding::new("Step", KeyboardShortcut::new(Modifiers::NONE, Key::Enter)),
        );
        #[cfg(not(target_arch = "wasm32"))]
        application.insert(
            Action::TakeScreenshot,
            Keybinding::new(
                "Screenshot",
                KeyboardShortcut::new(Modifiers::NONE, Key::F12),
            ),
        );
        application.insert(
            Action::OpenRom,
            Keybinding::new(
                "Open ROM",
                KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::CTRL, Key::O),
            ),
        );
        application.insert(
            Action::Quit,
            Keybinding::new(
                "Quit",
                KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::CTRL, Key::Q),
            ),
        );

        Self {
            in_game,
            application,
        }
    }
}

impl Keybindings {
    pub fn reset_all(&mut self) {
        *self = Self::default();
    }

    pub fn shortcut(&self, action: Action) -> KeyboardShortcut {
        self.in_game
            .get(&action)
            .or_else(|| self.application.get(&action))
            .map(|k| k.shortcut)
            .unwrap()
    }

    pub fn format_shortcut(&self, ctx: &egui::Context, action: Action) -> String {
        ctx.format_shortcut(&self.shortcut(action))
    }
}

#[derive(Default)]
pub struct Settings {
    pub keybindings: Keybindings,
}
