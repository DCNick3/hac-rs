use crate::crypto::{AesKey, AesXtsKey, KeyParseError, TitleKey};
use crate::formats::ticket::Ticket;
use crate::ids::{IdParseError, RightsId};
use binrw::{BinRead, BinWrite};
use ini::Properties;
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone)]
pub struct KeySet {
    // I don't want to deal with all key derivation machinery right now, so I'll just add the keys I need for now.
    header_key: Option<AesXtsKey>,
    title_kek: [Option<AesKey>; 0x10],
    key_area_key_application: [Option<AesKey>; 0x20],
    key_area_key_ocean: [Option<AesKey>; 0x20],
    key_area_key_system: [Option<AesKey>; 0x20],
    title_keys: HashMap<RightsId, TitleKey>,
}

pub struct KeyName {
    pub key_name: &'static str,
    pub index: Option<u8>,
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
    CommonKeysFileParse {
        line: usize,
        col: usize,
        msg: String,
    },
    #[snafu(display(
        "Could not parse title keys file at line {} column {}: {}",
        line,
        col,
        msg
    ))]
    TitleKeysFileParse {
        line: usize,
        col: usize,
        msg: String,
    },

    #[snafu(display("Could not parse key {}: {}", key_name, source))]
    KeyParse {
        key_name: KeyName,
        source: KeyParseError,
    },
    #[snafu(display("Could not parse rightsid {}: {}", rights_id, source))]
    RightsIdParse {
        rights_id: String,
        source: IdParseError,
    },
    #[snafu(display("Could not parse title key for rightsid {:?}: {}", rights_id, source))]
    TitleKeyParse {
        rights_id: RightsId,
        source: KeyParseError,
    },
}

#[derive(Snafu, Debug)]
pub enum SystemKeysetError {
    Parse { source: KeySetParseError },
    NotFound { tried: Vec<PathBuf> },
    Io { source: std::io::Error },
}

#[derive(Snafu, Debug)]
#[snafu(display("Missing title key for RightsId {}", rights_id))]
pub struct MissingTitleKeyError {
    pub rights_id: RightsId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum KeyAreaKeyIndex {
    Application = 0,
    Ocean = 1,
    System = 2,
}

impl KeySet {
    /// Loads a keyset from a file. The file format is the same as the one used by Hactool.
    /// By default the file is searched in the ".switch" dir in
    ///     the user's home directory and in "switch" in user's config directory (according to `dirs-next` crate).
    ///
    /// One can also provide a path to a custom keyset file, then the system directories are ignored.
    pub fn from_system(keys_dir: Option<&Path>) -> Result<Self, SystemKeysetError> {
        let paths = if let Some(key_path) = keys_dir {
            vec![Some(key_path.into())]
        } else {
            vec![
                dirs_next::config_dir().map(|v| v.join("switch")),
                dirs_next::home_dir().map(|v| v.join(".switch")),
            ]
        }
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let find_file = |file_name: &str| -> Result<PathBuf, SystemKeysetError> {
            for path in &paths {
                let file_path = path.join(file_name);
                if file_path.exists() {
                    return Ok(file_path);
                }
            }

            Err(SystemKeysetError::NotFound {
                tried: paths
                    .clone()
                    .into_iter()
                    .map(|p| p.join(file_name))
                    .collect(),
            })
        };

        let prod_keys_path = find_file("prod.keys")?;
        let title_keys_path = find_file("title.keys").ok();

        let prod_keys = std::fs::read_to_string(&prod_keys_path).context(IoSnafu)?;
        let title_keys = title_keys_path
            .as_ref()
            .map(|p| std::fs::read_to_string(p).context(IoSnafu))
            .transpose()?;

        Self::from_file_contents(&prod_keys, title_keys.as_deref().unwrap_or(""))
            .context(ParseSnafu {})
    }

    pub fn from_file_contents(
        common_keys: &str,
        title_keys: &str,
    ) -> Result<Self, KeySetParseError> {
        let common_keys = ini::Ini::load_from_str_noescape(common_keys).map_err(|e| {
            KeySetParseError::CommonKeysFileParse {
                line: e.line,
                col: e.col,
                msg: e.msg,
            }
        })?;
        let common_keys = common_keys.general_section();

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
                    index: Some(i as u8),
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

        let title_keys_ini = ini::Ini::load_from_str_noescape(title_keys).map_err(|e| {
            KeySetParseError::TitleKeysFileParse {
                line: e.line,
                col: e.col,
                msg: e.msg,
            }
        })?;

        let mut title_keys = HashMap::new();
        for (rights_id, title_key) in title_keys_ini.general_section().iter() {
            let rights_id = rights_id.parse().context(RightsIdParseSnafu {
                rights_id: rights_id.to_string(),
            })?;
            let title_key = title_key
                .parse()
                .context(TitleKeyParseSnafu { rights_id })?;
            title_keys.insert(rights_id, title_key);
        }

        Ok(Self {
            header_key: parse_key(common_keys, "header_key")?,
            title_kek: parse_keys(common_keys, "titlekek")?,
            key_area_key_application: parse_keys(common_keys, "key_area_key_application")?,
            key_area_key_ocean: parse_keys(common_keys, "key_area_key_ocean")?,
            key_area_key_system: parse_keys(common_keys, "key_area_key_system")?,
            title_keys,
        })
    }
}

impl KeySet {
    pub fn header_key(&self) -> Result<AesXtsKey, MissingKeyError> {
        self.header_key.ok_or(MissingKeyError {
            key_name: KeyName {
                key_name: "header_key",
                index: None,
            },
        })
    }

    pub fn import_ticket(&mut self, ticket: &Ticket) {
        self.title_keys
            .insert(ticket.rights_id, ticket.title_key(self));
    }

    pub fn title_kek(&self, master_key_revision: u8) -> Result<AesKey, MissingKeyError> {
        self.title_kek[master_key_revision as usize].ok_or(MissingKeyError {
            key_name: KeyName {
                key_name: "title_kek",
                index: Some(master_key_revision),
            },
        })
    }

    pub fn key_area_key(
        &self,
        master_key_revision: u8,
        key_area_key_index: KeyAreaKeyIndex,
    ) -> Result<AesKey, MissingKeyError> {
        let (kek_array, name) = match key_area_key_index {
            KeyAreaKeyIndex::Application => {
                (&self.key_area_key_application, "key_area_key_application")
            }
            KeyAreaKeyIndex::Ocean => (&self.key_area_key_ocean, "key_area_key_ocean"),
            KeyAreaKeyIndex::System => (&self.key_area_key_system, "key_area_key_system"),
        };
        kek_array[master_key_revision as usize].ok_or(MissingKeyError {
            key_name: KeyName {
                key_name: name,
                index: Some(master_key_revision),
            },
        })
    }

    pub fn title_key(&self, rights_id: &RightsId) -> Result<TitleKey, MissingTitleKeyError> {
        self.title_keys
            .get(rights_id)
            .copied()
            .ok_or(MissingTitleKeyError {
                rights_id: *rights_id,
            })
    }
}
