use anyhow::Result;
use egui::{Key, KeyboardShortcut, Modifiers};
use indexmap::IndexMap;
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

fn empty_str() -> &'static str {
    ""
}

fn default_shortcut_placeholder() -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::NONE, Key::Questionmark)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Keybinding {
    #[serde(skip_deserializing, default = "empty_str")]
    pub name: &'static str,
    pub shortcut: KeyboardShortcut,
    #[serde(skip_deserializing, default = "default_shortcut_placeholder")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    pub in_game: IndexMap<Action, Keybinding>,
    pub application: IndexMap<Action, Keybinding>,
}

impl Default for Keybindings {
    fn default() -> Self {
        let mut in_game = IndexMap::new();
        in_game.insert(
            Action::A,
            Keybinding::new("A", KeyboardShortcut::new(Modifiers::NONE, Key::Z)),
        );
        in_game.insert(
            Action::B,
            Keybinding::new("B", KeyboardShortcut::new(Modifiers::NONE, Key::X)),
        );
        in_game.insert(
            Action::Start,
            Keybinding::new("Start", KeyboardShortcut::new(Modifiers::NONE, Key::Enter)),
        );
        in_game.insert(
            Action::Select,
            Keybinding::new(
                "Select",
                KeyboardShortcut::new(Modifiers::NONE, Key::Escape),
            ),
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

        let mut application = IndexMap::new();
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

    pub fn apply_defaults(&mut self) {
        let defaults = Keybindings::default();

        self.in_game = defaults
            .in_game
            .iter()
            .map(|(action, default)| {
                let shortcut = self
                    .in_game
                    .get(action)
                    .map(|loaded| loaded.shortcut)
                    .unwrap_or(default.shortcut);
                (
                    *action,
                    Keybinding {
                        shortcut,
                        ..*default
                    },
                )
            })
            .collect();

        self.application = defaults
            .application
            .iter()
            .map(|(action, default)| {
                let shortcut = self
                    .application
                    .get(action)
                    .map(|loaded| loaded.shortcut)
                    .unwrap_or(default.shortcut);
                (
                    *action,
                    Keybinding {
                        shortcut,
                        ..*default
                    },
                )
            })
            .collect();
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub keybindings: Keybindings,
}

fn load_from_file() -> Result<Option<Settings>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::fs;

        use crate::platform::native::{ProjDirKind, get_project_dir};

        let mut path = get_project_dir(ProjDirKind::Config)?;
        path.push("config.toml");
        if !fs::exists(&path)? {
            return Ok(None);
        }
        let contents = fs::read_to_string(path)?;
        let mut settings: Settings = toml::from_str(&contents)?;
        settings.keybindings.apply_defaults();

        Ok(Some(settings))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(None)
    }
}

impl Settings {
    pub fn new() -> Settings {
        if let Ok(Some(settings)) = load_from_file() {
            settings
        } else {
            let settings: Settings = Default::default();
            if let Err(e) = settings.save_to_file() {
                error!("{}", e);
            }
            settings
        }
    }

    pub fn save_to_file(&self) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::fs;

            use crate::platform::native::{ProjDirKind, get_project_dir};

            let mut path = get_project_dir(ProjDirKind::Config)?;
            fs::create_dir_all(&path)?;
            path.push("config.toml");

            let contents = toml::to_string_pretty(self)?;
            fs::write(path, contents)?;
        }
        Ok(())
    }
}
