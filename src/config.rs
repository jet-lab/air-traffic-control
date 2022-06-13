// Copyright (C) 2022 JET PROTOCOL HOLDINGS, LLC.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde::Deserialize;
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Config {
    pub settings: Settings,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub rpc_endpoint: String,
    pub percentages: PercentageSettings,
    pub port: u16,
    pub workers: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(rename_all = "camelCase")]
pub struct PercentageSettings {
    pub rpc_success: f32,
    pub tx_success: f32,
}

impl Default for PercentageSettings {
    fn default() -> Self {
        Self {
            rpc_success: 0.65,
            tx_success: 0.8,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings {
                rpc_endpoint: "http://127.0.0.1:8899".into(),
                percentages: Default::default(),
                port: 8080,
                workers: 10,
            },
        }
    }
}

impl TryFrom<PathBuf> for Config {
    type Error = Box<dyn std::error::Error>;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        let val = read_to_string(p)?;
        Self::try_from(val.as_str()).map_err(Into::into)
    }
}

impl TryFrom<&str> for Config {
    type Error = serde_json::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(s).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::read_to_string;
    use std::str::FromStr;

    #[test]
    fn config_default() {
        assert_eq!(
            Config::default(),
            Config {
                settings: Settings {
                    rpc_endpoint: "http://127.0.0.1:8899".into(),
                    percentages: PercentageSettings {
                        rpc_success: 0.65,
                        tx_success: 0.8,
                    },
                    port: 8080,
                    workers: 10
                }
            }
        );
    }

    #[test]
    fn config_from_str() {
        let val = read_to_string("./.github/resources/test_config.json").unwrap();
        assert_eq!(
            Config::try_from(val.as_str()).unwrap(),
            Config {
                settings: Settings {
                    rpc_endpoint: "http://localhost:8899".into(),
                    percentages: PercentageSettings {
                        rpc_success: 1.0,
                        tx_success: 0.5
                    },
                    port: 8080,
                    workers: 10
                }
            }
        );
    }

    #[test]
    fn config_from_path() {
        let p = PathBuf::from_str("./.github/resources/test_config.json").unwrap();
        assert_eq!(
            Config::try_from(p).unwrap(),
            Config {
                settings: Settings {
                    rpc_endpoint: "http://localhost:8899".into(),
                    percentages: PercentageSettings {
                        rpc_success: 1.0,
                        tx_success: 0.5
                    },
                    port: 8080,
                    workers: 10
                }
            }
        );
    }
}
