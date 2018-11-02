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

//! # cargo-wix Binary and Subcommand
//!
//! The goal of the cargo-wix project and the `cargo wix` subcommand is to make
//! it easy to create a Windows installer (msi) for any Rust project. The
//! project is primarily implemented as a cargo subcommand, but the core
//! functionality is provided in a library (crate). See the module-level
//! comments for the [`lib.rs`] file for more information about usage and
//! organization of the `cargo-wix` crate. The remainder of this documentation
//! focuses on the usage and features of the `cargo wix` subcommand.
//!
//! ## Quick Start
//!
//! Ensure the [WiX Toolset] is installed and a `WIX` system environment
//! variable has been created. The installer for the WiX Toolset should create
//! the `WIX` system environment variable automatically. Then, start or restart
//! a command prompt (cmd.exe) and execute the following commands:
//!
//! ```dos
//! C:\>cargo install cargo-wix
//! C:\>cd Path\To\Project
//! C:\Path\To\Project\>cargo wix init
//! C:\Path\To\Project\>cargo wix
//! ```
//!
//! The Windows installer (msi) will be in the `C:\Path\To\Project\target\wix`
//! folder. The `cargo wix init` command will create a `wix` folder alongside
//! the `src` folder and the project's manifest (Cargo.toml). The `wix` folder
//! will contain the WiX Source file (`main.wxs`) that was automatically
//! generated for the project based the contents of its manifest. The WiX Source
//! file can be customized using an text editor, and once the file exists, the
//! `cargo wix init` command does not need to be used again.
//!
//! The `cargo wix` command uses the `wix\main.wxs` file generated from the
//! previous `cargo wix init` command as input for the WiX Toolset's "compiler"
//! and "linker", i.e. `candle.exe` and `light.exe`, respectively, to create the
//! Windows installer (msi). A variety of artifact files will be created in the
//! `target\wix` folder. These can be ignored and/or deleted.
//!
//! The installer that is created will install the executable file in a `bin`
//! folder within the destination selected by the user during installation. It
//! will add a license file to the same folder as the `bin` folder, and it will
//! add the `bin` folder to the `PATH` system environment variable so that the
//! executable can be called from anywhere with a commmand prompt. Most of these
//! behaviors can be adjusted during the installation process of the Windows
//! installer. The default destination is `C:\Program Files\<project name>`,
//! where `<project name>` is replaced with the name of the Rust project's name.
//!
//! ## Features
//!
//! The cargo-wix binary, and related `cargo wix` subcommand, use the WiX
//! Toolset and the [SignTool] application available in the [Windows 10 SDK].
//! These are obviously Windows-only applications, so while the crate and binary
//! can be built on any platform supported by the [Rust] programming language
//! and [Cargo], the `cargo wix` subcommand is only really useful in a Windows
//! environment.
//!
//! The WiX Toolset provides a "compiler" (`candle.exe`) and "linker"
//! (`light.exe`). These can be found in the `bin` directory of the installation
//! location for the WiX Toolset. The value of the `WIX` system environment
//! variable that is created during installation of the WiX Toolset is a path to
//! the installation folder that contains the `bin` folder. The `WIX` system
//! environment variable is used by the `cargo wix` subcommand and library with
//! the [`std::process::Command`] module to create the installer. The
//! `-B,--bin-path` option can be used to specify a path (relative or absolute)
//! to the WiX Toolset `bin` folder. The `-B,--bin-path` option is useful if a
//! different version of the WiX Toolset needs to be used to create the
//! installer. The descending order of precedence is: (1) `-B,--bin-path` option
//! then (2) `WIX` system environment variable. An error will be displayed if
//! the compiler and/or linker cannot be found.
//!
//! The Windows SDK provides a signer (`signtool`) application for signing
//! installers. The application is installed in the `bin` folder of the Windows
//! SDK installation. The location of the `bin` folder varies depending on the
//! version. It is recommended to use the Developer Prompt to ensure the
//! `signtool` application is available; however, it is possible to specify the
//! path to the Windows SDK `bin` folder using the `-B,--bin-path` option for
//! the `cargo wix sign` subcommand. The descending order of precedence for
//! locating the signer application is: (1) `-B,--bin-path` option then (2) the
//! order used by the [`std::process::Command::status`] method. Signing an
//! installer is optional.
//!
//! The WiX Toolset requires a WiX Source (wxs) file, which is an XML file. A
//! template is provided with this subcommand that attempts to meet the majority
//! of use cases for developers, so extensive knowledge of the WiX Toolset and
//! Windows installer technologies is not required (but always recommended).
//! Modification of the template is encouraged, but please consult the WiX
//! Toolset's extensive [documentation] and [tutorials] for information about
//! writing, customizing, and using wxs files. The documentation here is only
//! for this subcommand.
//!
//! The [WXS] template is embedded in the binary installation of the subcommand
//! and it can be printed to stdout using the `cargo wix print wxs` command from
//! the command prompt (cmd.exe). Note, each time the `cargo wix print wxs`
//! command is invoked, new GUIDs are generated for fields that require them.
//! Thus, a developer does not need to worry about generating GUIDs and can
//! begin using the template immediately with this subcommand or the WiX
//! Toolset's compiler (`candle.exe`) and linker (`light.exe`) applications.
//!
//! In addition to the WXS template, there are several license templates which
//! are used to generate an End User License Agreement (EULA) during the `cargo
//! wix init` command. Depending on the license ID(s) in the `license` field for
//! a package's manifest (Cargo.toml), a license file in the Rich Text Format
//! (RTF) is generated from a template and placed in the `wix` folder. This RTF
//! file is then displayed in the license dialog of the installer. See the help
//! information on the `print` command for information about supported licenses.
//! If the `license` field is not used, or the license ID is not supported, then
//! the EULA is _not_ automatically created during initialization and it will
//! have to be created manually with a text editor or other authoring tool.
//!
//! The `cargo wix init` subcommand uses a combination of the [`license`] and
//! [`license-file`] fields of the project's manifest (Cargo.toml) to determine
//! if a [sidecar] license file should be included in the installation folder
//! along side the `bin` folder. The `license` field appears to be the more
//! commonly used field to describe the licensing for a Rust project and
//! package, while the `license-file` field is used to specify a custom, or
//! properitary, license for the project and package. The top three most common
//! licenses for Rust projects are supported from the `license` field, i.e. MIT,
//! Apache-2.0, and GPLv3. If any of these three supported open source licenses
//! are used for the `license` field, then a `License.rtf` file is generated
//! from an embedded template in the `wix` folder as part of the `cargo wix
//! init` subcommand. This generated RTF file will be used as a sidecar file and
//! the End User License Agreement (EULA) that is displayed in the license
//! dialog of the installer. If the `license-file` field is used and it contains
//! a path to a file with the `.rtf` extension, then this file will be used as a
//! sidecar file and the End User License Agreement (EULA). If neither of these
//! fields exist or contain valid values, then no sidecar file is included in
//! the installation and no license dialog appears during installation. This
//! default behavior can be overridden with the `-l,--license` and `-E,--eula`
//! options for the `cargo wix init` subcommand.
//!
//! Generally, any value that is obtained from the package's manifest
//! (Cargo.toml) can be overridden at the command line with an appropriate
//! option. For example, the manufacturer, which is displayed as the "Publisher"
//! in the Add/Remove Programs (ARP) control panel is obtained from the first
//! author listed in the `authors` field of a project's manifest, but it can be
//! overridden using the `-m,--manufacturer` option with the `cargo wix init`
//! subcommand. The default in most cases is to use a value from a field in the
//! project's manifest.
//!
//! The `cargo wix` subcommand uses the package name for the product name. The
//! default install location is at `C:\Program Files\<Product Name>`, where
//! `<Product Name>` is replaced with the product name determined from the
//! package name. This can be overridden with the `-N,--name` option for the
//! `cargo wix` subcommand or the `-p,--product-name` option for the `cargo wix
//! init` subcommand. The binary name, which is the `name` field for the
//! `[[bin]]` section, is used for the executable file name, i.e. "name.exe".
//! This can also be overridden using the `-b,--bin-name` option for the `cargo
//! wix init` subcommand. The package description is used in multiple places for
//! the installer, including the text that appears in the blue UAC dialog when
//! using a signed installer. This can be overridden using the
//! `-d,--description` option with the `cargo wix init` or `cargo wix sign`
//! subcommands, respectively.
//!
//! [`lib.rs`]: https://volks73.github.io/cargo-wix/cargo_wix/index.html
//! [WiX Toolset]: http://wixtoolset.org
//! [SignTool]: https://msdn.microsoft.com/en-us/library/windows/desktop/aa387764(v=vs.85).aspx
//! [Windows 10 SDK]: https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk
//! [Rust]: https://www.rust-lang.org
//! [Cargo]: https://crates.io
//! [`std::process::Command`]: https://doc.rust-lang.org/std/process/struct.Command.html
//! [`std::process::Command::status`]: https://doc.rust-lang.org/std/process/struct.Command.html#method.status
//! [documentation]: http://wixtoolset.org/documentation/
//! [tutorials]: https://www.firegiant.com/wix/tutorial/
//! [WXS]: https://github.com/volks73/cargo-wix/blob/master/src/main.wxs.mustache
//! [`license`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [`license-file`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [sidecar]: https://en.wikipedia.org/wiki/Sidecar_file

extern crate cargo_wix;
#[macro_use] extern crate clap;
extern crate env_logger;
extern crate log;
extern crate termcolor;

use clap::{App, Arg, SubCommand};
use env_logger::Builder;
use env_logger::fmt::Color as LogColor;
use log::{Level, LevelFilter};
use std::error::Error;
use std::io::Write;
use cargo_wix::{BINARY_FOLDER_NAME, Cultures, Template, WIX_PATH_KEY};
use cargo_wix::clean;
use cargo_wix::create;
use cargo_wix::initialize;
use cargo_wix::print;
use cargo_wix::purge;
use cargo_wix::sign;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const SUBCOMMAND_NAME: &str = "wix";

fn main() {
    // The banner option for the `init` and `print` subcommands.
    let banner = Arg::with_name("banner")
        .help("Sets the path to a bitmap (.bmp) image file that \
               will be displayed across the top of each dialog in the \
               installer. The banner image dimensions should be \
               493 x 58 pixels.")
        .long("banner")
        .short("b")
        .takes_value(true);
    // The binary option for the `init` and `print` subcommands.
    let binary = Arg::with_name("binary")
        .help("Overrides the default binary included in the installer. The \
              default binary is the 'target\\release\\<package name>.exe' file, where \
              <package-name> is the value from the 'name' field in the '[package]' \
              section of the package's manifest (Cargo.toml).")
        .long("binary")
        .short("B")
        .takes_value(true);
    // The description option for the `init` and `print` subcommands.
    let description = Arg::with_name("description")
        .help("Overrides the 'description' field of the package's manifest (Cargo.toml) \
              as the description within the installer.")
        .long("description")
        .short("d")
        .takes_value(true);
    // The dialog option for the `init` and `print` subcommands.
    let dialog = Arg::with_name("dialog")
        .help("Sets the path to a bitmap (.bmp) image file that \
               will be displayed to the left on the first dialog of \
               the installer. The dialog image dimensions should \
               be 493 x 312 pxiels.")
        .long("dialog")
        .short("D")
        .takes_value(true);
    // The eula option for the `init` and `print` subcommands.
    let eula = Arg::with_name("eula")
        .help("Specifies a RTF file to use as the EULA for the license agreement dialog of the \
              installer. The default is to disable the license agreement dialog unless one of the \
              supported licenses (GPL-3.0, Apache-2.0, or MIT) is generated based on the value of \
              the 'license' field in the package's manifest (Cargo.toml). An EULA can be enabled \
              later by directly modifying the WiX Source (wxs) file with a text editor.")
        .long("eula")
        .short("e")
        .takes_value(true);
    // The license option for the `init` and `print` subcommands.
    let license = Arg::with_name("license")
        .help("Overrides the 'license-file' field of the package's manifest (Cargo.toml). If an \
              appropriate license file does not exist, cannot be found, or is not specified, then \
              no license file is included in the installer. A file containing the license, such as \
              a TXT, PDF, or RTF file, can be added later by directly editing the generated WiX \
              Source file (wxs) in a text editor.")
        .long("license")
        .short("l")
        .takes_value(true);
    // The url option for the `init` and `print` subcommands
    let url = Arg::with_name("url")
        .help("Adds a URL to the installer that will be displayed in the Add/Remove \
              Programs control panel for the application. The default is to disable \
              it unless a URL is specified for either the 'homepage', \
              'documentation', or 'repository' fields in the package's manifest \
              (Cargo.toml). The help URL can be enabled after initialization by \
              directly modifying the WiX Source (wxs) file with a text editor.")
        .long("url")
        .short("u")
        .takes_value(true);
    // The holder option for the `init` and `print` subcommands
    let holder = Arg::with_name("holder")
        .help("Sets the copyright holder for the license during initialization. The \
              default is to use the first author from the package's manifest \
              (Cargo.toml). This is only used when generate a license based on the \
              value of the 'license' field in the package's manifest.")
        .long("holder")
        .short("h")
        .takes_value(true);
    // The manufacturer option for the `init` and `print` subcommands
    let manufacturer = Arg::with_name("manufacturer")
        .help("Overrides the first author in the 'authors' field of the package's \
              manifest (Cargo.toml) as the manufacturer within the installer. The \
              manufacturer can be changed after initialization by directly \
              modifying the WiX Source file (wxs) with a text editor.")
        .long("manufacturer")
        .short("m")
        .takes_value(true);
    // The product icon option for the `init` and `print` subcommands
    let product_icon = Arg::with_name("product-icon")
        .help("Sets the path to an image file that will be displayed as an icon \
              in the Add/Remove Programs (ARP) control panel for the installed \
              application.")
        .long("product-icon")
        .short("p")
        .takes_value(true);
    // The product name option for the `init`, `print`, and `sign` subcommands
    let product_name = Arg::with_name("product-name")
        .help("Overrides the 'name' field of the package's manifest (Cargo.toml) as \
              the product name within the installer. The product name can be \
              changed after initialization by directly modifying the WiX Source \
              file (wxs) with a text editor.")
        .long("product-name")
        .short("P")
        .takes_value(true);
    // The "global" verbose flag for all subcommands.
    let verbose = Arg::with_name("verbose")
        .help("Sets the level of verbosity. The higher the level of verbosity, the more \
              information that is printed and logged when the application is executed. \
              This flag can be specified multiple times, where each occurrance \
              increases the level and/or details written for each statement.")
        .long("verbose")
        .short("v")
        .multiple(true);
    let year = Arg::with_name("year")
        .help("Sets the copyright year for the license during initialization. The \
              default is to use the current year. This is only used if a license \
              is generated from one of the supported licenses based on the value \
              of the 'license' field in the package's manifest (Cargo.toml).")
        .long("year")
        .short("y")
        .takes_value(true);
    let default_culture = Cultures::EnUs.to_string();
    let matches = App::new(crate_name!())
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_NAME)
                .version(crate_version!())
                .about(crate_description!())
                .arg(Arg::with_name("bin-path")
                    .help(&format!(
                        "Specifies the path to the WiX Toolset's '{0}' folder, which should contain \
                        the needed 'candle.exe' and 'light.exe' applications. The default is to use \
                        the path specified with the {1} system environment variable that is created \
                        during the installation of the WiX Toolset. Failing the existence of the \
                        {1} system environment variable, the path specified in the PATH system \
                        environment variable is used. This is useful when working with multiple \
                        versions of the WiX Toolset.",
                        BINARY_FOLDER_NAME,
                        WIX_PATH_KEY
                    ))
                    .long("bin-path")
                    .short("B")
                    .takes_value(true))
                .subcommand(SubCommand::with_name("clean")
                    .version(crate_version!())
                    .about("Deletes the 'target\\wix' folder if it exists.")
                    .arg(Arg::with_name("INPUT")
                        .help("A package's manifest (Cargo.toml). The 'target\\wix' folder that \
                              exists alongside the package's manifest will be removed. This is \
                              optional and the default is to use the current working directory (cwd).")
                        .index(1)))
                .arg(Arg::with_name("culture")
                    .help("Sets the culture for localization. Use with the '-L,--locale' option. \
                          See the WixUI localization documentation for more information about \
                          acceptable culture codes. The codes are case insenstive.")
                    .long("culture")
                    .short("C")
                    .default_value(&default_culture)
                    .takes_value(true))
                .subcommand(SubCommand::with_name("init")
                    .version(crate_version!())
                    .about("Uses a package's manifest (Cargo.toml) to generate a WiX Source (wxs) \
                           file that can be used immediately without modification to create an \
                           installer for the package. This will also generate an EULA in the Rich \
                           Text Format (RTF) if the 'license' field is specified with a supported \
                           license (GPL-3.0, Apache-2.0, or MIT). All generated files are placed in \
                           the 'wix' sub-folder by default.")
                    .arg(banner.clone())
                    .arg(binary.clone())
                    .arg(description.clone())
                    .arg(dialog.clone())
                    .arg(eula.clone())
                    .arg(Arg::with_name("force")
                        .help("Overwrites any existing files that are generated during \
                              initialization. Use with caution.")
                        .long("force"))
                    .arg(holder.clone())
                    .arg(Arg::with_name("INPUT")
                        .help("A package's manifest (Cargo.toml). If the '-o,--output' option is \
                              not used, then all output from initialization will be placed in \
                              a 'wix' folder created alongside this path.")
                        .index(1))
                    .arg(license.clone())
                    .arg(manufacturer.clone())
                    .arg(Arg::with_name("output")
                        .help("Sets the destination for all files generated during initialization. \
                              The default is to create a 'wix' folder within the project then \
                              generate all files in the 'wix' sub-folder.")
                        .long("output")
                        .short("o")
                        .takes_value(true))
                    .arg(product_icon.clone())
                    .arg(product_name.clone())
                    .arg(url.clone())
                    .arg(verbose.clone())
                    .arg(year.clone()))
                .arg(Arg::with_name("install-version")
                    .help("Overrides the version from the package's manifest (Cargo.toml), which \
                          is used for the installer name and appears in the Add/Remove Programs \
                          control panel.")
                    .long("install-version")
                    .short("I")
                    .takes_value(true))
                .arg(Arg::with_name("locale")
                    .help("Sets the path to a WiX localization file, '.wxl', which contains \
                          localized strings.")
                    .long("locale")
                    .short("L")
                    .takes_value(true))
                .arg(Arg::with_name("name")
                    .help("Overrides the package's 'name' field in the manifest (Cargo.toml), which \
                          is used in the name for the installer. This does not change the name of \
                          the executable within the installer. The name of the executable can be \
                          changed by modifying the WiX Source (wxs) file with a text editor.")
                    .long("name")
                    .short("N")
                    .takes_value(true))
                .arg(Arg::with_name("no-build")
                    .help("Skips building the release binary. The installer is created, but the \
                          'cargo build --release' is not executed.")
                    .long("no-build"))
                .arg(Arg::with_name("no-capture")
                    .help("By default, this subcommand captures, or hides, all output from the \
                          builder, compiler, linker, and signer for the binary and Windows \
                          installer, respectively. Use this flag to show the output.")
                    .long("nocapture"))
                .arg(Arg::with_name("output")
                    .help("Sets the destination file name and path for the created installer. The \
                          default is to create an installer with the \
                          '<product-name>-<version>-<arch>.msi' file name in the 'target\\wix' \
                          folder.")
                    .long("output")
                    .short("o")
                    .takes_value(true))
                .subcommand(SubCommand::with_name("print")
                    .version(crate_version!())
                    .about("Prints a template to stdout or a file. In the case of a license \
                           template, the output is in the Rich Text Format (RTF) and for a WiX \
                           Source file (wxs), the output is in XML. New GUIDs are generated for the \
                           'UpgradeCode' and Path Component each time the 'WXS' template is \
                           printed. [values: Apache-2.0, GPL-3.0, MIT, WXS]")
                    .arg(banner)
                    .arg(binary)
                    .arg(description)
                    .arg(dialog)
                    .arg(eula)
                    .arg(holder)
                    .arg(Arg::with_name("INPUT")
                        .help("A package's manifest (Cargo.toml). The selected template will be \
                              printed to stdout or a file based on the field values in this \
                              manifest. The default is to use the manifest in the current working \
                              directory (cwd). An error occurs if a manifest is not found.")
                        .index(2))
                    .arg(license)
                    .arg(manufacturer)
                    .arg(Arg::with_name("output")
                        .help("Sets the destination for printing the template. The default is to \
                              print/write the rendered template to stdout. If the destination, \
                              a.k.a. file, does not exist, it will be created.")
                        .long("output")
                        .short("o")
                         .takes_value(true))
                    .arg(product_icon)
                    .arg(product_name.clone())
                    .arg(Arg::with_name("TEMPLATE")
                        .help("The template to print. This is required and values are case \
                              insensitive. [values: Apache-2.0, GPL-3.0, MIT, WXS]")
                        .hide_possible_values(true)
                        .possible_values(&Template::possible_values()
                            .iter()
                            .map(|s| s.as_ref())
                            .collect::<Vec<&str>>())
                        .required(true)
                        .index(1))
                    .arg(url)
                    .arg(year)
                    .arg(verbose.clone()))
                .subcommand(SubCommand::with_name("purge")
                    .version(crate_version!())
                    .about("Deletes the 'target\\wix' and 'wix' folders if they exist. Use with \
                            caution!")
                    .arg(Arg::with_name("INPUT")
                        .help("A package's manifest (Cargo.toml). The 'target\\wix' and 'wix' \
                              folders that exists alongside the package's manifest will be removed. \
                              This is optional and the default is to use the current working \
                              directory (cwd).")
                        .index(1)))
                .subcommand(SubCommand::with_name("sign")
                    .version(crate_version!())
                    .about("The Windows installer (msi) will be signed using the SignTool \
                          application available in the Windows 10 SDK. The signtool is invoked with \
                          the '/a' flag to automatically obtain an appropriate certificate from the \
                          Windows certificate manager. The default is to also use the Comodo \
                          timestamp server with the '/t' flag.")
                    .arg(Arg::with_name("bin-path")
                        .help("Specifies the path to the folder containg the 'signtool' application. \
                              The default is to use the PATH system environment variable to locate the \
                              application.")
                        .long("bin-path")
                        .short("B")
                        .takes_value(true))
                    .arg(Arg::with_name("description")
                         .help("The information for the extended text of the ACL dialog that \
                               appears. This will be appended to the product name and delimited by \
                               a dash, '-'. The default is to use the description from the \
                               package's manifest (Cargo.toml). This option will override the \
                               default.")
                         .long("description")
                         .short("s")
                         .takes_value(true))
                    .arg(Arg::with_name("homepage")
                         .help("The URL to the homepage for the product. This will be displayed in \
                               the ACL dialog.")
                         .long("homepage")
                         .short("U")
                         .takes_value(true))
                    .arg(Arg::with_name("INPUT")
                         .help("A package's manifest (Cargo.toml). The installer located in the \
                               'target\\wix' folder alongside this manifest will be signed based on \
                               the metadata within the manifest.")
                         .index(1))
                    .arg(Arg::with_name("no-capture")
                         .help("By default, this subcommand captures, or hides, all output from the \
                               signer. Use this flag to show the output.")
                         .long("nocapture"))
                    .arg(product_name)
                    .arg(Arg::with_name("timestamp")
                        .help("The alias or URL for the timestamp server used with the 'signtool' to \
                              sign the installer. This can only be used with the '-s,--sign' flag. \
                              Either an alias can be used or a URL. Available case-insensitive aliases \
                              include: Comodo and Verisign.")
                        .short("t")
                        .long("timestamp")
                        .takes_value(true))
                    .arg(verbose.clone()))
                .arg(verbose)
        ).get_matches();
    let matches = matches.subcommand_matches(SUBCOMMAND_NAME).unwrap();
    let verbosity = match matches.subcommand() {
        ("clean", Some(m)) => m,
        ("init", Some(m)) => m,
        ("print", Some(m)) => m,
        ("purge", Some(m)) => m,
        _ => matches,
    }.occurrences_of("verbose");
    // Using the `Builder::new` instead of the `Builder::from_env` or `Builder::from_default_env`
    // skips reading the configuration from any environment variable, i.e. `RUST_LOG`. The log
    // level is later configured with the verbosity using the `filter` method. There are many
    // questions related to implementing  support for environment variables:
    //
    // 1. What should the environment variable be called, WIX_LOG, CARGO_WIX_LOG, CARGO_LOG, etc.?
    //    WIX_LOG might conflict with a system variable that is used for the WiX Toolset. CARGO_LOG
    //    is too generic. The only viable one is CARGO_WIX_LOG.
    // 2. How is the environment variable supposed to work with the verbosity without crazy side
    //    effects? What if the level is set to TRACE with the environment variable, but the
    //    verbosity is only one?
    // 3. Should the RUST_LOG environment variable be "obeyed" for a cargo subcommand?
    //
    // For now, implementing environment variable support is held off and only the verbosity is
    // used to set the log level.
    let mut builder = Builder::new();
    builder.format(|buf, record| {
        // This implmentation for a format is copied from the default format implemented for the
        // `env_logger` crate but modified to use a colon, `:`, to separate the level from the
        // message and change the colors to match the previous colors used by the `loggerv` crate.
        let mut level_style = buf.style();
        let level = record.level();
        match level {
            // Light Gray, or just Gray, is not a supported color for non-ANSI enabled Windows
            // consoles, so TRACE and DEBUG statements are differentiated by boldness but use the
            // same white color.
            Level::Trace => level_style.set_color(LogColor::White).set_bold(false),
            Level::Debug => level_style.set_color(LogColor::White).set_bold(true),
            Level::Info => level_style.set_color(LogColor::Green).set_bold(true),
            Level::Warn => level_style.set_color(LogColor::Yellow).set_bold(true),
            Level::Error => level_style.set_color(LogColor::Red).set_bold(true),
        };
        let write_level = write!(buf, "{:>5}: ", level_style.value(level));
        let write_args = writeln!(buf, "{}", record.args());
        write_level.and(write_args)
    }).filter(Some("cargo_wix"), match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }).init();
    let result = match matches.subcommand() {
        ("clean", Some(m)) => {
            let mut clean = clean::Builder::new();
            clean.input(m.value_of("INPUT"));
            clean.build().run()
        },
        ("init", Some(m)) => {
            let mut init = initialize::Builder::new();
            init.banner(m.value_of("banner"));
            init.binary(m.value_of("binary"));
            init.copyright_holder(m.value_of("holder"));
            init.copyright_year(m.value_of("year"));
            init.description(m.value_of("description"));
            init.dialog(m.value_of("dialog"));
            init.eula(m.value_of("eula"));
            init.force(m.is_present("force"));
            init.help_url(m.value_of("url"));
            init.input(m.value_of("INPUT"));
            init.license(m.value_of("license"));
            init.manufacturer(m.value_of("manufacturer"));
            init.output(m.value_of("output"));
            init.product_icon(m.value_of("product-icon"));
            init.product_name(m.value_of("product-name"));
            init.build().run()
        },
        ("print", Some(m)) => {
            let template = value_t!(m, "TEMPLATE", Template).unwrap();
            match template {
                Template::Wxs => {
                    let mut print = print::wxs::Builder::new();
                    print.banner(m.value_of("banner"));
                    print.binary(m.value_of("binary"));
                    print.description(m.value_of("description"));
                    print.dialog(m.value_of("dialog"));
                    print.eula(m.value_of("eula"));
                    print.help_url(m.value_of("url"));
                    print.input(m.value_of("INPUT"));
                    print.license(m.value_of("license"));
                    print.manufacturer(m.value_of("manufacturer"));
                    print.output(m.value_of("output"));
                    print.product_icon(m.value_of("product-icon"));
                    print.product_name(m.value_of("product-name"));
                    print.build().run()
                },
                t => {
                    let mut print = print::license::Builder::new();
                    print.copyright_holder(m.value_of("holder"));
                    print.copyright_year(m.value_of("year"));
                    print.input(m.value_of("INPUT"));
                    print.output(m.value_of("output"));
                    print.build().run(t)
                },
            }
        },
        ("purge", Some(m)) => {
            let mut purge = purge::Builder::new();
            purge.input(m.value_of("INPUT"));
            purge.build().run()
        },
        ("sign", Some(m)) => {
            let mut sign = sign::Builder::new();
            sign.bin_path(m.value_of("bin-path"));
            sign.capture_output(!m.is_present("no-capture"));
            sign.description(m.value_of("description"));
            sign.homepage(m.value_of("homepage"));
            sign.input(m.value_of("INPUT"));
            sign.product_name(m.value_of("product-name"));
            sign.timestamp(m.value_of("timestamp"));
            sign.build().run()
        },
        _ => {
            let mut create = create::Builder::new();
            create.bin_path(matches.value_of("bin-path"));
            create.capture_output(!matches.is_present("no-capture"));
            create.culture(value_t!(matches, "culture", Cultures).unwrap_or_else(|e| e.exit()));
            create.input(matches.value_of("INPUT"));
            create.locale(matches.value_of("locale"));
            create.name(matches.value_of("name"));
            create.no_build(matches.is_present("no-build"));
            create.output(matches.value_of("output"));
            create.version(matches.value_of("install-version"));
            create.build().run()
        }
    };
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            {
                let mut stderr = StandardStream::stderr(ColorChoice::Auto);
                stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).expect("Coloring stderr");
                write!(&mut stderr, "Error[{}] ({}): ", e.code(), e.description()).expect("Write tag to stderr");
                // This prevents "leaking" the color settings to the console after the
                // sub-command/application has completed and ensures the message is not printed in
                // Red.
                //
                // See:
                //
                // - [Issue #47](https://github.com/volks73/cargo-wix/issues/47)
                // - [Issue #48](https://github.com/volks73/cargo-wix/issues/48).
                stderr.reset().expect("Revert color settings after printing the tag");
                stderr.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false)).expect("Coloring stderr");
                writeln!(&mut stderr, "{}", e).expect("Write message to stderr");
                // This prevents "leaking" the color settings to the console after the
                // sub-command/application has completed.
                //
                // See:
                //
                // - [Issue #47](https://github.com/volks73/cargo-wix/issues/47)
                // - [Issue #48](https://github.com/volks73/cargo-wix/issues/48).
                stderr.reset().expect("Revert color settings after printing the message");
            }
            std::process::exit(e.code());
        }
    }
}

