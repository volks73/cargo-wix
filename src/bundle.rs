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

use crate::Result;

use encoding_rs_io::DecodeReaderBytes;
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use std::path::{Path, PathBuf};
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

#[derive(Debug, PartialEq)]
pub enum BuildType {
//    None,
    Unknown, // rmv
    Product,
    Bundle,
//    Both,
}

impl FromStr for BuildType {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self> {
        if value == "product" {
            Ok(BuildType::Product)
        }
        else if value == "bundle" {
            Ok(BuildType::Bundle)
        }
        else {
            // fix?  Return an Err instead?
            Ok(BuildType::Unknown)
        }
    }
}

pub fn get_build_type_for_one(wxiobj: &Path) -> Result<BuildType> {
    let file = File::open(wxiobj)?;
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
    BuildType::from_str(&value)
}

impl TryFrom<&PathBuf> for BuildType
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
        BuildType::from_str(&value)
    }
}

//struct Wrapper<T>(Vec<T>);

/*
impl TryInto<BuildType> for &Path {
    type Error = crate::Error;

    fn try_into(self) -> Result<BuildType> {
        Ok(BuildType::Unknown)
    }
}
*/

/*
impl<P> TryInto<BuildType> for P
    where P: AsRef<Path>
{
    type Error = crate::Error;

    fn try_into(self) -> Result<BuildType> {
        Ok(BuildType::Unknown)
    }
}
*/

/*
impl TryInto<P> for 
impl<P> TryFrom<P> for BuildType
    where P: AsRef<Path>
//impl TryFrom<&PathBuf> for BuildType
{
    type Error = crate::Error;

    fn try_from(_path: P) -> Result<Self> {
        Ok(BuildType::Unknown)
    }
}
*/

pub fn get_build_type(wxiobjs: &Vec<PathBuf>) -> Result<BuildType> {
    if wxiobjs.len() > 0 {

        let _installer_kind: Result<InstallerKind> = wxiobjs
            .iter()
            .map(|p| InstallerKind::try_from(p))
            .sum();
        let _installer_kind = _installer_kind?;

        let _temp = BuildType::try_from(&wxiobjs[0])?;
        let build_type = get_build_type_for_one(&wxiobjs[0])?;
        for ref rover in wxiobjs.iter().skip(1) {
            let current = get_build_type_for_one(rover)?;
            if current != build_type {
                return Ok(BuildType::Unknown);
            }
        }
        Ok(build_type)
    }
    else {
        Ok(BuildType::Unknown)
    }
}
