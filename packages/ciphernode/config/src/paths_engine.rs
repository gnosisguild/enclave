use path_clean::clean;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct PathsEngine {
    name: String,
    /// Config dir override. This allows us to set specific config_dir location in the config file
    /// wherever that is. This will locate the `key` file
    config_dir_override: Option<PathBuf>,
    /// Configuration file as found derived from the passed in arg eg. `--config`
    /// If this is set it will always be a fully qualified path as this will be the result of
    /// searching through the filesystem.
    found_config_file: Option<PathBuf>,
    /// Data dir override. This allows us to set specific data_dir location in the config file
    /// wherever that is. This will locate the `db` file
    data_dir_override: Option<PathBuf>,
    /// This can either be a fully qualified path to a specific db file or a relative path to the
    /// data_dir location
    db_file_override: Option<PathBuf>,
    /// This can either be a fully qualified path to a specific key file or a relative path to the
    /// config_dir location
    key_file_override: Option<PathBuf>,
    /// Input from the OS as to where the default data dir is
    default_data_dir: PathBuf,
    /// Input from the OS as to where the default config dir is
    default_config_dir: PathBuf,
    /// A reference to the cwd
    cwd: PathBuf,
}

pub const DEFAULT_CONFIG_NAME: &str = "enclave.config.yaml";
pub const DEFAULT_KEY_NAME: &str = "key";
pub const DEFAULT_DB_NAME: &str = "db";

// Find the config file is specified anywhere upstream from cwd and if found then locate the
// data and config folders under .enclave/data and .enclave/config relative to the location of
// the config file. Otherwise locate config in the default app configuration folder and data in
// the default app data folder.
impl PathsEngine {
    pub fn new(
        name: &str,
        cwd: &PathBuf,
        default_data_dir: &PathBuf,
        default_config_dir: &PathBuf,
        config_dir_override: Option<&PathBuf>,
        found_config_file: Option<&PathBuf>,
        data_dir_override: Option<&PathBuf>,
        db_file_override: Option<&PathBuf>,
        key_file_override: Option<&PathBuf>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            cwd: PathBuf::from(cwd),
            default_data_dir: PathBuf::from(default_data_dir),
            default_config_dir: PathBuf::from(default_config_dir),
            config_dir_override: config_dir_override.map(PathBuf::from),
            found_config_file: found_config_file.map(PathBuf::from),
            data_dir_override: data_dir_override.map(PathBuf::from),
            db_file_override: db_file_override.map(PathBuf::from),
            key_file_override: key_file_override.map(PathBuf::from),
        }
    }

    /// Full path to the config file that will be loaded
    pub fn config_file(&self) -> PathBuf {
        if let Some(file) = self.found_config_file.clone() {
            return clean(file);
        }
        clean(self.default_config_dir.join(DEFAULT_CONFIG_NAME))
    }

    /// Full path to the key file containing secret key
    pub fn key_file(&self) -> PathBuf {
        if let Some(key_file) = self.key_file_override.clone() {
            if key_file.is_absolute() {
                return clean(key_file);
            } else {
                return clean(self.get_config_dir().join(&self.name).join(key_file));
            }
        }

        clean(
            self.get_config_dir()
                .join(&self.name)
                .join(DEFAULT_KEY_NAME),
        )
    }

    /// Full path to the database file containing the db
    pub fn db_file(&self) -> PathBuf {
        if let Some(data_file) = self.db_file_override.clone() {
            if data_file.is_absolute() {
                return clean(data_file);
            } else {
                return clean(self.get_data_dir().join(&self.name).join(data_file));
            }
        }

        clean(self.get_data_dir().join(&self.name).join(DEFAULT_DB_NAME))
    }

    pub fn relative_to_config(&self, path: &PathBuf) -> PathBuf {
        if path.is_absolute() {
            return PathBuf::from(path);
        }

        let config_file = self.config_file();

        // Most of the time the config_file will be in a folder
        // In case it is not use the cwd
        let relative_from = config_file.parent().unwrap_or(&self.cwd);

        clean(PathBuf::from(relative_from).join(path))
    }

    fn get_config_dir(&self) -> PathBuf {
        if let Some(config_dir) = self.config_dir_override.clone() {
            return config_dir;
        }
        if let Some(root_dir) = self.get_root_dir() {
            return root_dir.join("config");
        }

        return self.default_config_dir.clone();
    }

    fn get_data_dir(&self) -> PathBuf {
        if let Some(data_dir) = self.data_dir_override.clone() {
            return data_dir;
        }

        if let Some(root_dir) = self.get_root_dir() {
            return root_dir.join("data");
        }

        return self.default_data_dir.clone();
    }

    fn get_root_dir(&self) -> Option<PathBuf> {
        if let Some(file) = self.found_config_file.clone() {
            if let Some(parent) = file.parent() {
                return Some(PathBuf::from(parent).join(".enclave"));
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::PathsEngine;

    // Helper structs for the table test
    struct TestCase {
        name: &'static str,
        input: PathsInput,
        expected: PathsExpected,
    }

    struct PathsInput {
        name: &'static str,
        cwd: &'static str,
        default_data_dir: &'static str,
        default_config_dir: &'static str,
        config_dir_override: Option<&'static str>,
        found_config_file: Option<&'static str>,
        data_dir_override: Option<&'static str>,
        db_file_override: Option<&'static str>,
        key_file_override: Option<&'static str>,
    }

    struct PathsExpected {
        config_file: &'static str,
        key_file: &'static str,
        db_file: &'static str,
    }

    fn test_cases(test_cases: Vec<TestCase>) {
        // Run all test cases
        for test_case in test_cases {
            println!("Running test case: {}", test_case.name);

            // Convert string inputs to PathBufs
            let default_data_dir = PathBuf::from(test_case.input.default_data_dir);
            let default_config_dir = PathBuf::from(test_case.input.default_config_dir);
            let config_dir = test_case.input.config_dir_override.map(PathBuf::from);
            let config_file = test_case.input.found_config_file.map(PathBuf::from);
            let data_dir_override = test_case.input.data_dir_override.map(PathBuf::from);
            let db_file = test_case.input.db_file_override.map(PathBuf::from);
            let key_file = test_case.input.key_file_override.map(PathBuf::from);
            let cwd = PathBuf::from(test_case.input.cwd);

            let paths = PathsEngine::new(
                test_case.input.name,
                &cwd,
                &default_data_dir,
                &default_config_dir,
                config_dir.as_ref(),
                config_file.as_ref(),
                data_dir_override.as_ref(),
                db_file.as_ref(),
                key_file.as_ref(),
            );

            assert_eq!(
                paths.config_file(),
                PathBuf::from(test_case.expected.config_file),
                "Failed config_file assertion for test case: {}",
                test_case.name
            );
            assert_eq!(
                paths.key_file(),
                PathBuf::from(test_case.expected.key_file),
                "Failed key_file assertion for test case: {}",
                test_case.name
            );
            assert_eq!(
                paths.db_file(),
                PathBuf::from(test_case.expected.db_file),
                "Failed db_file assertion for test case: {}",
                test_case.name
            );
        }
    }

    #[test]
    fn test_all() {
        test_cases(vec![
            TestCase {
                name: "Defaults",
                input: PathsInput {
                    name: "default",
                    cwd: "/no/matter",
                    default_data_dir: "/home/user/.local/share/enclave",
                    default_config_dir: "/home/user/.config/enclave",
                    config_dir_override: None,
                    found_config_file: None,
                    data_dir_override: None,
                    db_file_override: None,
                    key_file_override: None,
                },
                expected: PathsExpected {
                    config_file: "/home/user/.config/enclave/enclave.config.yaml",
                    key_file: "/home/user/.config/enclave/default/key",
                    db_file: "/home/user/.local/share/enclave/default/db",
                },
            },
            TestCase {
                name: "Config file found",
                input: PathsInput {
                    name: "default",
                    cwd: "/no/matter",
                    default_data_dir: "/home/user/.local/share/enclave/data",
                    default_config_dir: "/home/user/.config/enclave/config",
                    config_dir_override: None,
                    found_config_file: Some("/foo/some.config.yaml"),
                    data_dir_override: None,
                    db_file_override: None,
                    key_file_override: None,
                },
                expected: PathsExpected {
                    config_file: "/foo/some.config.yaml",
                    key_file: "/foo/.enclave/config/default/key",
                    db_file: "/foo/.enclave/data/default/db",
                },
            },
            TestCase {
                name: "Data dir override",
                input: PathsInput {
                    name: "default",
                    cwd: "/no/matter",
                    default_data_dir: "/home/user/.local/share/enclave/data",
                    default_config_dir: "/home/user/.config/enclave/config",
                    config_dir_override: None,
                    found_config_file: Some("/foo/some.config.yaml"),
                    data_dir_override: Some("/path/to/data"),
                    db_file_override: None,
                    key_file_override: None,
                },
                expected: PathsExpected {
                    config_file: "/foo/some.config.yaml",
                    key_file: "/foo/.enclave/config/default/key",
                    db_file: "/path/to/data/default/db",
                },
            },
            TestCase {
                name: "Config dir override",
                input: PathsInput {
                    name: "default",
                    cwd: "/no/matter",
                    default_data_dir: "/home/user/.local/share/enclave/data",
                    default_config_dir: "/home/user/.config/enclave/config",
                    config_dir_override: Some("/confy/stuff"),
                    found_config_file: Some("/foo/some.config.yaml"),
                    data_dir_override: Some("/path/to/data"),
                    db_file_override: None,
                    key_file_override: None,
                },
                expected: PathsExpected {
                    config_file: "/foo/some.config.yaml",
                    key_file: "/confy/stuff/default/key",
                    db_file: "/path/to/data/default/db",
                },
            },
            TestCase {
                name: "Key file override absolute",
                input: PathsInput {
                    cwd: "/no/matter",
                    name: "default",
                    default_data_dir: "/home/user/.local/share/enclave/data",
                    default_config_dir: "/home/user/.config/enclave/config",
                    config_dir_override: Some("/confy/stuff"),
                    found_config_file: Some("/foo/some.config.yaml"),
                    data_dir_override: Some("/path/to/data"),
                    db_file_override: None,
                    key_file_override: Some("/ding/bat/key_file"),
                },
                expected: PathsExpected {
                    config_file: "/foo/some.config.yaml",
                    key_file: "/ding/bat/key_file",
                    db_file: "/path/to/data/default/db",
                },
            },
            TestCase {
                name: "Key file override relative",
                input: PathsInput {
                    cwd: "/no/matter",
                    name: "default",
                    default_data_dir: "/home/user/.local/share/enclave/data",
                    default_config_dir: "/home/user/.config/enclave/config",
                    config_dir_override: Some("/confy/stuff"),
                    found_config_file: Some("/foo/some.config.yaml"),
                    data_dir_override: Some("/path/to/data"),
                    db_file_override: None,
                    key_file_override: Some("../bat/key_file"),
                },
                expected: PathsExpected {
                    config_file: "/foo/some.config.yaml",
                    key_file: "/confy/stuff/bat/key_file",
                    db_file: "/path/to/data/default/db",
                },
            },
            TestCase {
                name: "Data file override absolute",
                input: PathsInput {
                    cwd: "/no/matter",
                    name: "default",
                    default_data_dir: "/home/user/.local/share/enclave/data",
                    default_config_dir: "/home/user/.config/enclave/config",
                    config_dir_override: Some("/confy/stuff"),
                    found_config_file: Some("/foo/some.config.yaml"),
                    data_dir_override: Some("/path/to/data"),
                    db_file_override: Some("/ding/blat/foo/my/data"),
                    key_file_override: Some("../bat/key_file"),
                },
                expected: PathsExpected {
                    config_file: "/foo/some.config.yaml",
                    key_file: "/confy/stuff/bat/key_file",
                    db_file: "/ding/blat/foo/my/data",
                },
            },
            TestCase {
                name: "Data file override relative",
                input: PathsInput {
                    name: "default",
                    cwd: "/no/matter",
                    default_data_dir: "/home/user/.local/share/enclave/data",
                    default_config_dir: "/home/user/.config/enclave/config",
                    config_dir_override: Some("/confy/stuff"),
                    found_config_file: Some("/foo/some.config.yaml"),
                    data_dir_override: Some("/path/to/data"),
                    db_file_override: Some("../../yes"),
                    key_file_override: Some("../bat/key_file"),
                },
                expected: PathsExpected {
                    config_file: "/foo/some.config.yaml",
                    key_file: "/confy/stuff/bat/key_file",
                    db_file: "/path/to/yes",
                },
            },
        ]);
    }
}
