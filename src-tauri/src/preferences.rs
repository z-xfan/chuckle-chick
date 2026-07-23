use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPosition {
    pub x: i32,
    pub y: i32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub version: u8,
    pub scale: f64,
    pub always_on_top: bool,
    #[serde(default = "default_true")]
    pub speech_bubbles_enabled: bool,
    pub current_pet: String,
    pub position: Option<SavedPosition>,
}

fn default_true() -> bool {
    true
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            version: 3,
            scale: 1.0,
            always_on_top: true,
            speech_bubbles_enabled: true,
            current_pet: "huangji-daxiao".to_string(),
            position: None,
        }
    }
}

#[derive(Debug)]
struct StateInner {
    preferences: AppPreferences,
    revision: u64,
}

#[derive(Clone, Debug)]
pub struct PersistentPreferences {
    path: Arc<PathBuf>,
    inner: Arc<Mutex<StateInner>>,
}

impl PersistentPreferences {
    pub fn load(path: PathBuf) -> Self {
        let mut migrated = false;
        let preferences = fs::read_to_string(&path)
            .ok()
            .and_then(|contents| serde_json::from_str::<AppPreferences>(&contents).ok())
            .map(|mut preferences| {
                if preferences.version == 1 {
                    preferences.scale = (preferences.scale / 0.75).clamp(0.4, 2.0);
                }
                if preferences.version <= 2 {
                    preferences.version = 3;
                    preferences.speech_bubbles_enabled = true;
                    migrated = true;
                }
                preferences
            })
            .filter(|preferences| preferences.version == 3)
            .unwrap_or_default();
        let state = Self {
            path: Arc::new(path),
            inner: Arc::new(Mutex::new(StateInner {
                preferences,
                revision: 0,
            })),
        };
        if migrated {
            let _ = state.persist();
        }
        state
    }

    pub fn snapshot(&self) -> AppPreferences {
        self.inner
            .lock()
            .expect("preferences lock poisoned")
            .preferences
            .clone()
    }

    pub fn update<F>(&self, update: F) -> Result<(), String>
    where
        F: FnOnce(&mut AppPreferences),
    {
        {
            let mut inner = self.inner.lock().map_err(|error| error.to_string())?;
            update(&mut inner.preferences);
            inner.revision += 1;
        }
        self.persist()
    }

    pub fn update_position(&self, position: SavedPosition) {
        let revision = {
            let Ok(mut inner) = self.inner.lock() else {
                return;
            };
            inner.preferences.position = Some(position);
            inner.revision += 1;
            inner.revision
        };

        let state = self.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            let should_persist = state
                .inner
                .lock()
                .map(|inner| inner.revision == revision)
                .unwrap_or(false);
            if should_persist {
                let _ = state.persist();
            }
        });
    }

    pub fn persist(&self) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(&self.snapshot()).map_err(|error| error.to_string())?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(self.path.as_ref(), json).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temporary_preferences_path() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after unix epoch")
            .as_nanos();
        let counter = PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "chuckle-chick-preferences-{}-{unique}-{counter}.json",
            std::process::id()
        ))
    }

    #[test]
    fn missing_file_uses_defaults() {
        let state = PersistentPreferences::load(temporary_preferences_path());
        let preferences = state.snapshot();

        assert_eq!(preferences.version, 3);
        assert_eq!(preferences.current_pet, "huangji-daxiao");
        assert!(preferences.always_on_top);
        assert!(preferences.speech_bubbles_enabled);
    }

    #[test]
    fn updated_preferences_can_be_reloaded() {
        let path = temporary_preferences_path();
        let state = PersistentPreferences::load(path.clone());
        state
            .update(|preferences| {
                preferences.scale = 1.25;
                preferences.always_on_top = false;
            })
            .expect("preferences should persist");

        let reloaded = PersistentPreferences::load(path.clone()).snapshot();
        assert_eq!(reloaded.scale, 1.25);
        assert!(!reloaded.always_on_top);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn version_one_scale_is_migrated_without_changing_visual_size() {
        let path = temporary_preferences_path();
        fs::write(
            &path,
            r#"{
              "version": 1,
              "scale": 0.75,
              "alwaysOnTop": true,
              "currentPet": "huangji-daxiao",
              "position": null
            }"#,
        )
        .expect("legacy preferences should be written");

        let migrated = PersistentPreferences::load(path.clone()).snapshot();
        assert_eq!(migrated.version, 3);
        assert_eq!(migrated.scale, 1.0);
        assert!(migrated.speech_bubbles_enabled);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn version_two_adds_enabled_speech_bubbles() {
        let path = temporary_preferences_path();
        fs::write(
            &path,
            r#"{
              "version": 2,
              "scale": 1.25,
              "alwaysOnTop": false,
              "currentPet": "huangji-daxiao",
              "position": null
            }"#,
        )
        .expect("version two preferences should be written");

        let migrated = PersistentPreferences::load(path.clone()).snapshot();
        assert_eq!(migrated.version, 3);
        assert_eq!(migrated.scale, 1.25);
        assert!(!migrated.always_on_top);
        assert!(migrated.speech_bubbles_enabled);
        let _ = fs::remove_file(path);
    }
}
