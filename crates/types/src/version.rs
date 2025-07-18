//! Version specification and constraint parsing
//!
//! Implements Python-style version constraints:
//! - `==1.2.3` - Exact version
//! - `>=1.2.0` - Minimum version
//! - `<=2.0.0` - Maximum version
//! - `~=1.2.0` - Compatible release (>=1.2.0,<1.3.0)
//! - `!=1.5.0` - Exclude version
//! - Multiple constraints: `>=1.2,<2.0,!=1.5.0`

use semver::Version;
use serde::{Deserialize, Serialize};
use sps2_errors::VersionError;
use std::fmt;
use std::str::FromStr;

/// A single version constraint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionConstraint {
    Exact(Version),
    GreaterEqual(Version),
    LessEqual(Version),
    Greater(Version),
    Less(Version),
    Compatible(Version),
    NotEqual(Version),
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint
    #[must_use]
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Exact(v) => version == v,
            Self::GreaterEqual(v) => version >= v,
            Self::LessEqual(v) => version <= v,
            Self::Greater(v) => version > v,
            Self::Less(v) => version < v,
            Self::NotEqual(v) => version != v,
            Self::Compatible(v) => {
                // ~=1.2.3 means >=1.2.3,<1.3.0 (patch version updates only)
                // ~=1.2.0 means >=1.2.0,<1.3.0 (patch version updates only)
                // For simplicity, always allow only patch updates for compatible constraints
                version >= v && version.major == v.major && version.minor == v.minor
            }
        }
    }

    /// Parse a single constraint from a string
    fn parse(s: &str) -> Result<Self, VersionError> {
        let s = s.trim();

        if let Some(version_str) = s.strip_prefix("==") {
            let version =
                Version::parse(version_str.trim()).map_err(|e| VersionError::ParseError {
                    message: e.to_string(),
                })?;
            Ok(Self::Exact(version))
        } else if let Some(version_str) = s.strip_prefix(">=") {
            let version =
                Version::parse(version_str.trim()).map_err(|e| VersionError::ParseError {
                    message: e.to_string(),
                })?;
            Ok(Self::GreaterEqual(version))
        } else if let Some(version_str) = s.strip_prefix("<=") {
            let version =
                Version::parse(version_str.trim()).map_err(|e| VersionError::ParseError {
                    message: e.to_string(),
                })?;
            Ok(Self::LessEqual(version))
        } else if let Some(version_str) = s.strip_prefix("!=") {
            let version =
                Version::parse(version_str.trim()).map_err(|e| VersionError::ParseError {
                    message: e.to_string(),
                })?;
            Ok(Self::NotEqual(version))
        } else if let Some(version_str) = s.strip_prefix("~=") {
            let version =
                Version::parse(version_str.trim()).map_err(|e| VersionError::ParseError {
                    message: e.to_string(),
                })?;
            Ok(Self::Compatible(version))
        } else if let Some(version_str) = s.strip_prefix('>') {
            let version =
                Version::parse(version_str.trim()).map_err(|e| VersionError::ParseError {
                    message: e.to_string(),
                })?;
            Ok(Self::Greater(version))
        } else if let Some(version_str) = s.strip_prefix('<') {
            let version =
                Version::parse(version_str.trim()).map_err(|e| VersionError::ParseError {
                    message: e.to_string(),
                })?;
            Ok(Self::Less(version))
        } else {
            Err(VersionError::InvalidConstraint {
                input: s.to_string(),
            })
        }
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "=={v}"),
            Self::GreaterEqual(v) => write!(f, ">={v}"),
            Self::LessEqual(v) => write!(f, "<={v}"),
            Self::Greater(v) => write!(f, ">{v}"),
            Self::Less(v) => write!(f, "<{v}"),
            Self::Compatible(v) => write!(f, "~={v}"),
            Self::NotEqual(v) => write!(f, "!={v}"),
        }
    }
}

/// A version specification that can contain multiple constraints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionSpec {
    constraints: Vec<VersionConstraint>,
}

impl VersionSpec {
    /// Create a version spec from a single constraint
    #[must_use]
    pub fn single(constraint: VersionConstraint) -> Self {
        Self {
            constraints: vec![constraint],
        }
    }

    /// Create an exact version spec
    #[must_use]
    pub fn exact(version: Version) -> Self {
        Self::single(VersionConstraint::Exact(version))
    }

    /// Check if a version satisfies all constraints
    #[must_use]
    pub fn matches(&self, version: &Version) -> bool {
        self.constraints.iter().all(|c| c.matches(version))
    }

    /// Get the constraints
    #[must_use]
    pub fn constraints(&self) -> &[VersionConstraint] {
        &self.constraints
    }

    /// Check if this spec has any constraints
    #[must_use]
    pub fn is_any(&self) -> bool {
        self.constraints.is_empty()
    }
}

impl FromStr for VersionSpec {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.is_empty() || s == "*" {
            // No constraints means any version
            return Ok(Self {
                constraints: vec![],
            });
        }

        // Split by comma and parse each constraint
        let constraints: Result<Vec<_>, _> = s
            .split(',')
            .map(|part| VersionConstraint::parse(part.trim()))
            .collect();

        let constraints = constraints?;

        if constraints.is_empty() {
            return Err(VersionError::InvalidConstraint {
                input: s.to_string(),
            });
        }

        Ok(Self { constraints })
    }
}

impl fmt::Display for VersionSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.constraints.is_empty() {
            write!(f, "*")
        } else {
            let strs: Vec<_> = self.constraints.iter().map(ToString::to_string).collect();
            write!(f, "{}", strs.join(","))
        }
    }
}
