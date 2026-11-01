//! Engine-independent action bindings; physical input cannot write simulation state.
#![forbid(unsafe_code)]
pub mod calibration;
pub mod devices;
mod storage;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Key {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Space,
    Escape,
    Enter,
    Tab,
    Backspace,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Punch,
    Kick,
    Backhand,
    Accelerate,
    Brake,
    SteerLeft,
    SteerRight,
    Pause,
    Resume,
    Confirm,
    Back,
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Context {
    Race,
    Menu,
    Paused,
}
impl Context {
    pub const ALL: [Self; 3] = [Self::Race, Self::Menu, Self::Paused];
    fn required(self) -> &'static [Action] {
        use Action::*;
        match self {
            Self::Race => &[Accelerate, Brake, SteerLeft, SteerRight, Pause],
            Self::Menu => &[
                Confirm,
                Back,
                NavigateUp,
                NavigateDown,
                NavigateLeft,
                NavigateRight,
            ],
            Self::Paused => &[Resume, Back],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionMap {
    pub schema_version: u32,
    pub contexts: BTreeMap<Context, BTreeMap<Action, Vec<Key>>>,
}
impl Default for ActionMap {
    fn default() -> Self {
        use Action::*;
        use Key::*;
        Self {
            schema_version: 1,
            contexts: BTreeMap::from([
                (
                    Context::Race,
                    BTreeMap::from([
                        (Accelerate, vec![ArrowUp, KeyW]),
                        (Brake, vec![ArrowDown, KeyS]),
                        (SteerLeft, vec![ArrowLeft, KeyA]),
                        (SteerRight, vec![ArrowRight, KeyD]),
                        (Pause, vec![Escape, KeyP]),
                        (Punch, vec![KeyZ]),
                        (Kick, vec![KeyX]),
                        (Backhand, vec![KeyC]),
                    ]),
                ),
                (
                    Context::Menu,
                    BTreeMap::from([
                        (Confirm, vec![Enter, Space]),
                        (Back, vec![Escape]),
                        (NavigateUp, vec![ArrowUp, KeyW]),
                        (NavigateDown, vec![ArrowDown, KeyS]),
                        (NavigateLeft, vec![ArrowLeft, KeyA]),
                        (NavigateRight, vec![ArrowRight, KeyD]),
                    ]),
                ),
                (
                    Context::Paused,
                    BTreeMap::from([(Resume, vec![Enter, KeyP]), (Back, vec![Escape])]),
                ),
            ]),
        }
    }
}
impl ActionMap {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.contexts.len() != 3 {
            return Err(Error(
                "unsupported bindings schema or missing contexts".into(),
            ));
        }
        for context in Context::ALL {
            let map = self
                .contexts
                .get(&context)
                .ok_or_else(|| Error(format!("missing {context:?} context")))?;
            if map.keys().any(|a| {
                !context.required().contains(a)
                    && !(context == Context::Race
                        && [Action::Punch, Action::Kick, Action::Backhand].contains(a))
            }) || context.required().iter().any(|a| !map.contains_key(a))
            {
                return Err(Error(format!("missing or invalid actions in {context:?}")));
            }
            let mut owners = BTreeMap::new();
            for (action, keys) in map {
                if keys.is_empty() || keys.len() > 4 {
                    return Err(Error(format!("{context:?}/{action:?}: provide 1..4 keys")));
                }
                for key in keys {
                    if let Some(previous) = owners.insert(*key, *action) {
                        return Err(Error(format!(
                            "{context:?}: {key:?} conflicts between {previous:?} and {action:?}"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn rebind(&mut self, context: Context, action: Action, keys: Vec<Key>) -> Result<()> {
        let mut next = self.clone();
        next.contexts
            .get_mut(&context)
            .ok_or_else(|| Error("missing context".into()))?
            .insert(action, keys);
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn reset(&mut self, context: Option<Context>) {
        if let Some(context) = context {
            self.contexts
                .insert(context, Self::default().contexts.remove(&context).unwrap());
        } else {
            *self = Self::default();
        }
    }
    pub fn active(
        &self,
        context: Context,
        pressed: impl IntoIterator<Item = Key>,
    ) -> BTreeSet<Action> {
        let pressed: BTreeSet<_> = pressed.into_iter().collect();
        self.contexts
            .get(&context)
            .into_iter()
            .flat_map(|map| map.iter())
            .filter(|(_, keys)| keys.iter().any(|key| pressed.contains(key)))
            .map(|(action, _)| *action)
            .collect()
    }
    pub fn read(path: &Path) -> Result<Self> {
        let map: Self = storage::read(path)?;
        map.validate()?;
        Ok(map)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        storage::save(self, path)
    }
}
