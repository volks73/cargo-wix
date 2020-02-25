// Copyright (C) 2020 Brian Cook (aka Coding Badly).
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

//! The implementation for is_bundle_build.  This function probes a compiler output file (wxiobj)
//! for indications of what is being built: product or bundle installer.

/*
use crate::Result;

use encoding_rs_io::DecodeReaderBytes;
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum InstallerKind {
    None,
    Product,
    Bundle,
    Both,
}

impl InstallerKind {
    pub fn is_bundle(&self) -> bool {
        match *self {
            Self::None => false,
            Self::Product => false,
            Self::Bundle => true,
            Self::Both => false,
        }
    }
}

impl FromStr for InstallerKind {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self> {
        if value == "product" {
            Ok(InstallerKind::Product)
        }
        else if value == "bundle" {
            Ok(InstallerKind::Bundle)
        }
        else {
            Err(Self::Err::Generic(format!("Unknown '{}' installer kind", value)))
        }
    }
}

impl Add<InstallerKind> for InstallerKind {
    type Output = InstallerKind;

    fn add(self, rhs: InstallerKind) -> InstallerKind {
        if self == rhs {
            self
        }
        else if self == InstallerKind::None {
            rhs
        }
        else if rhs == InstallerKind::None {
            self
        }
        else {
            InstallerKind::Both
        }
    }
}

impl AddAssign for InstallerKind {
    fn add_assign(&mut self, other: Self) {
        if *self == InstallerKind::None {
            *self = other;
        }
        else if *self != other {
            *self = InstallerKind::Both;
        }
        else {
            // The two already have to be equal but this covers all possibilities.
            *self = other;
        }
    }
}

impl Sum for InstallerKind {
    fn sum<I>(i: I) -> Self
        where I: Iterator<Item = Self>,
    {
        i.fold(InstallerKind::None, |a, v| a + v)
    }
}

impl TryFrom<&PathBuf> for InstallerKind
{
    type Error = crate::Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
		let file = File::open(path)?;
		let mut decoder = DecodeReaderBytes::new(file);
		let mut content = String::new();
		decoder.read_to_string(&mut content)?;
		let package = parser::parse(&content)?;
		let document = package.as_document();
		let mut context = Context::new();
		context.set_namespace("wix", "http://schemas.microsoft.com/wix/2006/objects");
		// The assumption is that the following cannot fail because the path is known to be valid at
		// compile-time.
		let xpath = Factory::new().build("/wix:wixObject/wix:section/@type").unwrap().unwrap();
		let value = xpath
			.evaluate(&context, document.root())?
			.string();
        InstallerKind::from_str(&value)
    }
}
*/
