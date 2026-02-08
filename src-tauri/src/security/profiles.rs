use crate::security::ConnectionProfile;
use std::fs;
use std::path::PathBuf;

/// Errors from profile storage operations.
#[derive(Debug)]
pub enum ProfileError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    NotFound(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileError::Io(e) => write!(f, "Profile I/O error: {}", e),
            ProfileError::Serialization(e) => write!(f, "Profile serialization error: {}", e),
            ProfileError::NotFound(id) => write!(f, "Profile not found: {}", id),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Manages connection profiles as JSON files in a directory.
/// Profiles are stored WITHOUT password fields - credential_key references Stronghold.
pub struct ProfileStore {
    profiles_dir: PathBuf,
}

impl ProfileStore {
    /// Create a new ProfileStore that manages profiles in the given directory.
    pub fn new(profiles_dir: PathBuf) -> Self {
        Self { profiles_dir }
    }

    /// Ensure the profiles directory exists.
    fn ensure_dir(&self) -> Result<(), ProfileError> {
        fs::create_dir_all(&self.profiles_dir).map_err(ProfileError::Io)
    }

    /// Get the file path for a profile by its ID.
    fn profile_path(&self, id: &str) -> PathBuf {
        self.profiles_dir.join(format!("{}.json", id))
    }

    /// Save a profile to disk as JSON. Overwrites if it already exists.
    pub fn save_profile(&self, profile: &ConnectionProfile) -> Result<(), ProfileError> {
        self.ensure_dir()?;
        let path = self.profile_path(&profile.id);
        let json =
            serde_json::to_string_pretty(profile).map_err(ProfileError::Serialization)?;
        fs::write(path, json).map_err(ProfileError::Io)
    }

    /// Load a profile from disk by its ID.
    pub fn load_profile(&self, id: &str) -> Result<ConnectionProfile, ProfileError> {
        let path = self.profile_path(id);
        if !path.exists() {
            return Err(ProfileError::NotFound(id.to_string()));
        }
        let json = fs::read_to_string(path).map_err(ProfileError::Io)?;
        serde_json::from_str(&json).map_err(ProfileError::Serialization)
    }

    /// List all profiles stored on disk.
    pub fn list_profiles(&self) -> Result<Vec<ConnectionProfile>, ProfileError> {
        self.ensure_dir()?;

        let mut profiles = Vec::new();
        let entries = fs::read_dir(&self.profiles_dir).map_err(ProfileError::Io)?;

        for entry in entries {
            let entry = entry.map_err(ProfileError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let json = fs::read_to_string(&path).map_err(ProfileError::Io)?;
                match serde_json::from_str::<ConnectionProfile>(&json) {
                    Ok(profile) => profiles.push(profile),
                    Err(_) => {
                        // Skip malformed files
                        continue;
                    }
                }
            }
        }

        // Sort by name for consistent ordering
        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(profiles)
    }

    /// Delete a profile from disk by its ID.
    pub fn delete_profile(&self, id: &str) -> Result<(), ProfileError> {
        let path = self.profile_path(id);
        if !path.exists() {
            return Err(ProfileError::NotFound(id.to_string()));
        }
        fs::remove_file(path).map_err(ProfileError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_profile(id: &str, name: &str) -> ConnectionProfile {
        ConnectionProfile {
            id: id.to_string(),
            name: name.to_string(),
            engine: "postgresql".to_string(),
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: Some("testdb".to_string()),
            username: Some("admin".to_string()),
            credential_key: Some(format!("upsert_cred_{}", id)),
            file_path: None,
            read_only: false,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_save_and_load_profile() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));

        let profile = make_profile("p1", "My Connection");
        store.save_profile(&profile).unwrap();

        let loaded = store.load_profile("p1").unwrap();
        assert_eq!(loaded.id, "p1");
        assert_eq!(loaded.name, "My Connection");
        assert_eq!(loaded.engine, "postgresql");
        assert_eq!(loaded.host.as_deref(), Some("localhost"));
        assert_eq!(loaded.port, Some(5432));
        assert_eq!(
            loaded.credential_key.as_deref(),
            Some("upsert_cred_p1")
        );
    }

    #[test]
    fn test_load_nonexistent_profile() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));
        store.ensure_dir().unwrap();

        let result = store.load_profile("nonexistent");
        assert!(result.is_err());
        match result {
            Err(ProfileError::NotFound(id)) => assert_eq!(id, "nonexistent"),
            other => panic!("Expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_list_profiles() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));

        store
            .save_profile(&make_profile("p1", "B Connection"))
            .unwrap();
        store
            .save_profile(&make_profile("p2", "A Connection"))
            .unwrap();
        store
            .save_profile(&make_profile("p3", "C Connection"))
            .unwrap();

        let profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 3);
        // Sorted by name
        assert_eq!(profiles[0].name, "A Connection");
        assert_eq!(profiles[1].name, "B Connection");
        assert_eq!(profiles[2].name, "C Connection");
    }

    #[test]
    fn test_list_empty() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));

        let profiles = store.list_profiles().unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_delete_profile() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));

        store
            .save_profile(&make_profile("p1", "My Connection"))
            .unwrap();
        assert!(store.load_profile("p1").is_ok());

        store.delete_profile("p1").unwrap();
        assert!(store.load_profile("p1").is_err());
    }

    #[test]
    fn test_delete_nonexistent_profile() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));
        store.ensure_dir().unwrap();

        let result = store.delete_profile("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_overwrite_profile() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));

        let mut profile = make_profile("p1", "Original");
        store.save_profile(&profile).unwrap();

        profile.name = "Updated".to_string();
        store.save_profile(&profile).unwrap();

        let loaded = store.load_profile("p1").unwrap();
        assert_eq!(loaded.name, "Updated");
    }

    #[test]
    fn test_profile_json_has_no_password_field() {
        let profile = make_profile("p1", "Test");
        let json = serde_json::to_string_pretty(&profile).unwrap();

        // ConnectionProfile struct has no password field, only credential_key
        assert!(json.contains("credential_key"));
        assert!(!json.contains("\"password\""));
    }

    #[test]
    fn test_profile_with_no_credential_key() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("profiles"));

        let mut profile = make_profile("p1", "SQLite Connection");
        profile.credential_key = None;
        profile.engine = "sqlite".to_string();
        profile.host = None;
        profile.port = None;
        profile.file_path = Some("/path/to/db.sqlite".to_string());

        store.save_profile(&profile).unwrap();
        let loaded = store.load_profile("p1").unwrap();
        assert!(loaded.credential_key.is_none());
        assert_eq!(loaded.file_path.as_deref(), Some("/path/to/db.sqlite"));
    }
}
