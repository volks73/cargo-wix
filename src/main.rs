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

//! # `cargo-wix` Binary and Subcommand
//!
//! The goal of the cargo-wix project and the `cargo wix` subcommand is to make
//! it easy to create a Windows installer (msi) for any Rust project. The
//! project is primarily implemented as a [cargo subcommand], but the core
//! functionality is provided in a library (crate). See the module-level
//! comments for the [library] for more information about usage and
//! organization of the `wix` crate. The remainder of this documentation
//! focuses on the usage and features of the `cargo wix` subcommand.
//!
//! ## Table of Contents
//!
//! - [Quick Start](#quick-start)
//! - [C Runtime](#c-runtime)
//! - [Examples](#examples)
//! - [Features](#features)
//!   - [Signing](#signing)
//!   - [Templates](#templates)
//!   - [Variables](#variables)
//!   - [Extensions](#extensions)
//!   - [Multiple WiX Sources](#multiple-wix-sources)
//!   - [Bundles](#bundles)
//! - [Configuration](#configuration)
//! - [Flags and Options](#flags-and-options)
//!
//! ## Quick Start
//!
//! Ensure the [WiX Toolset] is installed and a `WIX` system environment
//! variable has been created. The installer for the WiX Toolset should create
//! the `WIX` system environment variable automatically. Then, start, or
//! restart, a command prompt (cmd.exe) and execute the following commands:
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
//! file can be customized using a text editor, and once the file exists, the
//! `cargo wix init` command does not need to be used again.
//!
//! The `cargo wix` command uses the `wix\main.wxs` file generated from the
//! previous `cargo wix init` command as input for the WiX Toolset's "compiler"
//! and "linker", i.e. `candle.exe` and `light.exe`, respectively, to create the
//! Windows installer (msi). A variety of artifact files will be created in the
//! `target\wix` folder. These can be ignored and/or deleted.
//!
//! The created installer will install the executable file in a `bin` folder
//! within the destination selected by the user during installation. It will add
//! a license file to the same folder as the `bin` folder, and it will add the
//! `bin` folder to the `PATH` system environment variable so that the
//! executable can be called from anywhere with a command prompt. Most of these
//! behaviors can be adjusted during the installation process. The default
//! installation destination is `C:\Program Files\<project name>`, where
//! `<project name>` is replaced with the name of the project.
//!
//! ## C Runtime
//!
//! If using the `x86_64-pc-windows-msvc` or the `i686-pc-windows-msvc`
//! toolchain, then an appropriate C RunTime (CRT) must be available on the host
//! system _or_ the CRT must be statically compiled with the Rust binary
//! (executable). By default, the CRT is dynamically linked to a Rust binary if
//! using Microsoft Visual C compiler (MSVC). This may cause issues and errors
//! when running the Rust binary immediately after installation on a clean
//! Windows installation, i.e. the CRT has not already been installed. If the
//! Rust compiler is installed and an `-msvc` toolchain is available, then the
//! Rust binary will execute after installation without an issue.
//!
//! **Note**, the Rust programming language does _not_ need to be installed
//! to run an executable built using Cargo and Rust. Only a Microsoft-provided CRT
//! needs to be installed. These are often referred to as "redistributables". Rust
//! with either of the `-msvc` toolchains will dynamically link against the
//! `vcruntime140.dll`, which is part of the Visual C++ 2015 redistributable and
//! freely provided by Microsoft.
//!
//! For developers using the `cargo wix` subcommand to create an installer, this
//! dependency on the presence of an appropriate C runtime can lead to a sub-optimal
//! user experience. There are three options: (i) statically link the C runtime
//! within the Rust binary (recommended), (ii) add the Visual C++ redistributable to
//! the installer via the [Merge module] method for the WiX Toolset, or (iii)
//! provide the user with instructions for downloading and installing the CRT
//! _before_ running the executable.
//!
//! The current recommended option is to [statically link the CRT] when building
//! the Rust binary. Rust v1.19 or newer is required, and the solution
//! ultimately becomes adding the `-C target-feature=+crt-static` option to the
//! invocation of the Rust compiler (rustc). There are a variety of methods for
//! adding the option to the invocation, including but not limited to: (i)
//! creating a Cargo configuration file for the user or project, i.e.
//! `.cargo/config.toml`, and adding the following:
//!
//! ```
//! [target.x86_64-pc-windows-msvc]
//! rustflags = ["-C", "target-feature=+crt-static"]
//!
//! [target.i686-pc-windows-msvc]
//! rustflags = ["-C", "target-feature=+crt-static"]
//! ```
//!
//! to the `config.toml` file or (ii) creating the `RUSTFLAGS` environment
//! variable and setting its value to `"-C target-feature=+crt-static"`. Please
//! see the Rust compiler documentation on static linking for more information
//! and details.
//!
//! ## Examples
//!
//! All of the following examples use the native Command Prompt (cmd.exe) for
//! the Windows OS; however, the [Developer Prompt] installed with the [VC Build
//! Tools] is recommended. A [git bash], [PowerShell], or [Alacritty] terminal can
//! also be used.
//!
//! Begin each example by starting the appropriate prompt and navigating to the
//! root folder of the Rust project. The `cargo wix` subcommand and binary
//! assumes the current working directory (cwd) is the project, a.k.a. package,
//! root folder, i.e. the same folder as the package's manifest (Cargo.toml).
//!
//! Let's create a basic project with Cargo and then an installer.
//!
//! ```dos
//! C:\Path\to\Project> cargo init --bin --vcs none --name example
//! ```
//!
//! This will create a simple binary package named "example" without any version
//! control. While the `cargo wix` subcommand and binary does work with
//! libraries (crates) in addition to binaries (applications), generally an
//! installer is needed for a binary (application) not a library. Rust libraries
//! are typically distributed via [crates.io] and do not need an installer. The
//! package's manifest should look like the following, but with your name and
//! email address for the `authors` field:
//!
//! ```toml
//! [package]
//! name = "example"
//! version = "0.1.0"
//! authors = ["First Last <first.last@example.com>"]
//!
//! [dependencies]
//! ```
//!
//! The next step is to create the WiX Source (wxs) file, which is needed to
//! create the installer for the project.
//!
//! ```dos
//! C:\Path\to\Project> cargo wix init
//!  WARN: A description was not specified at the command line or in the package's manifest (Cargo.toml). The description can be added manually to the generated WiX Source (wxs) file using a text editor.
//!  WARN: An EULA was not specified at the command line, a RTF license file was not specified in the package manifest's (Cargo.toml) 'license-file' field, or the license ID from the package manifest's 'license' field is not recognized. The license agreement dialog will be excluded from the installer. An EULA can be added manually to the generated WiX Source (wxs) file using a text editor.
//!  WARN: A help URL could not be found and it will be excluded from the installer. A help URL can be added manually to the generated WiX Source (wxs) file using a text editor.
//!  WARN: A license file could not be found and it will be excluded from the installer. A license file can be added manually to the generated WiX Source (wxs) file using a text editor.
//! ```
//!
//! The warnings can be ignored for the time being, and they will be addressed in
//! subsequent examples. The above command will create a `wix` folder with the
//! `main.wxs` file:
//!
//! ```dos
//! C:\Path\to\Project> dir /B
//! Cargo.toml
//! src
//! wix
//! C:\Path\to\Project> dir wix /B
//! main.wxs
//! ```
//!
//! The `wix` folder and the `main.wxs` file now exist, and an installer can be
//! created:
//!
//! ```dos
//! C:\Path\to\Project> cargo wix
//! ```
//!
//! This may take a moment to complete as the `cargo wix` subcommand will build
//! the application with the _Release_ target profile and then build the
//! installer. The installer will be located in the `target\wix` folder.
//!
//! ```dos
//! C:\Path\to\Project> dir target\wix /B
//! example-0.1.0-x86_64.msi
//! main.wixobj
//! ```
//!
//! Great! An installer (msi) exists for the application. The `main.wixobj` file
//! is an artifact of the installer build process and can be ignored and/or
//! deleted.
//!
//! You can also automatically run the installer after creating it by
//! specifying the `--install` argument:
//!
//! ```dos
//! C:\Path\to\Project> cargo wix --install
//!  ```
//!
//! The installer that is created with the above steps and commands will install
//! the `example.exe` file to: `C:\Program Files\example\bin\example.exe`. It
//! will also add the `C:\Program Files\example\bin` path to the `PATH` system
//! environment variable, unless this feature is disabled during the
//! installation process from the Custom Setup dialog. The default welcome
//! screen, banner, and icons from the WiX Toolset's default UI component will
//! be used. A license dialog will _not_ appear for this installer.
//!
//! The installer that is created from the previous example is relatively simple,
//! but so is the application and package. It would be nice to have a license
//! dialog appear in the installer that allows the end-user to review and
//! acknowledge the End User License Agreement (EULA). It would also be nice to
//! have this same license appear in the installation folder for the application
//! so end-users can review it after installation.
//!
//! The license agreement dialog for a WiX Toolset-created installer must be in
//! the [Rich Text Format] (.rtf). The majority of Rust binaries and libraries
//! are developed and distributed with an open source license. We will do the
//! same for the example binary here and use the GPL-3.0 license. Creating a RTF
//! version of the GPL-3.0 requires a third-party tool, such as [WordPad], which
//! is freely available with any Windows distribution, [Microsoft Office],
//! [LibreOffice], or any number of freely available text editors, word
//! processors, or document conversion tools. Luckily, the `cargo wix`
//! subcommand and binary has a RTF version of the GPL-3.0 license available and
//! eliminates the need to install, pay, and/or learn another tool to create the
//! EULA.
//!
//! First, open the package's manifest (Cargo.toml) in a text editor, like
//! [Microsoft Notepad], and add the [`license`] field to the `package`
//! section with the `GPL-3.0` value:
//!
//! ```toml
//! [package]
//! name = "example"
//! version = "0.1.0"
//! authors = ["First Last <first.last@example.com>"]
//! license = "GPL-3.0"
//!
//! [dependencies]
//! ```
//!
//! Save the package's manifest and exit the text editor. Now, we can create a
//! `wix\main.wxs` file that will include the license dialog with a GPL-3.0
//! EULA.
//!
//! ```dos
//! C:\Path\to\Project> cargo wix init --force
//!  WARN: A description was not specified at the command line or in the package's manifest (Cargo.toml). The description can be added manually to the generated WiX Source (wxs) file using a text editor.
//!  WARN: A help URL could not be found and it will be excluded from the installer. A help URL can be added manually to the generated WiX Source (wxs) file using a text editor.
//!
//! C:\Path\to\Project> dir wix /B
//! License.rtf
//! main.wxs
//! C:\Path\to\Project> cargo wix
//! ```
//!
//! The `--force` flag is needed so that the existing `wix\main.wxs` is
//! overwritten with the new EULA-enabled `wix\main.wxs` file. The majority of
//! the warnings have also been addressed. Notice there is now a `License.rtf`
//! file in the `wix` folder. This will be used as the EULA in the license
//! agreement dialog for the installer and be included in the installation
//! folder with the binary.
//!
//! A side note, while version control has been disabled for these examples, it is
//! best practice to include installer-related files in version control; thus,
//! the `wix\main.wxs` and `wix\License.rtf` should be added to version control.
//!
//! Let's fix the remaining warnings. Both of the remaining warnings can be
//! resolved in multiple ways. The first is to use the options available for the
//! `cargo wix init` subcommmand following the previous example project:
//!
//! ```dos
//! C:\Path\to\Project> cargo wix init --force -d "This is a description" -u http://www.example.com
//!
//! C:\Path\to\Project> dir wix /B
//! License.rtf
//! main.wxs
//! C:\Path\to\Project> cargo wix
//! ```
//!
//! The warnings for the description and help URL have disappeared. The
//! `-d,--description` option for the `cargo wix init` command adds a
//! description for the installer. Similarly, the `-u,--url` option adds a help
//! URL. The `--force` flag is still needed to overwrite the previous
//! `wix\main.wxs` file.
//!
//! Another possibility is to add the `description` and `homepage` fields to the
//! package's manifest (Cargo.toml), and then initialize and create the
//! installer. The cargo-wix binary and subcommand will use these fields, among
//! others, to automatically include the values into the installer. The
//! `-d,--description` and `-u,--url` options can still be used to override the
//! values from the package's manifest. This can be useful if the contents of
//! the installer might need to be different or more detailed than the package.
//!
//! Following from the previous example, open the package's manifest
//! (Cargo.toml) in a text editor, like [Microsoft Notepad], and add the
//! [`description`] and [`homepage`] fields to the `package` section:
//!
//! ```toml
//! [package]
//! name = "example"
//! version = "0.1.0"
//! authors = ["First Last <first.last@example.com>"]
//! license = "GPL-3.0"
//! description = "This is a description"
//! homepage = "http://www.example.com"
//!
//! [dependencies]
//! ```
//!
//! Save the package's manifest and exit the text editor. Now, we can create a
//! `wix\main.wxs` file without any warnings and uses the description and
//! homepage from the package's manifest:
//!
//! ```dos
//! C:\Path\to\Project> cargo wix init --force
//! C:\Path\to\Project> dir wix /B
//! License.rtf
//! main.wxs
//! C:\Path\to\Project> cargo wix
//! ```
//!
//! The [`documentation`] and [`repository`] fields can be used instead of the
//! [`homepage`] field for the help URL, too.
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
//! environment variable is used by the `cargo wix` subcommand with
//! the [`std::process::Command`] module to create installers.
//!
//! ### Signing
//!
//! The Windows SDK provides a signer (`signtool`) application for signing
//! installers. The application is installed in the `bin` folder of the Windows
//! SDK installation. The location of the `bin` folder varies depending on the
//! version. It is recommended to use the Developer Prompt to ensure the
//! `signtool` application is available. Signing an installer is optional.
//!
//! ### Templates
//!
//! The WiX Toolset requires a WiX Source (WXS) file, which is an [XML] file. A
//! template is provided with this binary that attempts to meet the majority
//! of use cases for developers and avoid requiring extensive knowledge of the
//! WiX Toolset and Windows installer technologies. Modification of the template
//! is encouraged, but please consult the WiX Toolset's extensive
//! [documentation] and [tutorials] for information about writing (authoring),
//! customizing, and using WXS files. This documentation here is only for this
//! binary and subcommand.
//!
//! The [WXS] template is embedded in the binary installation of the subcommand
//! and it can be printed to STDOUT using the `cargo wix print wxs` command from
//! the command prompt (cmd.exe). Note, each time the `cargo wix print wxs`
//! command is invoked, new Globally Unique Identifiers ([GUID]) are generated
//! for fields that require them. Thus, a developer does not need to worry about
//! generating GUIDs and can begin using the template immediately with this
//! subcommand or the WiX Toolset's compiler (`candle.exe`) and linker
//! (`light.exe`) applications.
//!
//! In addition to the WXS template, there are several license templates which
//! are used to generate an End User License Agreement (EULA) during the `cargo
//! wix init` command. Depending on the license ID(s) in the `license` field for
//! a package's manifest (Cargo.toml), a license file in the Rich Text Format
//! (RTF) is generated from a template and placed in the `wix` folder. This RTF
//! file is then displayed in the license agreement dialog of the installer. See
//! the help information on the `carge wix print` subcommand:
//!
//! ```dos
//! C:\Path\to\Project> cargo wix print --help
//! ```
//!
//! for information about supported licenses. If the `license` field is
//! not used, or the license ID is not supported, then the EULA is _not_
//! automatically created during initialization and it will have to be created
//! manually with a text editor or some other authoring tool.
//!
//! The `cargo wix init` subcommand uses a combination of the [`license`] and
//! [`license-file`] fields of the project's manifest (Cargo.toml) to determine
//! if a [sidecar] license file should be included in the installation folder
//! alongside the `bin` folder. The `license` field appears to be the more
//! commonly used field to describe the licensing for a Rust project and
//! package, while the `license-file` field is used to specify a custom, or
//! proprietary, license.
//!
//! The top three most common licenses for Rust projects are supported from the
//! `license` field, i.e. MIT, Apache-2.0, and GPLv3. If any of these three
//! supported open source licenses are used for the `license` field, then a
//! `License.rtf` file is generated from an embedded template and placed in the
//! `wix` folder as part of the `cargo wix init` subcommand. This generated RTF
//! file will be used as a sidecar file and for the End User License Agreement
//! (EULA) that is displayed in the license agreement dialog of the installer.
//! If the `license-file` field is used and it contains a path to a file with
//! the `.rtf` extension, then this file will be used as a sidecar file and for
//! the EULA. If neither of these fields exist or contain valid values, then no
//! sidecar file is included in the installation and no license agreement dialog
//! appears during installation. This default behavior can be overridden with
//! the `-l,--license` and `-e,--eula` options for the `cargo wix init`
//! subcommand.
//!
//! ### Variables
//!
//! The cargo-wix subcommand automatically passes some Cargo and build
//! configuration-related values to the WiX Toolset compiler (candle.exe). These
//! variables can be used within a WiX Source (WXS) file using the
//! `$(var.<VARIABLE>)` notation, where `<VARIABLE>` is replaced with the name
//! of variable passed from the cargo-wix subcommand to the WiX Toolset
//! compiler using the `-DKEY=VALUE` option. Notable usage of these variables
//! are in [WiX preprocessor] directives to dynamically change the installer
//! creation at build time. Below is a current list of variables passed from the
//! cargo-wix subcommand to a WXS file during installer creation.
//!
//! - `TargetTriple` = The rustc target triple name as seen with the `rustc
//!   --print target-list` command
//! - `TargetEnv` = The rustc target environment, as seen with the output from
//! the `rustc --print cfg` command as `target_env`. On Windows, this typically
//! either `msvc` or `gnu` depending on the toolchain downloaded and installed.
//! - `TargetVendor` = The rustc target vendor, as seen with the output from the
//! `rustc --print cfg` command as `target_vendor`. This is typically `pc`, but Rust
//! does support other vendors, like `uwp`.
//! - `CargoTargetBinDir` = The complete path to the binary (exe). The default
//! would be `target\release\<BINARY_NAME>.exe` where `<BINARY_NAME>` is replaced
//! with the name of each binary target defined in the package's manifest
//! (Cargo.toml). If a different rustc target triple is used than the host, i.e.
//! cross-compiling, then the default path would be
//! `target\<CARGO_TARGET>\<CARGO_PROFILE>\<BINARY_NAME>.exe`, where
//! `<CARGO_TARGET>` is replaced with the `CargoTarget` variable value and
//! `<CARGO_PROFILE>` is replaced with the value from the `CargoProfile` variable.
//! - `CargoTargetDir` = The path to the directory for the build artifacts, i.e.
//! `target`.
//! - `CargoProfile` = Either `debug` or `release` depending on the build
//! profile. The default is `release`.
//! - `Platform` = (Deprecated) Either `x86`, `x64`, `arm`, or `arm64`. See the
//! documentation for the WiX Toolset compiler (candle.exe) `-arch` option.
//! Note, this variable is deprecated and will eventually be removed because it is
//! ultimately redundant to the `$(sys.BUILDARCH)` variable that is already provided
//! by the WiX Toolset compiler. Existing projects should replace usage of
//! `$(var.Platform)` with `$(sys.BUILDARCH)`. No action is needed for new projects.
//! - `Profile` = (Deprecated) See `CargoProfile`.
//! - `Version` = The version for the installer. The default is the
//! `Major.Minor.Fix` semantic versioning number of the Rust package.
//!
//! Additional, user-defined variables for custom WXS files can be passed to the
//! WiX Toolset compiler (candle.exe) using the cargo-wix subcommand
//! `-C,--compiler-arg` option. For example,
//!
//! ```dos
//! C:\Path\To\Project\>cargo wix -C "-DUSER_KEY=USER_VALUE"
//! ```
//!
//! ### Extensions
//!
//! The [WixUIExtension] and [WixUtilExtension] are included in every execution
//! of the default _create_ cargo-wix subcommand, i.e. `cargo wix`. This is the
//! same as calling either the compiler (candle.exe) or the linker (light.exe)
//! with the `-ext WixUIExtension -ext WixUtilExtension` options. These two
//! extensions are commonly used to create installers when using the WiX
//! Toolset, so these are included by default. Additionally, the WixUIExtension
//! is used for the template WXS file.
//!
//! Additionally, the [WixBalExtension] is automatically included if the
//! cargo-wix subcommand detects a bundle (exe) is to be created instead of a
//! MSI package. See the [Bundles](#bundles) section for more information about
//! creating and managing bundles with the cargo-wix subcommand.
//!
//! ### Multiple WiX Sources
//!
//! The cargo-wix subcommand supports including multiple WXS files when creating
//! an installer. A lot of customization is possible through the WXS file and
//! sometimes the installer's source code becomes its own project where
//! organization and formatting are important. Breaking up a single, large WXS
//! file into multiple WXS files can be useful for code readability and project
//! navigation. Thus, cargo-wix will include any file with the `.wxs` file
//! extension found in the default source folder, `wix`, when creating an
//! installer. For example, say you have the following project with three WXS
//! files in the `wix` sub-folder:
//!
//! ```dos
//! C:\Path\to\Project> dir /B
//! Cargo.toml
//! src
//! wix
//! C:\Path\to\Project> dir wix /B
//! first.wxs
//! second.wxs
//! third.wxs
//! ```
//!
//! When the `cargo wix` default _create_ command is executed, all three WXS files
//! will be included and used to create the installer. Generally, this
//! translates to the following set of commands:
//!
//! ```dos
//! C:\Path\to\Project> candle -out target\wix\ wix\first.wxs wix\second.wxs wix\third.wxs
//! C:\Path\to\Project> light -out target\wix\example-0.1.0-x86_64.msi target\wix\first.wixobj target\wix\second.wixobj target\wix\third.wixobj
//! ```
//!
//! Alternatively, multiple WXS files can also be included when creating an
//! installer by including the relative or absolute paths to the WXS files as
//! arguments to the subcommand, but any WXS files in the default, `wix`,
//! sub-folder are ignored and would have to be explicitly included. For
//! example,
//!
//! ```dos
//! C:\Path\To\Project> cargo wix path\to\first\wxs\file\one.wxs path\to\second\wxs\file\two.wxs
//! ```
//!
//! ### Bundles
//!
//! It is possible to create [bundle-based installers] with the WiX Toolset. The
//! cargo-wix subcommand and binary currently offer limited support for creating
//! bundles from Rust projects. This includes automatically detecting if a
//! bundle is to be created by inspecting all WiX Object (wixobj) files before
//! linking and changing the file extension from `msi` to `exe`, as bundles
//! require the executable (exe) file extension. In addition to automatically
//! changing the file extension of the installer, the [WixBalExtension] is
//! included automatically during linking if a bundle is detected because this
//! extension includes a standard bootstrapper application that is commonly used
//! to build and customize bundles.
//!
//! While the cargo-wix subcommand does provide some support for bundles with
//! automatic file extension determination and inclusion of useful
//! bundle-centric extensions, the process for creating a bundle for a Rust
//! project is currently a manual process. Let's assume the following
//! [workspace]-based Rust project layout and structure:
//!
//! ```text
//! |-- C:\Path\to\Rust\Project
//! |-- |-- Cargo.toml
//! |-- |-- client
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! |-- |-- server
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! ```
//!
//! The virtual manifest, Cargo.toml, in the root of the project is a [virtual
//! manifest] that only contains the following:
//!
//! ```toml
//! [workspace]
//! members = ["client", "server"]
//! ```
//!
//! The package manifests for the workspace members, `client\Cargo.toml` and
//! `server\Cargo.toml`, are the typical package manifests content for a binary
//! package.
//!
//! The goal is to create a bundle-based executable that contains and
//! installs both the client and server packages through their respective
//! installers (MSI packages). A combination of manual file manipulation and the
//! cargo-wix subcommand can be used to accomplish this goal, all from the root
//! of the workspace. Begin by creating the WiX Source (wxs) files for the
//! client and server MSI packages:
//!
//! ```dos
//! C:\Path\to\Rust\Workspace> cargo wix init client\Cargo.toml
//! C:\Path\to\Rust\Workspace> cargo wix init server\Cargo.toml
//! ```
//!
//! The following project layout should have been created:
//!
//! ```text
//! |-- C:\Path\to\Rust\Workspace
//! |-- |-- Cargo.toml
//! |-- |-- client
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! |-- |-- |-- wix
//! |-- |-- |-- |-- main.wxs
//! |-- |-- server
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! |-- |-- |-- wix
//! |-- |-- |-- |-- main.wxs
//! ```
//!
//! The WiX Source (wxs) files created for the client and server crates of the
//! Rust workspace using the cargo-wix subcommand will be generated following
//! the normal rules, features, and templates when using the `cargo wix init`
//! within a non-workspace-based project.
//!
//! With the WiX Source (wxs) files created for the two MSI-based installers, a
//! bundle-based WiX Source (wxs) file must be created for the workspace and
//! bundle. Create a new `main.wxs` file and place it in a `wix` sub-folder
//! relative to the workspace root. The following project layout should exist:
//!
//! ```text
//! |-- C:\Path\to\Rust\Workspace
//! |-- |-- Cargo.toml
//! |-- |-- client
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! |-- |-- |-- wix
//! |-- |-- |-- |-- main.wxs
//! |-- |-- server
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! |-- |-- |-- wix
//! |-- |-- |-- |-- main.wxs
//! |-- |-- wix
//! |-- |-- |-- main.wxs
//! ```
//!
//! Open the `wix\main.wxs` file in a suitable text editor and add the following
//! XML to it to define the bundle:
//!
//! ```xml
//! <?xml version="1.0"?>
//! <Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
//!   <Bundle Version="1.0.0" UpgradeCode="[Your GUID Here]">
//!     <BootstrapperApplicationRef Id="WixStandardBootstrapperApplication.RtfLicense"/>
//!     <Chain>
//!       <MsiPackage SourceFile="client\target\wix\client-0.1.0-x86_64.msi" />
//!       <MsiPackage SourceFile="server\target\wix\server-0.1.0-x86_64.msi" />
//!     </Chain>
//!   </Bundle>
//! </Wix>
//! ```
//!
//! A GUID will need to be manually generated for the `UpgradeCode` attribute of
//! the `Bundle` tag and used to replace the `[Your GUID Here]` value. Now, the
//! bundle can be created but first both the client and server MSI packages must
//! be created. Thus, creating the bundle is a multi-step, or multi-command,
//! process:
//!
//! ```dos
//! C:\Path\to\Workspace> cargo wix client\Cargo.toml
//! C:\Path\to\Workspace> cargo wix server\Cargo.toml
//! C:\Path\to\Workspace> cargo wix --name Bundle --install-version 1.0.0
//! ```
//!
//! The following project layout should exist:
//!
//! ```text
//! |-- C:\Path\to\Rust\Workspace
//! |-- |-- Cargo.toml
//! |-- |-- client
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! |-- |-- |-- target
//! |-- |-- |-- |-- wix
//! |-- |-- |-- |-- |-- server-0.1.0-x86_64.msi
//! |-- |-- |-- wix
//! |-- |-- |-- |-- main.wxs
//! |-- |-- server
//! |-- |-- |-- Cargo.toml
//! |-- |-- |-- src
//! |-- |-- |-- |-- main.rs
//! |-- |-- |-- target
//! |-- |-- |-- |-- wix
//! |-- |-- |-- |-- |-- server-0.1.0-x86_64.msi
//! |-- |-- |-- wix
//! |-- |-- |-- |-- main.wxs
//! |-- |-- target
//! |-- |-- |-- Release
//! |-- |-- |-- |-- client.exe
//! |-- |-- |-- |-- server.exe
//! |-- |-- |-- wix
//! |-- |-- |-- |-- Bundle-1.0.0-x86_64.exe
//! |-- |-- wix
//! |-- |-- |-- main.wxs
//! ```
//!
//! Note the built binaries are located in the `target\Release` folder relative
//! to the workspace root as opposed to the `client\target\Release` and
//! `server\target\Release` folders, even though the MSI packages are available
//! in the member's `target\wix` folders. This will fail if the various `cargo
//! wix` commands are _not_ executed from the workspace root.
//!
//! The `name` and `install-version` options can be moved into a
//! [configuration](#configuration) section for the subcommand within the
//! virtual manifest of the workspace, i.e. the Cargo.toml file with the
//! `[workspace]` section to avoid having to retype them each time a bundle is
//! created, but all three `cargo wix` commands must be issued each time.
//!
//! While the above steps will create a bundle installer for the workspace-based
//! Rust project with a default, placeholder EULA, it is very manual and
//! cumbersome. For example, the bundle-based WiX Source (wxs) file will have to
//! be manually updated each time the version numbers of the member MSI packages
//! are changed because the paths to the source files are hard-coded in the XML.
//! Efforts are underway to improve support within the cargo-wix subcommand for
//! both workspaces and bundles ([Issue #74] and [Issue #98]).
//!
//! [Issue #74]: https://github.com/volks73/cargo-wix/issues/74
//! [Issue #98]: https://github.com/volks73/cargo-wix/issues/98
//!
//! ## Configuration
//!
//! The default subcommand, `cargo wix`, which creates a MSI based on the
//! contents of the package's manifest (Cargo.toml) can be configured by adding
//! a `[package.metadata.wix]` section to the manifest. For each CLI option for
//! the default _create_ subcommand, a field can be added to the
//! `[package.metadata.wix]` section. If the corresponding CLI option is used
//! with the default _create_ subcommand, then the CLI option value will
//! override the value in the metadata section.
//!
//! Below is an example `[package.metadata.wix]` section from a package's
//! manifest that configures the default _create_ subcommand:
//!
//! ```toml
//! [package.metadata.wix]
//! compiler-args = ["-nologo", "-wn"]
//! culture = "Fr-Fr"
//! dbg-build = false
//! dbg-name = false
//! eula = "path\to\eula.rtf"
//! include = ["Path\to\WIX\Source\File\One.wxs", "Path\to\WIX\Source\File\Two.wxs"]
//! license = "path\to\license.txt"
//! linker-args = ["-nologo"]
//! locale = "Path\to\WIX\Localization\File.wxl"
//! name = "example"
//! no-build = false
//! output = "Path\and\file\name\for\installer.msi"
//! path-guid = "BFD25009-65A4-4D1E-97F1-0030465D90D6"
//! upgrade-guid = "B36177BE-EA4D-44FB-B05C-EDDABDAA95CA"
//! version = "2.1.0"
//! ```
//!
//! See the documentation for each CLI option for more information about each
//! field and its purpose. Note, the `name` and `version` fields will be the
//! name and version number of the application as it appears in the Add/Remove
//! Programs control panel, and the version number does _not_ need to match the
//! package's version number. In other words, the installed application can have
//! a different version number from the package, which is useful with multiple
//! binaries, workspaces, or distributing a "suite" of applications, where the
//! version number would be for the suite and not necessarily the individual
//! applications within the suite.
//!
//! Please note that unlike most of the fields, the `include` field is an [TOML
//! array] instead of a string value. This is the same as passing multiple paths
//! to the default _create_ subcommand using multiple `-I,--include` options or
//! including multiple WXS files in the default, `wix`, project source location.
//!
//! The only CLI option, or argument, that is not supported in the
//! `[package.metadata.wix]` section is the `<INPUT>` argument for the default
//! _create_ command, which specifies a relative or absolute path to a package's
//! manifest file. The assumption is that the package manifest (Cargo.toml) to
//! be used for the default _create_ subcommand is the same manifest that
//! contains the `[package.metadata.wix]` section.
//!
//! ## Flags and Options
//!
//! Generally, any value that is obtained from the package's manifest
//! (Cargo.toml) can be overridden at the command line with an appropriate
//! option. For example, the manufacturer, which is displayed as the "Publisher"
//! in the Add/Remove Programs (ARP) control panel is obtained from the first
//! author listed in the `authors` field of a project's manifest, but it can be
//! overridden using the `-m,--manufacturer` option with the `cargo wix init`
//! subcommand.
//!
//! Use the `-h,--help` flag with each subcommand to get a full list of options
//! and flags that are available. The short flag, `-h`, will print a condensed
//! version of each flag and option for the subcommand, while the long flag,
//! `--help`, will print a detailed help for each flag and option. The rest of
//! this section is a list of all flags and options implemented for all
//! subcommands.
//!
//! ### `-b,--banner`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Sets the path to an image file (.png, .bmp, ...) that will be displayed across
//! the top of each dialog in the installer. The banner image dimensions should
//! be 493 x 58 pixels, but the left-most 50% of that image is covered with text,
//! so if you want to leave a blank background for text readability, you only want
//! to color in the right-most ~200 pixels of that image.
//!
//!
//! ### `-b,--bin-path`
//!
//! Available for the default _create_ (`cargo wix`) and _sign_ (`cargo wix
//! sign`) subcommands.
//!
//! The `-b,--bin-path` option can be used to specify a path (relative or
//! absolute) to the WiX Toolset `bin` folder. The `-b,--bin-path` option is
//! useful if a different version of the WiX Toolset needs to be used to create
//! the installer. The descending order of precedence is: (1) `-b,--bin-path`
//! option then (2) `WIX` system environment variable. An error will be
//! displayed if the compiler and/or linker cannot be found.
//!
//! This option is also available for the `cargo wix sign` subcommand and can be
//! used to specify a path to the Windows SDK `bin` folder. This can be used to
//! override default `signtool` application found using the
//! [`std::process::Command::status`] method.
//!
//! ### `-B,--binary`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! A path to a binary, a.k.a. executable, to include in the installer _instead_
//! of any and all binaries defined in the package's manifest (Cargo.toml). By
//! default, all binaries defined with the `[[bin]]` section in the package's
//! manifest are included in the installer. If no `[[bin]]` section is defined,
//! then the package's `name` field is used. This option can be used to
//! override, i.e. _not_ append, the binaries that should be included in the
//! installer.
//!
//! This option can be used multiple times to define multiple binaries to
//! include in the installer. The value is a path to a binary file. The file
//! stem (file name without extension) is used as the binary name within the WXS
//! file. A relative or absolute path is acceptable.
//!
//! ### `-C,--compiler-arg`
//!
//! Available for the default _create (`cargo wix`) subcommand.
//!
//! Appends an argument to the WiX compiler (candle.exe) invocation. This
//! provides a mechanism for "passing" arguments to the compiler. This can be called
//! multiple times to pass multiple arguments (flags or options), but only one
//! value per occurrence is allowed to avoid ambiguity during argument parsing.
//! Note, if it is an option, i.e. argument with an accompanying value, then the
//! value must be passed as a separate usage of this option. For example, adding
//! an user-defined compiler extension would require the following command
//! `cargo wix -C -ext -C UserDefinedExtension` to yield a `candle -ext
//! UserDefinedExtension` invocation.
//!
//! ### `-c,--culture`
//!
//! Available for the default _create_ (`cargo wix`) subcommand.
//!
//! Sets the culture for localization. Use with the [`-l,--locale`] option. See
//! the [WixUI localization documentation] for more information about acceptable
//! culture codes. The codes are case insensitive.
//!
//! ### `-d,--dbg-build`
//!
//! Available only for the default _create_ (`cargo wix`) subcommmand.
//!
//! Builds the package using the Debug profile instead of the Release profile
//! with Cargo. This flag is ignored if the `--no-build` flag is used. The
//! default is to build the package with the Release profile.
//!
//! ### `-D,--dbg-name`
//!
//! Available only for the default _create_ (`cargo wix`) subcommand.
//!
//! Appends `-debug` to the file stem (portion before the dot and file
//! extension) of the installer's file name. The default is to _not_ include the
//! suffix. Generally, this should be used to indicate an installer contains a
//! binary that was built with debugging information and minimal optimizations
//! as a tradeoff for being able to troubleshoot execution of the application on
//! an end-user's system. A release build generally does not contain any
//! debugging information but includes optimizations. It is typical to use this
//! flag with the `-d,--dbg-build` flag but it is not required. This allows a
//! developer to provide other mechanisms for creating a debugging variant of
//! his or her application and still use the Release profile.
//!
//! ### `-d,--description`
//!
//! Available for the _init_ (`cargo wix init`), _print_ (`cargo wix print`),
//! and _sign_ (`cargo wix sign`) subcommands.
//!
//! The package description is used in multiple places for the installer,
//! including the text that appears in the blue UAC dialog when using a signed
//! installer. This can be overridden using the `-d,--description` option with
//! the `cargo wix init` or `cargo wix sign` subcommands, respectively.
//!
//! ### `-D,--dialog`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Sets the path to an image file (.png, bmp, ...) that will be displayed as
//! the background of the first dialog of the installer. The dialog image dimensions
//! should be 493 x 312 pixels, but the right-most 60% of that area is covered
//! by the actual text of the dialog, so if you want to leave a blank background for text
//! readability, you only want to color in the left-most ~200 pixels of that image.
//!
//! The first dialog is known as the "Welcome" dialog.
//!
//! ### `-e,--eula`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Specifies a Rich Text Format (RTF) file to use as the End User License
//! Agreement (EULA) for the license agreement dialog of the installer. The
//! default is to disable the license agreement dialog unless one of the
//! supported licenses (GPL-3.0, Apache-2.0, or MIT) is generated based on the
//! value of the `license` field in the package's manifest (Cargo.toml). An EULA
//! can be enabled later by directly modifying the WiX Source (WXS) file with a
//! text editor.
//!
//! When specified via `package.metadata.wix.eula` the path is assumed to be relative
//! to the Cargo.toml (directory). This field can also be set to `false` to disable
//! the eula even if we could auto-generate one for you, as described above.
//!
//! ### `--force`
//!
//! Available for the _init_ (`cargo wix init`) subcommand.
//!
//! Forces overwriting of generated files from the _init_ subcommand. Use with
//! caution! This cannot be undone.
//!
//! ### `-h,--help`
//!
//! Available for all subcommands.
//!
//! The short flag, `-h`, will print a condensed version of the help text, while
//! the long flag, `--help`, will print a more detailed version of the help
//! text.
//!
//! ### `-u,--homepage`
//!
//! Available for the _sign_ (`cargo wix sign`) subcommand.
//!
//! This will be displayed in the ACL dialog.
//!
//! ### `--install`
//!
//! Available for the default _create_ (`cargo wix`) subcommand.
//!
//! Automatically runs the installer after creating it.
//!
//! ### `-i,--install-version`
//!
//! Available for the default _create_ (`cargo wix`) subcommand.
//!
//! Overrides the version from the package's manifest (Cargo.toml), which is
//! used for the installer name and appears in the Add/Remove Programs (ARP)
//! control panel.
//!
//! ### `-I,--include`
//!
//! Available only for the default _create_ (`cargo wix`) subcommand.
//!
//! This option can be used multiple times to include multiple WiX Source (WXS)
//! files in the creation of an installer. The option takes a path to a single
//! WXS file as its value. Any WXS files located in the default `wix` folder
//! located within the package's root folder, i.e. same location as the
//! package's manifest (Cargo.toml) are automatically included and used in the
//! creation of the installer. This option allows the inclusion of other WXS
//! files outside of the default `wix` location.
//!
//! ### `-l,--license`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Overrides the `license-file` field or any license generated from the
//! `license` field of the package's manifest (Cargo.toml). If an appropriate
//! license file does not exist, cannot be found, or is not specified, then no
//! license file is included in the installer or in the installation folder
//! along side the `bin` folder. A file containing the license, such as a TXT,
//! PDF, or RTF file, can be added later by directly editing the generated WiX
//! Source file (WXS) in a text editor.
//!
//! When specified via `package.metadata.wix.license` the path is assumed to be relative
//! to the Cargo.toml (directory). This field can also be set to `false` to disable
//! the license auto-generation features described above.
//!
//! ### `-L,--linker-arg`
//!
//! Available for the default _create (`cargo wix`) subcommand.
//!
//! Appends an argument to the WiX linker (light.exe) invocation. This provides
//! a mechanism for "passing" arguments to the linker. This can be called
//! multiple times to pass multiple arguments (flags or options). Only one value
//! per occurrence is allowed to avoid ambiguity during argument parsing. Note,
//! if it is an option, i.e. argument with an accompanying value, then the value
//! must be passed as a separate usage of this option. For example, adding an
//! user-defined linker extension would require the following command `cargo wix
//! -L -ext -L UserDefinedExtension` to yield a `light -ext
//! UserDefinedExtension` invocation.
//!
//! ### `-l,--locale`
//!
//! Available for the default _create_ (`cargo wix`) subcommand.
//!
//! Sets the path to a WiX localization file (wxl) which contains localized
//! strings. Use in conjunction with the [`-c,--culture`] option.
//!
//! ### `-m,--manufacturer`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Overrides the `authors` field of the package's manifest
//! (Cargo.toml) as the manufacturer within the installer. The manufacturer can
//! be changed after initialization by directly modifying the WiX Source file
//! (WXS) with a text editor.
//!
//! ### `-n,--name`
//!
//! Available for the default _create_ (`cargo wix`) subcommand.
//!
//! Overrides the `name` field in the package's manifest (Cargo.toml), which is
//! used in the file name of the installer (msi). This does not change the name
//! of the executable _within_ the installer.
//!
//! ### `--no-build`
//!
//! Available for the default _create_ (`cargo wix`) subcommand.
//!
//! This skips building the Rust package using Cargo for the Release target.
//!
//! ### `--nocapture`
//!
//! Available for the default _create_ (`cargo wix`) and _sign_ (`cargo wix sign`)
//! subcommands.
//!
//! Displays all output from the builder (Cargo), compiler (candle.exe), linker
//! (light.exe), and signer (signtool.exe) applications.
//!
//! ### `-o,--output`
//!
//! Available for the default _create_ (`cargo wix`), _init_ (`cargo wix init`) and
//! _print_ (`cargo wix print`) subcommands.
//!
//! Sets the destination for _init_ subcommand files, such as the WiX Source
//! file (WXS), an alternative to stdout for _print_ subcommand, and the created
//! installer for the default _create_ subcommand.
//!
//! When used with the default _create_ subcommand to create an installer (MSI),
//! if the path is to an existing directory or the path has a trailing `/` or
//! `\`, then the MSI will be available after creation at the specified path,
//! but the MSI file name will be the default file name based on the package
//! name, version, and platform.
//!
//! ### `-O,--owner`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Sets the copyright owner of the license during initialization or printing.
//! The default is to use the `authors` field of the package's manifest
//! (Cargo.toml). This is only used when generating a license based on the value
//! of the `license` field in the package's manifest.
//!
//! ### `-p,--package`
//!
//! Available for the _create_ (`cargo wix`), _init_ (`cargo wix init`), and
//! _print_ (`cargo wix print`) subcommands.
//!
//! Selects the package within a workspace. This is required if a project
//! organized with a workspace. A workspace can have one or more members, where
//! each member may have a separate installer. This option has no effect if the
//! project does not use a workspace.
//!
//! ### `--path-guid`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Overrides the automatically generated GUID for the path component with an
//! existing GUID in the hyphenated, uppercase format. The path GUID should only
//! be generated once for a product/project. The same GUID should be used for
//! all installer creations to ensure no artifacts are left after uninstalling
//! and proper modification of the `PATH` environment variable.
//!
//! ### `--product-icon`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Sets the path to a 16x16 image file (.ico) that will be display as an icon in the
//! Add/Remove Programs (ARP) control panel for the installed application.
//!
//! ### `--product-name`
//!
//! Available for the _init_ (`cargo wix init`), _print_ (`cargo wix print`),
//! and _sign_ (`cargo wix sign`) subcommands.
//!
//! Overrides the `name` field of the package's manifest (Cargo.toml) as the
//! product name within the installer. The product name can be changed after
//! initialization or printing by directly modifying the WiX Source file (wxs)
//! with a text editor.
//!
//! ### `-t,--timestamp`
//!
//! Available for the _sign_ (`cargo wix sign`) subcommand.
//!
//! An alias or URL to a timestamp server when signing an installer with a
//! certificate. Valid aliases are: `Comodo` and `Versign`, which are case
//! insensitive.
//!
//! ### `--upgrade-guid`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Overrides the automatically generated GUID for the product's upgrade code
//! with an existing GUID in the hyphenated, uppercase format. The upgrade code
//! should only be generated once for a product/project. The same upgrade code
//! should then be used for all installer creations of the same product/project. If
//! a new GUID is used every time an installer is created, then each installer will
//! be installing the same product but as separate installations.
//!
//! ### `-u,--url`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Adds a URL to the installer that will be displayed in the Add/Remove
//! Programs (ARP) control panel for the application. The default is to disable
//! it unless a URL is specified for either the `homepage`, `documentation`, or
//! `repository` fields in the package's manifest (Cargo.toml). The help URL can
//! be enabled after initialization by directly modifying the WiX Source (wxs)
//! file with a text editor.
//!
//! ### `-V,--version`
//!
//! Available for all subcommands.
//!
//! Prints the cargo-wix binary and subcommand version.
//!
//! ### `-v,--verbose`
//!
//! Available for all subcommands.
//!
//! Increases the level of logging statements based on occurrence count of the
//! flag. The more `-v,--verbose` flags used, the more logging statements that
//! will be printed during execution of a subcommand. When combined with the
//! `--nocapture` flag, this is useful for debugging and testing.
//!
//! ### `-i,--installer`
//!
//! Available for the _sign_ (`cargo wix sign`) subcommand.
//!
//! Speicifies path to the installer(msi) to be signed.
//!
//! ### `-y,--year`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Sets the copyright year for the license during initialization. The default
//! is to use the current year. This is only used if a license is generated from
//! one of the supported licenses based on the value of the `license` field in
//! the package's manifest (Cargo.toml).
//!
//! [Alacritty]: https://github.com/alacritty/alacritty
//! [bundle-based installers]: https://wixtoolset.org/documentation/manual/v3/bundle/
//! [Cargo]: https://crates.io
//! [cargo subcommand]: https://github.com/rust-lang/cargo/wiki/Third-party-cargo-subcommands
//! [crates.io]: https://crates.io
//! [Developer Prompt]: https://msdn.microsoft.com/en-us/library/f35ctcxw.aspx
//! [`description`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [`documentation`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [documentation]: http://wixtoolset.org/documentation/
//! [git bash]: https://gitforwindows.org/
//! [GUID]: https://en.wikipedia.org/wiki/Universally_unique_identifier
//! [`homepage`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [library]: ../wix/index.html
//! [LibreOffice]: https://www.libreoffice.org/
//! [`license`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [`license-file`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [Merge module]: https://wixtoolset.org/documentation/manual/v3/howtos/redistributables_and_install_checks/install_vcredist.html
//! [Microsoft Office]: https://products.office.com/en-us/home
//! [Microsoft Notepad]: https://en.wikipedia.org/wiki/Microsoft_Notepad
//! [PowerShell]: https://github.com/PowerShell/PowerShell
//! [`repository`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [Rich Text Format]: https://en.wikipedia.org/wiki/Rich_Text_Format
//! [Rust]: https://www.rust-lang.org
//! [sidecar]: https://en.wikipedia.org/wiki/Sidecar_file
//! [SignTool]: https://msdn.microsoft.com/en-us/library/windows/desktop/aa387764(v=vs.85).aspx
//! [statically link the CRT]: https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes
//! [`std::process::Command`]: https://doc.rust-lang.org/std/process/struct.Command.html
//! [`std::process::Command::status`]: https://doc.rust-lang.org/std/process/struct.Command.html#method.status
//! [TOML array]: https://github.com/toml-lang/toml#user-content-array
//! [tutorials]: https://www.firegiant.com/wix/tutorial/
//! [VC Build Tools]: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2017
//! [virtual manifest]: https://doc.rust-lang.org/cargo/reference/workspaces.html
//! [Windows 10 SDK]: https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk
//! [WixBalExtension]: https://wixtoolset.org/documentation/manual/v3/bundle/wixstdba/
//! [WixUIExtension]: https://wixtoolset.org//documentation/manual/v3/wixui/wixui_dialog_library.html
//! [WixUtilExtension]: https://wixtoolset.org/documentation/manual/v3/xsd/util/
//! [WixUI localization documentation]: http://wixtoolset.org/documentation/manual/v3/wixui/wixui_localization.html
//! [WiX preprocessor]: https://wixtoolset.org/documentation/manual/v3/overview/preprocessor.html
//! [WiX Toolset]: http://wixtoolset.org
//! [WordPad]: https://en.wikipedia.org/wiki/WordPad
//! [workspace]: https://doc.rust-lang.org/cargo/reference/workspaces.html
//! [WXS]: ../wix/enum.Template.html
//! [XML]: https://en.wikipedia.org/wiki/XML

use clap::{Arg, ArgAction, Command};

use env_logger::fmt::Color as LogColor;
use env_logger::Builder;

use log::{Level, LevelFilter};

use std::io::Write;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use wix::clean;
use wix::create;
use wix::initialize;
use wix::print;
use wix::purge;
use wix::sign;
use wix::{Template, BINARY_FOLDER_NAME, WIX_PATH_KEY};

pub const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

const SUBCOMMAND_NAME: &str = "wix";

fn main() {
    // The banner option for the `init` and `print` subcommands.
    let banner = Arg::new("banner")
        .help("A path to an image file (.bmp) for the installer's banner")
        .long_help(
            "Sets the path to a bitmap (.bmp) image file that will be \
             displayed across the top of each dialog in the installer. The banner \
             image dimensions should be 493 x 58 pixels.",
        )
        .long("banner")
        .short('b')
        .num_args(1);
    // The binaries option for the `init` and `print` subcommands.
    let binaries = Arg::new("binaries")
        .help("A path to an executable file (.exe) for the application")
        .long_help(
            "Sets the path to an executable file that will override the default \
             binaries included in the installer. The default binaries are the \
             'target\\release\\<package name>.exe' file, where <package-name> is \
             the value from the 'name' field in the '[package]' section of the \
             package's manifest (Cargo.toml) if no '[[bin]]' sections are \
             defined; otherwise, all binaries defined in the package's manifest \
             in each '[[bin]]' section are included. This option overrides any \
             and all binaries defined in the package's manifest. Use this option \
             repeatedly to include multiple binaries.",
        )
        .long("binary")
        .action(ArgAction::Append)
        .short('B')
        .num_args(1);
    // The description option for the `init` and `print` subcommands.
    let description = Arg::new("description")
        .help("A string describing the application in the installer")
        .long_help(
            "Overrides the 'description' field of the package's manifest \
             (Cargo.toml) as the description within the installer. Text with spaces \
             should be surrounded by double quotes.",
        )
        .long("description")
        .short('d')
        .num_args(1);
    // The dialog option for the `init` and `print` subcommands.
    let dialog = Arg::new("dialog")
        .help("A path to an image file (.bmp) for the installer's welcome dialog")
        .long_help(
            "Sets the path to a bitmap (.bmp) image file that will be \
             displayed to the left on the first dialog of the installer. The dialog \
             image dimensions should be 493 x 312 pxiels.",
        )
        .long("dialog")
        .short('D')
        .num_args(1);
    // The eula option for the `init` and `print` subcommands.
    let eula = Arg::new("eula")
        .help("A path to a RTF file (.rtf) for the installer's license agreement dialog")
        .long_help(
            "Specifies a Rich Text Format (RTF) file to use as the End \
             User License Agreement (EULA) for the license agreement dialog of the \
             installer. The default is to disable the license agreement dialog unless \
             one of the supported licenses (GPL-3.0, Apache-2.0, or MIT) is generated \
             based on the value of the 'license' field in the package's manifest \
             (Cargo.toml). An EULA can be enabled later by directly modifying the WiX \
             Source (wxs) file with a text editor.",
        )
        .long("eula")
        .short('e')
        .num_args(1);
    // The license option for the `init` and `print` subcommands.
    let license = Arg::new("license")
        .help("A path to any file (.txt, .pdf, .rtf, etc.) to be used as a license")
        .long_help(
            "Overrides the 'license-file' field or any license generated \
             from the 'license' field of the package's manifest (Cargo.toml). If an \
             appropriate license file does not exist, cannot be found, or is not \
             specified, then no license file is included in the installer or the \
             installation folder along side the binary. A file containing the \
             license, such as a TXT, PDF, or RTF file, can be added later by directly \
             editing the generated WiX Source file (wxs) in a text editor.",
        )
        .long("license")
        .short('l')
        .num_args(1);
    // The url option for the `init` and `print` subcommands
    let url = Arg::new("url")
        .help("A URL for the Add/Remove Programs control panel's Help Link")
        .long_help(
            "Adds a URL to the installer that will be displayed in the \
             Add/Remove Programs control panel for the application. The default is to \
             disable it unless a URL is specified for either the 'homepage', \
             'documentation', or 'repository' fields in the package's manifest \
             (Cargo.toml). The help URL can be enabled after initialization by \
             directly modifying the WiX Source (wxs) file with a text editor.",
        )
        .long("url")
        .short('u')
        .num_args(1);
    // The manufacturer option for the `init` and `print` subcommands
    let manufacturer = Arg::new("manufacturer")
        .help("A string for the Add/Remove Programs control panel's Manufacturer")
        .long_help(
            "Overrides the 'authors' field of the \
             package's manifest (Cargo.toml) as the manufacturer within the \
             installer. The manufacturer can be changed after initialization by \
             directly modifying the WiX Source file (wxs) with a text editor.",
        )
        .long("manufacturer")
        .short('m')
        .num_args(1);
    // The owner option for the `init` and `print` subcommands
    let owner = Arg::new("owner")
        .help("A string for a generated license's copyright holder")
        .long_help(
            "Sets the copyright owner for the license during \
             initialization. The default is to use the `authors` field from the \
             package's manifest (Cargo.toml). This is only used when generating a \
             license based on the value of the 'license' field in the package's \
             manifest.",
        )
        .long("owner")
        .short('O')
        .num_args(1);
    // The package option for the `create`, `init`, and `print` subcommands
    let package = Arg::new("package")
        .help("The name of the package in the current workspace")
        .long_help(
            "Selects the package within a project organized as a workspace. \
             Workspaces have one or more members, where each member is a package. \
             This option selects the package by name.",
        )
        .long("package")
        .short('p')
        .num_args(1);
    // The path guid option for the `init` and `print` subcommands
    let path_guid = Arg::new("path-guid")
        .help("A string formatted as a v4 hyphenated, uppercase UUID for the path component")
        .long_help(
            "Overrides the automatically generated GUID for the path component. \
             The path component needs a constant GUID so that the PATH \
             environment variable can be updated properly during uninstall and \
             upgrading. Generally, the GUID is generated once at the start \
             of a product/project and stored in the WiX Source (WXS) file. Using \
             a different GUID for each installer creation will leave artifacts \
             after uninstallation.",
        )
        .long("path-guid")
        .num_args(1);
    // The product icon option for the `init` and `print` subcommands
    let product_icon = Arg::new("product-icon")
        .help("A path to an image file (.ico) for the Add/Remove Programs control panel")
        .long_help(
            "Sets the path to an image file that will be displayed as an \
             icon in the Add/Remove Programs (ARP) control panel for the installed \
             application.",
        )
        .long("product-icon")
        .num_args(1);

    // The product name option for the `init`, `print`, and `sign` subcommands.
    let product_name = Arg::new("product-name")
        .help("A string for the Add/Remove Programs control panel's Name")
        .long_help(
            "Overrides the 'name' field of the package's manifest \
             (Cargo.toml) as the product name within the installer. The product name \
             can be changed after initialization by directly modifying the WiX Source \
             file (wxs) with a text editor.",
        )
        .long("product-name")
        .num_args(1);
    // The upgrade guid option for the `init` and `print` subcommands.
    let upgrade_guid = Arg::new("upgrade-guid")
        .help("A string formatted as a v4 hyphenated, uppercase UUID for the globally unique upgrade code")
        .long_help(
            "Overrides the automatically generated GUID for the product's \
             upgrade code. The upgrade code is used to determine if the installer \
             is for a different product or the same product but should be \
             upgraded instead of a new install. Generally, the upgrade code is \
             generated once at the start of a product/project and stored in the \
             WiX Source (WXS) file. Using a different GUID for each installer \
             creation will install separate versions of a product."
        )
        .long("upgrade-guid")
        .num_args(1);
    // The "global" verbose flag for all subcommands.
    let verbose = Arg::new("verbose")
        .help("The verbosity level for logging statements")
        .long_help(
            "Sets the level of verbosity. The higher the level of \
             verbosity, the more information that is printed and logged when the \
             application is executed. This flag can be specified multiple times, \
             where each occurrence increases the level and/or details written for \
             each statement.",
        )
        .long("verbose")
        .short('v')
        .action(ArgAction::Count);
    let year = Arg::new("year")
        .help("A string for a generated license's copyright year")
        .long_help(
            "Sets the copyright year for the license during \
             initialization. The default is to use the current year. This is only \
             used if a license is generated from one of the supported licenses based \
             on the value of the 'license' field in the package's manifest \
             (Cargo.toml).",
        )
        .long("year")
        .short('y')
        .num_args(1);
    let matches = Command::new(PKG_NAME)
        .bin_name("cargo")
        .subcommand(
            Command::new(SUBCOMMAND_NAME)
                .version(PKG_VERSION)
                .about(PKG_DESCRIPTION)
                .arg(Arg::new("bin-path")
                     .help(format!(
                         "A path to the WiX Toolset's '{BINARY_FOLDER_NAME}' folder"))
                     .long_help(format!(
                         "Specifies the path to the WiX Toolset's '{BINARY_FOLDER_NAME}' folder, which should contain \
                         the needed 'candle.exe' and 'light.exe' applications. The default is to use \
                         the path specified with the {WIX_PATH_KEY} system environment variable that is created \
                         during the installation of the WiX Toolset. Failing the existence of the \
                         {WIX_PATH_KEY} system environment variable, the path specified in the PATH system \
                         environment variable is used. This is useful when working with multiple \
                         versions of the WiX Toolset."))
                     .long("bin-path")
                     .short('b')
                     .num_args(1))
                .subcommand(Command::new("clean")
                    .version(PKG_VERSION)
                    .about("Deletes the 'target\\wix' folder")
                    .long_about("Deletes the 'target\\wix' folder if it exists.")
                    .arg(verbose.clone())
                    .arg(Arg::new("INPUT")
                         .help("A path to a package's manifest (Cargo.toml)")
                         .long_help("The 'target\\wix' folder that exists \
                            alongside the package's manifest will be removed. This \
                            is optional and the default is to use the current \
                            working directory (cwd).")
                         .index(1)))
                .arg(Arg::new("culture")
                    .help("The culture code for localization")
                    .long_help("Sets the culture for localization. Use with the \
                        '-l,--locale' option. See the WixUI localization \
                        documentation for more information about acceptable culture \
                        codes. The codes are case insensitive.")
                    .long("culture")
                    .short('c')
                    .num_args(1))
                .arg(Arg::new("compiler-arg")
                    .help("Send an argument to the WiX compiler (candle.exe)")
                    .long_help("Appends the argument to the command that is \
                        invoked when compiling an installer. This is the same as \
                        manually typing the option or flag for the compiler at the \
                        command line. If the argument is for an option with a value, \
                        the option's value must be passed as a separate call of this \
                        option. Multiple occurrences are possible, but only one \
                        value per occurrence is allowed to avoid ambiguity in \
                        argument parsing. For example, '-C -ext -C \
                        WixUtilExtension'.")
                    .long("compiler-arg")
                    .short('C')
                    .num_args(1)
                    .action(ArgAction::Append)
                    .allow_hyphen_values(true))
                .arg(Arg::new("target")
                    .help("The cargo target to build the WiX installer for.")
                    .long_help("Tells cargo to build the given target, and instructs \
                        WiX to build an installer targeting the right architecture.")
                    .long("target")
                    .short('t')
                    .num_args(1))
                .arg(Arg::new("debug-build")
                    .help("Builds the package using the Debug profile")
                    .long_help("Uses the Debug profile when building the package \
                        using Cargo. The default is to build the package using the \
                        Release profile.")
                    .long("dbg-build")
                    .short('d')
                    .action(ArgAction::SetTrue))
                .arg(Arg::new("profile")
                    .help("Builds the package using the given profile")
                    .long_help("Uses the given profile when building the package \
                        using Cargo. The default is to build the package using the \
                        Release profile.")
                    .long("profile")
                    .num_args(1))
                .arg(Arg::new("debug-name")
                    .help("Appends '-debug' to the file stem of installer's file name")
                    .long_help("Adds the '-debug' suffix to the file stem \
                        (content before the file extension) for the installer's file \
                        name. This should be used to indicate the binary distributed \
                        within the installer was built using debugging information \
                        and optimizations. Generally, this should be used in \
                        combination with the '-d,--dbg-build' flag to build the \
                        binary with the Debug profile.")
                    .long("dbg-name")
                    .short('D')
                    .action(ArgAction::SetTrue))
                .arg(Arg::new("include")
                    .help("Include an additional WiX Source (wxs) file")
                    .long_help("Includes a WiX source (wxs) file for a project, \
                        where the wxs file is not located in the default location, \
                        i.e. 'wix'. Use this option multiple times to include \
                        multiple wxs files.")
                    .long("include")
                    .short('I')
                    .num_args(1)
                    .action(ArgAction::Append))
                .subcommand(Command::new("init")
                    .version(PKG_VERSION)
                    .about("Generates files from a package's manifest (Cargo.toml) to create an installer")
                    .long_about("Uses a package's manifest (Cargo.toml) to generate a WiX Source (wxs) \
                        file that can be used immediately without modification to create an \
                        installer for the package. This will also generate an EULA in the Rich \
                        Text Format (RTF) if the 'license' field is specified with a supported \
                        license (GPL-3.0, Apache-2.0, or MIT). All generated files are placed in \
                        the 'wix' sub-folder by default.")
                        .arg(Arg::new("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("If the '-o,--output' option is not used, \
                            then all output from initialization will be placed in a \
                            'wix' folder created alongside this path.")
                        .index(1))
                    .arg(banner.clone())
                    .arg(binaries.clone())
                    .arg(description.clone())
                    .arg(dialog.clone())
                    .arg(eula.clone())
                    .arg(Arg::new("force")
                        .help("Overwrite existing WiX-related files")
                        .long_help("Overwrites any existing files that are \
                            generated during initialization. Use with caution.")
                        .long("force")
                        .action(ArgAction::SetTrue))
                    .arg(license.clone())
                    .arg(manufacturer.clone())
                    .arg(Arg::new("output")
                        .help("A path to a folder for generated files")
                        .long_help("Sets the destination for all files \
                            generated during initialization. The default is to \
                            create a 'wix' folder within the project then generate \
                            all files in the 'wix' sub-folder.")
                        .long("output")
                        .short('o')
                        .num_args(1))
                    .arg(owner.clone())
                    .arg(package.clone())
                    .arg(path_guid.clone())
                    .arg(product_icon.clone())
                    .arg(product_name.clone())
                    .arg(upgrade_guid.clone())
                    .arg(url.clone())
                    .arg(verbose.clone())
                    .arg(year.clone()))
                .arg(Arg::new("INPUT")
                     .help("Path to a package's manifest (Cargo.toml) file.")
                     .long_help("If no value is provided, then the current \
                        working directory (CWD) will be used to locate a package's \
                        manifest. An error will occur if a manifest cannot be \
                        found. A relative or absolute path to a package's manifest \
                        (Cargo.toml) file can be used. Only one manifest is \
                        allowed. The creation of an installer will be relative to \
                        the specified manifest.")
                     .required(false)
                     .index(1))
                .arg(Arg::new("install-version")
                    .help("A string for the Add/Remove Programs control panel's version number")
                    .long_help("Overrides the version from the package's manifest \
                        (Cargo.toml), which is used for the installer name and \
                        appears in the Add/Remove Programs control panel.")
                    .long("install-version")
                    .short('i')
                    .num_args(1))
                .arg(Arg::new("linker-arg")
                    .help("Send an argument to the WiX linker (light.exe)")
                    .long_help("Appends the argument to the command that is \
                        invoked when linking an installer. This is the same as \
                        manually typing the option or flag for the linker at the \
                        command line. If the argument is for an option with a value, \
                        the option's value must be passed as a separate call of this \
                        option. Multiple occurrences are possible, but only one \
                        value per occurrence is allowed to avoid ambiguity in \
                        argument parsing. For example, '-L -ext -L \
                        WixUIExtension'.")
                    .long("linker-arg")
                    .short('L')
                    .num_args(1)
                    .action(ArgAction::Append)
                    .allow_hyphen_values(true))
                .arg(Arg::new("locale")
                    .help("A path to a WiX localization file (.wxl)")
                    .long_help("Sets the path to a WiX localization file (wxl) \
                        which contains localized strings. Use in conjunction with \
                        the '-c,--culture' option.")
                    .long("locale")
                    .short('l')
                    .num_args(1))
                .arg(Arg::new("name")
                    .help("A string for the installer's product name")
                    .long_help("Overrides the 'name' field in the package's \
                        manifest (Cargo.toml), which is used in the file name of the \
                        installer (msi). This does not change the name of the \
                        executable within the installer.")
                    .long("name")
                    .short('n')
                    .num_args(1))
                .arg(Arg::new("no-build")
                    .help("Skips building the release binary")
                    .long_help("The installer is created, but the 'cargo build \
                        --release' is not executed.")
                    .long("no-build")
                    .action(ArgAction::SetTrue))
                .arg(Arg::new("target-bin-dir")
                    .help("A path to the directory of binaries to include in the installer")
                    .long_help("Sets the CargoTargetBinDir variable that will be substituted \
                        into main.wxs. Use in conjunction with --no-build to fully handle builds.")
                    .long("target-bin-dir")
                    .num_args(1))
                .arg(Arg::new("no-capture")
                    .help("Displays all output from the builder, compiler, linker, and signer")
                    .long_help("By default, this subcommand captures, or hides, \
                        all output from the builder, compiler, linker, and signer \
                        for the binary and Windows installer, respectively. Use this \
                        flag to show the output.")
                    .long("nocapture")
                    .action(ArgAction::SetTrue))
                .arg(Arg::new("install")
                    .help("Runs the installer after creating it")
                    .long_help("Creates the installer and runs it after that.")
                    .long("install")
                    .action(ArgAction::SetTrue))
                .arg(Arg::new("output")
                    .help("A path to a destination file or an existing folder")
                    .long_help("Sets the destination file name and path for the \
                        created installer, or the destination folder for the \
                        installer with the default file name. If the path is to \
                        an existing folder or has a trailing slash (forward and \
                        backward), then the default installer file name will be \
                        used and the installer will be available in the folder \
                        after creation. Otherwise, this value overwrites the \
                        default file name and path for the installer. The \
                        default is to create an installer with the \
                        '<product-name>-<version>-<arch>.msi' file name in the \
                        'target\\wix' folder.")
                    .long("output")
                    .short('o')
                    .num_args(1))
                .arg(package.clone())
                .subcommand(Command::new("print")
                    .version(PKG_VERSION)
                    .about("Prints a template")
                    .long_about("Prints a template to stdout or a file. In the case \
                        of a license template, the output is in the Rich Text Format \
                        (RTF) and for a WiX Source file (wxs), the output is in XML. \
                        New GUIDs are generated for the 'UpgradeCode' and Path \
                        Component each time the 'WXS' template is printed. [values: \
                        Apache-2.0, GPL-3.0, MIT, WXS]")
                        .arg(Arg::new("TEMPLATE")
                        .help("A name of a template")
                        .long_help("This is required and values are case \
                            insensitive. [values: Apache-2.0, GPL-3.0, MIT, WXS]")
                        .hide_possible_values(true)
                        .value_parser(Template::possible_values().iter().map(String::as_str).collect::<Vec<&str>>())
                        .required(true)
                        .index(1))
                        .arg(Arg::new("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("The selected template will be printed to \
                            stdout or a file based on the field values in this \
                            manifest. The default is to use the manifest in the \
                            current working directory (cwd). An error occurs if a \
                            manifest is not found.")
                        .index(2))
                    .arg(banner)
                    .arg(binaries)
                    .arg(description)
                    .arg(dialog)
                    .arg(eula)
                    .arg(license)
                    .arg(manufacturer)
                    .arg(Arg::new("output")
                        .help("A path to a folder for generated files")
                        .long_help("Sets the destination for printing the \
                            template. The default is to print/write the rendered \
                            template to stdout. If the destination, a.k.a. file, \
                            does not exist, it will be created.")
                        .long("output")
                        .short('o')
                        .num_args(1))
                    .arg(owner)
                    .arg(package)
                    .arg(path_guid)
                    .arg(product_icon)
                    .arg(product_name.clone())
                    .arg(upgrade_guid)
                    .arg(url)
                    .arg(year)
                    .arg(verbose.clone()))
                .subcommand(Command::new("purge")
                    .version(PKG_VERSION)
                    .about("Deletes the 'target\\wix' and 'wix' folders")
                    .long_about("Deletes the 'target\\wix' and 'wix' folders if they \
                        exist. Use with caution!")
                    .arg(Arg::new("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("The 'target\\wix' and 'wix' folders that \
                            exists alongside the package's manifest will be removed. \
                            This is optional and the default is to use the current \
                            working directory (cwd).")
                        .index(1)))
                .subcommand(Command::new("sign")
                    .version(PKG_VERSION)
                    .about("Signs an installer")
                    .long_about("The Windows installer (msi) will be signed using the \
                        SignTool application available in the Windows 10 SDK. The \
                        signtool is invoked with the '/a' flag to automatically \
                        obtain an appropriate certificate from the Windows \
                        certificate manager. The default is to also use the Comodo \
                        timestamp server with the '/t' flag.")
                    .arg(Arg::new("bin-path")
                        .help("A path to the folder containing the 'signtool' application")
                        .long_help("The default is to use the PATH system environment \
                             variable to locate the application.")
                        .long("bin-path")
                        .short('b')
                        .num_args(1))
                    .arg(Arg::new("description")
                        .help("A string for the extended ACL dialog")
                        .long_help("The information for the extended text of \
                            the ACL dialog that appears. This will be appended to \
                            the product name and delimited by a dash, '-'. The \
                            default is to use the description from the package's \
                            manifest (Cargo.toml). This option will override the \
                            default.")
                        .long("description")
                        .short('d')
                        .num_args(1))
                    .arg(Arg::new("homepage")
                        .help("A URL for the product's homepage")
                        .long_help("This will be displayed in the ACL dialog.")
                        .long("homepage")
                        .short('u')
                        .num_args(1))
                    .arg(Arg::new("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("The installer located in the 'target\\wix' \
                            folder alongside this manifest will be signed based on \
                            the metadata within the manifest.")
                        .index(1))
                    .arg(Arg::new("installer")
                        .help("specify the installer to be signed")
                        .long_help("Specify the installer to be signed.")
                        .long("installer")
                        .short('i')
                        .num_args(1))
                    .arg(Arg::new("no-capture")
                        .help("Display output from the signer")
                        .long_help("By default, this subcommand captures, or \
                            hides, all output from the signer. Use this flag to \
                            show the output.")
                        .long("nocapture"))
                    .arg(product_name)
                    .arg(Arg::new("timestamp")
                        .help("An alias or URL to a timestamp server")
                        .long_help("Either an alias or URL can be used. Aliases \
                            are case-insensitive. [values: Comodo, Verisign]")
                        .short('t')
                        .long("timestamp")
                        .num_args(1))
                    .arg(verbose.clone()))
                .arg(verbose)
        ).get_matches();
    let matches = matches.subcommand_matches(SUBCOMMAND_NAME).unwrap();
    let verbosity = match matches.subcommand() {
        Some(("clean", m)) => m,
        Some(("init", m)) => m,
        Some(("print", m)) => m,
        Some(("purge", m)) => m,
        _ => matches,
    }
    .get_count("verbose");
    // Using the `Builder::new` instead of the `Builder::from_env` or `Builder::from_default_env`
    // skips reading the configuration from any environment variable, i.e. `RUST_LOG`. The log
    // level is later configured with the verbosity using the `filter` method. There are many
    // questions related to implementing support for environment variables:
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
    builder
        .format(|buf, record| {
            // This implementation for a format is copied from the default format implemented for the
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
        })
        .filter(
            Some("wix"),
            match verbosity {
                0 => LevelFilter::Warn,
                1 => LevelFilter::Info,
                2 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            },
        )
        .init();
    let result = match matches.subcommand() {
        Some(("clean", m)) => {
            let mut clean = clean::Builder::new();
            clean.input(m.get_one("INPUT").map(String::as_str));
            clean.build().run()
        }
        Some(("init", m)) => {
            let mut init = initialize::Builder::new();
            init.banner(m.get_one("banner").map(String::as_str));
            init.binaries(
                m.get_many::<String>("binaries")
                    .map(|v| v.map(String::as_str).collect()),
            );
            init.copyright_holder(m.get_one("owner").map(String::as_str));
            init.copyright_year(m.get_one("year").map(String::as_str));
            init.description(m.get_one("description").map(String::as_str));
            init.dialog(m.get_one("dialog").map(String::as_str));
            init.eula(m.get_one("eula").map(String::as_str));
            init.force(m.get_flag("force"));
            init.help_url(m.get_one("url").map(String::as_str));
            init.input(m.get_one("INPUT").map(String::as_str));
            init.license(m.get_one("license").map(String::as_str));
            init.manufacturer(m.get_one("manufacturer").map(String::as_str));
            init.output(m.get_one("output").map(String::as_str));
            init.package(m.get_one("package").map(String::as_str));
            init.path_guid(m.get_one("path-guid").map(String::as_str));
            init.product_icon(m.get_one("product-icon").map(String::as_str));
            init.product_name(m.get_one("product-name").map(String::as_str));
            init.upgrade_guid(m.get_one("upgrade-guid").map(String::as_str));
            init.build().run()
        }
        Some(("print", m)) => {
            let template = m
                .get_one::<String>("TEMPLATE")
                .unwrap()
                .parse::<Template>()
                .unwrap();
            match template {
                Template::Wxs => {
                    let mut print = print::wxs::Builder::new();
                    print.banner(m.get_one("banner").map(String::as_str));
                    print.binaries(
                        m.get_many("binaries")
                            .map(|v| v.map(String::as_str).collect()),
                    );
                    print.description(m.get_one("description").map(String::as_str));
                    print.dialog(m.get_one("dialog").map(String::as_str));
                    print.eula(m.get_one("eula").map(String::as_str));
                    print.help_url(m.get_one("url").map(String::as_str));
                    print.input(m.get_one("INPUT").map(String::as_str));
                    print.license(m.get_one("license").map(String::as_str));
                    print.manufacturer(m.get_one("manufacturer").map(String::as_str));
                    print.output(m.get_one("output").map(String::as_str));
                    print.package(m.get_one("package").map(String::as_str));
                    print.path_guid(m.get_one("path-guid").map(String::as_str));
                    print.product_icon(m.get_one("product-icon").map(String::as_str));
                    print.product_name(m.get_one("product-name").map(String::as_str));
                    print.upgrade_guid(m.get_one("upgrade-guid").map(String::as_str));
                    print.build().run()
                }
                t => {
                    let mut print = print::license::Builder::new();
                    print.copyright_holder(m.get_one("owner").map(String::as_str));
                    print.copyright_year(m.get_one("year").map(String::as_str));
                    print.input(m.get_one("INPUT").map(String::as_str));
                    print.output(m.get_one("output").map(String::as_str));
                    print.package(m.get_one("package").map(String::as_str));
                    print.build().run(&t)
                }
            }
        }
        Some(("purge", m)) => {
            let mut purge = purge::Builder::new();
            purge.input(m.get_one("INPUT").map(String::as_str));
            purge.build().run()
        }
        Some(("sign", m)) => {
            let mut sign = sign::Builder::new();
            sign.bin_path(m.get_one("bin-path").map(String::as_str));
            sign.capture_output(!m.get_flag("no-capture"));
            sign.description(m.get_one("description").map(String::as_str));
            sign.homepage(m.get_one("homepage").map(String::as_str));
            sign.input(m.get_one("INPUT").map(String::as_str));
            sign.installer(m.get_one("installer").map(String::as_str));
            sign.package(m.get_one("package").map(String::as_str));
            sign.product_name(m.get_one("product-name").map(String::as_str));
            sign.timestamp(m.get_one("timestamp").map(String::as_str));
            sign.build().run()
        }
        _ => {
            let mut create = create::Builder::new();
            create.bin_path(matches.get_one("bin-path").map(String::as_str));
            create.capture_output(!matches.get_flag("no-capture"));
            create.compiler_args(
                matches
                    .get_many("compiler-arg")
                    .map(|v| v.map(String::as_str).collect()),
            );
            create.culture(matches.get_one("culture").map(String::as_str));
            create.debug_build(matches.get_flag("debug-build"));
            create.profile(matches.get_one("profile").map(String::as_str));
            create.debug_name(matches.get_flag("debug-name"));
            create.includes(
                matches
                    .get_many("include")
                    .map(|v| v.map(String::as_str).collect()),
            );
            create.input(matches.get_one("INPUT").map(String::as_str));
            create.linker_args(
                matches
                    .get_many("linker-arg")
                    .map(|v| v.map(String::as_str).collect()),
            );
            create.locale(matches.get_one("locale").map(String::as_str));
            create.name(matches.get_one("name").map(String::as_str));
            create.no_build(matches.get_flag("no-build"));
            create.target_bin_dir(matches.get_one("target-bin-dir").map(String::as_str));
            create.install(matches.get_flag("install"));
            create.output(matches.get_one("output").map(String::as_str));
            create.version(matches.get_one("install-version").map(String::as_str));
            create.package(matches.get_one("package").map(String::as_str));
            create.target(matches.get_one("target").map(String::as_str));
            create.build().run()
        }
    };
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            {
                let mut stderr = StandardStream::stderr(ColorChoice::Auto);
                stderr
                    .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
                    .expect("Coloring stderr");
                write!(&mut stderr, "Error[{}] ({}): ", e.code(), e.as_str())
                    .expect("Write tag to stderr");
                // This prevents "leaking" the color settings to the console after the
                // sub-command/application has completed and ensures the message is not printed in
                // Red.
                //
                // See:
                //
                // - [Issue #47](https://github.com/volks73/cargo-wix/issues/47)
                // - [Issue #48](https://github.com/volks73/cargo-wix/issues/48).
                stderr
                    .reset()
                    .expect("Revert color settings after printing the tag");
                stderr
                    .set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false))
                    .expect("Coloring stderr");
                writeln!(&mut stderr, "{e}").expect("Write message to stderr");
                // This prevents "leaking" the color settings to the console after the
                // sub-command/application has completed.
                //
                // See:
                //
                // - [Issue #47](https://github.com/volks73/cargo-wix/issues/47)
                // - [Issue #48](https://github.com/volks73/cargo-wix/issues/48).
                stderr
                    .reset()
                    .expect("Revert color settings after printing the message");
            }
            std::process::exit(e.code());
        }
    }
}
