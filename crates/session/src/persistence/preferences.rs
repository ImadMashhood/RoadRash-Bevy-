use super::{MAX_BYTES, Result};
use crate::flow::Settings;
use rr_input::{Action, ActionMap, Context, Key};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u8,
    pub settings: Settings,
    pub bindings: ActionMap,
    pub active_slot: u8,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            settings: Settings::default(),
            bindings: ActionMap::default(),
            active_slot: 0,
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.active_slot > 4 {
            return Err("Unsupported preferences version/slot".into());
        }
        self.settings.validate()?;
        self.bindings.validate().map_err(|e| e.to_string())
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec_pretty(self).map_err(|e| e.to_string())
    }
    pub fn decode(bytes: &[u8]) -> Result<(Self, Vec<String>)> {
        if bytes.len() > MAX_BYTES {
            return Err("Preferences exceed 64 KiB".into());
        }
        let mut raw: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| format!("Invalid preferences JSON: {e}"))?;
        let mut warnings = vec![];
        if let Some(bindings) = raw.get_mut("bindings") {
            let version = bindings
                .get("schema_version")
                .and_then(|v| v.as_u64())
                .unwrap_or(1);
            if version != 1 {
                return Err("Unsupported input bindings version".into());
            }
            let mut map = ActionMap::default();
            if let Some(contexts) = bindings.get("contexts").and_then(|v| v.as_object()) {
                for (name, actions) in contexts {
                    let context: Context = serde_json::from_value(name.clone().into())
                        .map_err(|_| format!("Unsupported input context {name}"))?;
                    let actions = actions
                        .as_object()
                        .ok_or("Input actions must be an object")?;
                    let mut candidate = map.clone();
                    for (name, keys) in actions {
                        let action: Action = serde_json::from_value(name.clone().into())
                            .map_err(|_| format!("Unsupported input action {name}"))?;
                        match serde_json::from_value::<Vec<Key>>(keys.clone()) {
                            Ok(keys) => {
                                candidate.contexts.get_mut(&context)
                                    .ok_or("Input context")?.insert(action, keys);
                            }
                            Err(_) => warnings.push(format!(
                                "Unsupported old key name for {context:?}/{action:?}; its default binding was retained."
                            )),
                        }
                    }
                    if candidate.validate().is_ok() {
                        map = candidate;
                    } else {
                        warnings.push(format!("Conflicting/invalid {context:?} bindings were reset to that context's defaults."));
                    }
                }
            }
            *bindings = serde_json::to_value(map).map_err(|e| e.to_string())?;
        }
        let value: Self = serde_json::from_value(raw).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok((value, warnings))
    }
}
