use crate::crypto::{AesKey, AesXtsKey, KeyParseError};
use ini::Properties;
use snafu::{ResultExt, Snafu};
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct KeySet {
    // I don't want to deal with all key derivation machinery right now, so I'll just add the keys I need for now.
    header_key: Option<AesXtsKey>,
    title_kek: [Option<AesKey>; 0x10],
}

pub struct KeyName {
    pub key_name: &'static str,
    pub index: Option<usize>,
}

impl Debug for KeyName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(index) = self.index {
            write!(f, "{}_{:02x}", self.key_name, index)
        } else {
            write!(f, "{}", self.key_name)
        }
    }
}

impl Display for KeyName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Snafu, Debug)]
pub struct MissingKeyError {
    pub key_name: KeyName,
}

#[derive(Snafu, Debug)]
pub enum KeySetParseError {
    #[snafu(display("Could not parse keyset file at line {} column {}: {}", line, col, msg))]
    FileParse {
        line: usize,
        col: usize,
        msg: String,
    },
    #[snafu(display("Could not parse key {}: {}", key_name, source))]
    KeyParse {
        key_name: KeyName,
        source: KeyParseError,
    },
}

#[derive(Snafu, Debug)]
pub enum SystemKeysetError {
    Parse { source: KeySetParseError },
    NotFound { tried: Vec<PathBuf> },
    Io { source: std::io::Error },
}

impl KeySet {
    /// Loads a keyset from a file. The file format is the same as the one used by Hactool.
    /// By default the file is searched in the ".switch" dir in
    ///     the user's home directory and in "switch" in user's config directory (according to `dirs-next` crate).
    ///
    /// One can also provide a path to a custom keyset file, then the system directories are ignored.
    pub fn from_system(key_path: Option<&Path>) -> Result<Self, SystemKeysetError> {
        let paths = if let Some(key_path) = key_path {
            vec![Some(key_path.into())]
        } else {
            vec![
                dirs_next::config_dir().map(|mut v| {
                    v.push("switch");
                    v.push("prod.keys");
                    v
                }),
                dirs_next::home_dir().map(|mut v| {
                    v.push(".switch");
                    v.push("prod.keys");
                    v
                }),
            ]
        }
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        for path in paths.iter() {
            match std::fs::read_to_string(path) {
                Ok(r) => return Self::from_file_contents(&r).context(ParseSnafu {}),
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    continue;
                }
                e => {
                    e.context(IoSnafu)?;
                }
            }
        }

        Err(SystemKeysetError::NotFound { tried: paths })
    }

    pub fn from_file_contents(contents: &str) -> Result<Self, KeySetParseError> {
        let ini = ini::Ini::load_from_str_noescape(contents).map_err(|e| {
            KeySetParseError::FileParse {
                line: e.line,
                col: e.col,
                msg: e.msg,
            }
        })?;
        let props = ini.general_section();

        fn parse_key<K: FromStr<Err = KeyParseError>>(
            props: &Properties,
            name: &'static str,
        ) -> Result<Option<K>, KeySetParseError> {
            props
                .get(name)
                .map(|s| s.parse())
                .transpose()
                .map_err(|source| KeySetParseError::KeyParse {
                    key_name: KeyName {
                        key_name: name,
                        index: None,
                    },
                    source,
                })
        }

        fn parse_keys<K: FromStr<Err = KeyParseError> + Copy, const N: usize>(
            props: &Properties,
            name: &'static str,
        ) -> Result<[Option<K>; N], KeySetParseError> {
            let mut result = [None; N];
            for (i, result) in result.iter_mut().enumerate() {
                let key_name = KeyName {
                    key_name: name,
                    index: Some(i),
                };
                let key = props
                    .get(&key_name.to_string())
                    .map(|s| s.parse())
                    .transpose()
                    .map_err(|source| KeySetParseError::KeyParse { key_name, source })?;
                *result = key;
            }
            Ok(result)
        }

        Ok(Self {
            header_key: parse_key(props, "header_key")?,
            title_kek: parse_keys(props, "titlekek")?,
        })
    }

    pub fn header_key(&self) -> Result<AesXtsKey, MissingKeyError> {
        self.header_key.ok_or(MissingKeyError {
            key_name: KeyName {
                key_name: "header_key",
                index: None,
            },
        })
    }

    pub fn title_kek(&self, index: usize) -> Result<AesKey, MissingKeyError> {
        self.title_kek[index].ok_or(MissingKeyError {
            key_name: KeyName {
                key_name: "title_kek",
                index: Some(index),
            },
        })
    }
}
