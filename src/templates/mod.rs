// Copyright (C) 2017 Christopher R. Field.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::Error;
use lazy_static::lazy_static;
use std::fmt;
use std::str::FromStr;

/// The WiX Source (wxs) template.
static WIX_SOURCE_TEMPLATE: &str = include_str!("main.wxs.mustache");

/// The V4 Schema WiX Source (wxs) template.
/// 
/// Note: Used by both Wix4 and Wix5 toolsets
static WIX_V4_SOURCE_TEMPLATE: &str = include_str!("v4/main.wxs.mustache");

/// The Apache-2.0 Rich Text Format (RTF) license template.
static APACHE2_LICENSE_TEMPLATE: &str = include_str!("Apache-2.0.rtf.mustache");

/// The GPL-3.0 Rich Text Format (RTF) license template.
static GPL3_LICENSE_TEMPLATE: &str = include_str!("GPL-3.0.rtf.mustache");

/// The MIT Rich Text Format (RTF) license template.
static MIT_LICENSE_TEMPLATE: &str = include_str!("MIT.rtf.mustache");

/// The different templates that can be printed or written to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    /// The [Apache-2.0] license.
    ///
    /// [Apache-2.0]: https://opensource.org/licenses/Apache-2.0
    Apache2,
    /// The [GPL-3.0] license.
    ///
    /// [GPL-3.0]: https://opensource.org/licenses/gpl-3.0.html
    Gpl3,
    /// The [MIT] license.
    ///
    /// [MIT]: https://opensource.org/licenses/MIT
    Mit,
    /// A [WiX Source (wxs)] file.
    ///
    /// [Wix Source (wxs)]: http://wixtoolset.org/documentation/manual/v3/overview/files.html
    Wxs,
    /// A [Modern Wix Source (wxs)] file.
    /// 
    /// [Modern Wix Source (wxs)]: https://wixtoolset.org/docs/schema/wxs/
    WxsV4,
}

lazy_static! {
    static ref POSSIBLE_VALUES: Vec<String> = vec![
        Template::Apache2.id().to_owned(),
        Template::Apache2.id().to_lowercase(),
        Template::Gpl3.id().to_owned(),
        Template::Gpl3.id().to_lowercase(),
        Template::Mit.id().to_owned(),
        Template::Mit.id().to_lowercase(),
        Template::Wxs.id().to_owned(),
        Template::Wxs.id().to_lowercase(),
        Template::WxsV4.id().to_lowercase(),
        // Added for convenience instead of http://wixtoolset.org/schemas/v4/wxs
        String::from("wxs4"),
        String::from("WXS4")
    ];
}

impl Template {
    /// Gets the ID for the template.
    ///
    /// In the case of a license template, the ID is the [SPDX ID] which is also used for the
    /// `license` field in the package's manifest (Cargo.toml). This is also the same value used
    /// with the `cargo wix print` subcommand.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wix::Template;
    ///
    /// assert_eq!(Template::Apache2.id(), "Apache-2.0");
    /// assert_eq!(Template::Gpl3.id(), "GPL-3.0");
    /// assert_eq!(Template::Mit.id(), "MIT");
    /// assert_eq!(Template::Wxs.id(), "WXS");
    /// assert_eq!(Template::WxsModern.id(), crate::toolset::project::V4_NAMESPACE_URI);
    /// ```
    ///
    /// [SPDX ID]: https://spdx.org/licenses/
    pub fn id(&self) -> &str {
        match *self {
            Template::Apache2 => "Apache-2.0",
            Template::Gpl3 => "GPL-3.0",
            Template::Mit => "MIT",
            Template::Wxs => "WXS",
            Template::WxsV4 => crate::toolset::project::V4_NAMESPACE_URI
        }
    }

    /// Gets the possible string representations of each variant.
    ///
    /// The possibilities are combination of case (upper and lower) for the
    /// various templates that are available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wix::Template;
    ///
    /// assert_eq!(
    ///     Template::possible_values(),
    ///     &vec![
    ///         "Apache-2.0".to_owned(),
    ///         "apache-2.0".to_owned(),
    ///         "GPL-3.0".to_owned(),
    ///         "gpl-3.0".to_owned(),
    ///         "MIT".to_owned(),
    ///         "mit".to_owned(),
    ///         "WXS".to_owned(),
    ///         "wxs".to_owned()
    ///         crate::toolset::project::V4_NAMESPACE_URI.to_owned(),
    ///         "wxs4".to_owned(),
    ///         "WXS4".to_owned
    ///     ]
    /// );
    /// ```
    pub fn possible_values() -> &'static Vec<String> {
        &POSSIBLE_VALUES
    }

    /// Gets the IDs of all supported licenses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wix::Template;
    ///
    /// assert_eq!(
    ///     Template::license_ids(),
    ///     vec![
    ///         "Apache-2.0".to_owned(),
    ///         "GPL-3.0".to_owned(),
    ///         "MIT".to_owned(),
    ///     ]
    /// );
    /// ```
    pub fn license_ids() -> Vec<String> {
        vec![
            Template::Apache2.id().to_owned(),
            Template::Gpl3.id().to_owned(),
            Template::Mit.id().to_owned(),
        ]
    }

    /// Gets the embedded contents of the template as a string.
    pub fn to_str(&self) -> &str {
        match *self {
            Template::Apache2 => APACHE2_LICENSE_TEMPLATE,
            Template::Gpl3 => GPL3_LICENSE_TEMPLATE,
            Template::Mit => MIT_LICENSE_TEMPLATE,
            Template::Wxs => WIX_SOURCE_TEMPLATE,
            Template::WxsV4 => WIX_V4_SOURCE_TEMPLATE,
        }
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl FromStr for Template {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "apache-2.0" => Ok(Template::Apache2),
            "gpl-3.0" => Ok(Template::Gpl3),
            "mit" => Ok(Template::Mit),
            "wxs" => Ok(Template::Wxs),
            crate::toolset::project::V4_NAMESPACE_URI | "wxs4" | "WXS4" => Ok(Template::WxsV4),
            _ => Err(Error::Generic(format!(
                "Cannot convert from '{s}' to a Template variant"
            ))),
        }
    }
}
