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
//! - [Examples](#examples)
//! - [Features](#features)
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
//! ## Examples
//!
//! All of the following examples use the native Command Prompt (cmd.exe) for
//! the Windows OS; however, the [Developer Prompt] installed with the [VC Build
//! Tools] is recommended. The [git bash] terminal can also be used.
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
//! This will create a simple binary package named "example" without any verison
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
//!  WARN: An EULA was not specified at the command line, a RTF license file was not specified in the package manifest's (Cargo.toml) 'license-file' field, or the license ID from the pacakge manifest's 'license' field is not recognized. The license agreement dialog will be excluded from the installer. An EULA can be added manually to the generated WiX Source (wxs) file using a text editor.
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
//! the application with the _Release_ target and then build the installer. The
//! installer will be located in the `target\wix` folder.
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
//! The license agreement dialog for a WiX Toolset created installer must be in
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
//! `wix\main.wxs` file witout any warnings and uses the description and
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
//! The Windows SDK provides a signer (`signtool`) application for signing
//! installers. The application is installed in the `bin` folder of the Windows
//! SDK installation. The location of the `bin` folder varies depending on the
//! version. It is recommended to use the Developer Prompt to ensure the
//! `signtool` application is available. Signing an installer is optional.
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
//! properitary, license.
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
//! The [WixUIExtension] and [WixUtilExtension] are included in every execution
//! of the default _create_ cargo-wix subcommand, i.e. `cargo wix`. This is the
//! same as calling either the compiler (candle.exe) or the linker (light.exe)
//! with the `-ext WixUIExtension -ext WixUtilExtension` options. These two
//! extensions are commonly used to create installers when using the WiX
//! Toolset, so these are included by default. Additionally, the WixUIExtension
//! is used for the template WXS file.
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
//! culture = "Fr-Fr"
//! include = ["Path\to\WIX\Source\File\One.wxs", "Path\to\WIX\Source\File\Two.wxs"]
//! locale = "Path\to\WIX\Localization\File.wxl"
//! name = "example"
//! no-build = false
//! output = "Path\and\file\name\for\installer.msi"
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
//! be used for the cdefault _create_ subcommand is the same manifest that
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
//! Sets the path to a bitmap (.bmp) image file that will be displayed across
//! the top of each dialog in the installer. The banner image dimensions should
//! be 493 x 58 pixels.
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
//! This option is also avaliable for the `cargo wix sign` subcommand and can be
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
//! ### `-c,--culture`
//!
//! Available for the default _create_ (`cargo wix`) subcommand.
//!
//! Sets the culture for localization. Use with the [`-l,--locale`] option. See
//! the [WixUI localization documentation] for more information about acceptable
//! culture codes. The codes are case insensitive.
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
//
//! Sets the path to a bitmap (.bmp) image file that will be displayed to the
//! left on the first dialog of the installer. The dialog image dimensions
//! should be 493 x 312 pixels. The first dialog is known as the "Welcome"
//! dialog.
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
//! Overrides the first author in the `authors` field of the package's manifest
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
//! The default is to use the first author from the package's manifest
//! (Cargo.toml). This is only used when generating a license based on the value
//! of the `license` field in the package's manifest.
//!
//! ### `-p,--product-icon`
//!
//! Available for the _init_ (`cargo wix init`) and _print_ (`cargo wix print`)
//! subcommands.
//!
//! Sets the path to an image file that will be display as an icon in the
//! Add/Remove Programs (ARP) control panel for the installed application.
//!
//! ### `-P,--product-name`
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
//! insenstive.
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
//! Increases the level of logging statements based on occurance count of the
//! flag. The more `-v,--verbose` flags used, the more logging statements that
//! will be printed during execution of a subcommand. When combined with the
//! `--nocapture` flag, this is useful for debugging and testing.
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
//! [Cargo]: https://crates.io
//! [cargo subcommand]: https://github.com/rust-lang/cargo/wiki/Third-party-cargo-subcommands
//! [crates.io]: https://crates.io
//! [Developer Prompt]: https://msdn.microsoft.com/en-us/library/f35ctcxw.aspx
//! [`description`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [`documentation`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [documentation]: http://wixtoolset.org/documentation/
//! [git bash]: https://gitforwindows.org/
//! [`homepage`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [library]: ../wix/index.html
//! [LibreOffice]: https://www.libreoffice.org/
//! [`license`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [`license-file`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [Microsoft Office]: https://products.office.com/en-us/home
//! [Microsoft Notepad]: https://en.wikipedia.org/wiki/Microsoft_Notepad
//! [`repository`]: https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata
//! [Rich Text Format]: https://en.wikipedia.org/wiki/Rich_Text_Format
//! [Rust]: https://www.rust-lang.org
//! [sidecar]: https://en.wikipedia.org/wiki/Sidecar_file
//! [SignTool]: https://msdn.microsoft.com/en-us/library/windows/desktop/aa387764(v=vs.85).aspx
//! [`std::process::Command`]: https://doc.rust-lang.org/std/process/struct.Command.html
//! [`std::process::Command::status`]: https://doc.rust-lang.org/std/process/struct.Command.html#method.status
//! [TOML array]: https://github.com/toml-lang/toml#user-content-array
//! [tutorials]: https://www.firegiant.com/wix/tutorial/
//! [VC Build Tools]: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2017
//! [Windows 10 SDK]: https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk
//! [WixUIExtension]: https://wixtoolset.org//documentation/manual/v3/wixui/wixui_dialog_library.html
//! [WixUtilExtension]: https://wixtoolset.org/documentation/manual/v3/xsd/util/
//! [WixUI localization documentation]: http://wixtoolset.org/documentation/manual/v3/wixui/wixui_localization.html
//! [WiX Toolset]: http://wixtoolset.org
//! [WordPad]: https://en.wikipedia.org/wiki/WordPad
//! [WXS]: ../wix/enum.Template.html
//! [XML]: https://en.wikipedia.org/wiki/XML

#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate log;
extern crate termcolor;
extern crate wix;

use clap::{App, Arg, SubCommand};

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
use wix::{Cultures, Template, BINARY_FOLDER_NAME, WIX_PATH_KEY};

const SUBCOMMAND_NAME: &str = "wix";

fn main() {
    // The banner option for the `init` and `print` subcommands.
    let banner = Arg::with_name("banner")
        .help("A path to an image file (.bmp) for the installer's banner")
        .long_help(
            "Sets the path to a bitmap (.bmp) image file that will be \
             displayed across the top of each dialog in the installer. The banner \
             image dimensions should be 493 x 58 pixels.",
        )
        .long("banner")
        .short("b")
        .takes_value(true);
    // The binaries option for the `init` and `print` subcommands.
    let binaries = Arg::with_name("binaries")
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
        .multiple(true)
        .number_of_values(1)
        .short("B")
        .takes_value(true);
    // The description option for the `init` and `print` subcommands.
    let description = Arg::with_name("description")
        .help("A string describing the application in the installer")
        .long_help(
            "Overrides the 'description' field of the package's manifest \
             (Cargo.toml) as the description within the installer. Text with spaces \
             should be surrounded by double quotes.",
        )
        .long("description")
        .short("d")
        .takes_value(true);
    // The dialog option for the `init` and `print` subcommands.
    let dialog = Arg::with_name("dialog")
        .help("A path to an image file (.bmp) for the installer's welcome dialog")
        .long_help(
            "Sets the path to a bitmap (.bmp) image file that will be \
             displayed to the left on the first dialog of the installer. The dialog \
             image dimensions should be 493 x 312 pxiels.",
        )
        .long("dialog")
        .short("D")
        .takes_value(true);
    // The eula option for the `init` and `print` subcommands.
    let eula = Arg::with_name("eula")
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
        .short("e")
        .takes_value(true);
    // The license option for the `init` and `print` subcommands.
    let license = Arg::with_name("license")
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
        .short("l")
        .takes_value(true);
    // The url option for the `init` and `print` subcommands
    let url = Arg::with_name("url")
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
        .short("u")
        .takes_value(true);
    // The manufacturer option for the `init` and `print` subcommands
    let manufacturer = Arg::with_name("manufacturer")
        .help("A string for the Add/Remove Programs control panel's Manufacturer")
        .long_help(
            "Overrides the first author in the 'authors' field of the \
             package's manifest (Cargo.toml) as the manufacturer within the \
             installer. The manufacturer can be changed after initialization by \
             directly modifying the WiX Source file (wxs) with a text editor.",
        )
        .long("manufacturer")
        .short("m")
        .takes_value(true);
    // The owner option for the `init` and `print` subcommands
    let owner = Arg::with_name("owner")
        .help("A string for a generated license's copyright holder")
        .long_help(
            "Sets the copyright owner for the license during \
             initialization. The default is to use the first author from the \
             package's manifest (Cargo.toml). This is only used when generating a \
             license based on the value of the 'license' field in the package's \
             manifest.",
        )
        .long("owner")
        .short("O")
        .takes_value(true);
    // The product icon option for the `init` and `print` subcommands
    let product_icon = Arg::with_name("product-icon")
        .help("A path to an image file (.ico) for the Add/Remove Programs control panel")
        .long_help(
            "Sets the path to an image file that will be displayed as an \
             icon in the Add/Remove Programs (ARP) control panel for the installed \
             application.",
        )
        .long("product-icon")
        .short("p")
        .takes_value(true);
    // The product name option for the `init`, `print`, and `sign` subcommands
    let product_name = Arg::with_name("product-name")
        .help("A string for the Add/Remove Programs control panel's Name")
        .long_help(
            "Overrides the 'name' field of the package's manifest \
             (Cargo.toml) as the product name within the installer. The product name \
             can be changed after initialization by directly modifying the WiX Source \
             file (wxs) with a text editor.",
        )
        .long("product-name")
        .short("P")
        .takes_value(true);
    // The "global" verbose flag for all subcommands.
    let verbose = Arg::with_name("verbose")
        .help("The verbosity level for logging statements")
        .long_help(
            "Sets the level of verbosity. The higher the level of \
             verbosity, the more information that is printed and logged when the \
             application is executed. This flag can be specified multiple times, \
             where each occurrance increases the level and/or details written for \
             each statement.",
        )
        .long("verbose")
        .short("v")
        .multiple(true);
    let year = Arg::with_name("year")
        .help("A string for a generated license's copyright year")
        .long_help(
            "Sets the copyright year for the license during \
             initialization. The default is to use the current year. This is only \
             used if a license is generated from one of the supported licenses based \
             on the value of the 'license' field in the package's manifest \
             (Cargo.toml).",
        )
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
                         "A path to the WiX Toolset's '{}' folder",
                         BINARY_FOLDER_NAME))
                     .long_help(&format!(
                         "Specifies the path to the WiX Toolset's '{0}' folder, which should contain \
                         the needed 'candle.exe' and 'light.exe' applications. The default is to use \
                         the path specified with the {1} system environment variable that is created \
                         during the installation of the WiX Toolset. Failing the existence of the \
                         {1} system environment variable, the path specified in the PATH system \
                         environment variable is used. This is useful when working with multiple \
                         versions of the WiX Toolset.",
                         BINARY_FOLDER_NAME,
                         WIX_PATH_KEY))
                     .long("bin-path")
                     .short("b")
                     .takes_value(true))
                .subcommand(SubCommand::with_name("clean")
                    .version(crate_version!())
                    .about("Deletes the 'target\\wix' folder")
                    .long_about("Deletes the 'target\\wix' folder if it exists.")
                    .arg(Arg::with_name("INPUT")
                         .help("A path to a package's manifest (Cargo.toml)")
                         .long_help("The 'target\\wix' folder that exists \
                            alongside the package's manifest will be removed. This \
                            is optional and the default is to use the current \
                            working directory (cwd).")
                         .index(1)))
                .arg(Arg::with_name("culture")
                    .help("The culture code for localization")
                    .long_help("Sets the culture for localization. Use with the \
                       '-l,--locale' option. See the WixUI localization \
                       documentation for more information about acceptable culture \
                       codes. The codes are case insensitive.")
                    .long("culture")
                    .short("c")
                    .default_value(&default_culture)
                    .takes_value(true))
                .arg(Arg::with_name("include")
                    .help("Include an additional WiX Source (wxs) file")
                    .long_help("Includes a WiX source (wxs) file for a project, \
                        where the wxs file is not located in the default location, \
                        i.e. 'wix'. Use this option multiple times to include \
                        multiple wxs files.")
                    .long("include")
                    .multiple(true)
                    .short("I")
                    .takes_value(true))
                .subcommand(SubCommand::with_name("init")
                    .version(crate_version!())
                    .about("Generates files from a package's manifest (Cargo.toml) to create an installer")
                    .long_about("Uses a package's manifest (Cargo.toml) to generate a WiX Source (wxs) \
                           file that can be used immediately without modification to create an \
                           installer for the package. This will also generate an EULA in the Rich \
                           Text Format (RTF) if the 'license' field is specified with a supported \
                           license (GPL-3.0, Apache-2.0, or MIT). All generated files are placed in \
                           the 'wix' sub-folder by default.")
                    .arg(banner.clone())
                    .arg(binaries.clone())
                    .arg(description.clone())
                    .arg(dialog.clone())
                    .arg(eula.clone())
                    .arg(Arg::with_name("force")
                        .help("Overwrite existing WiX-related files")
                        .long_help("Overwrites any existing files that are \
                            generated during initialization. Use with caution.")
                        .long("force"))
                    .arg(Arg::with_name("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("If the '-o,--output' option is not used, \
                            then all output from initialization will be placed in a \
                            'wix' folder created alongside this path.")
                        .index(1))
                    .arg(license.clone())
                    .arg(manufacturer.clone())
                    .arg(Arg::with_name("output")
                        .help("A path to a folder for generated files")
                        .long_help("Sets the destination for all files \
                            generated during initialization. The default is to \
                            create a 'wix' folder within the project then generate \
                            all files in the 'wix' sub-folder.")
                        .long("output")
                        .short("o")
                        .takes_value(true))
                    .arg(owner.clone())
                    .arg(product_icon.clone())
                    .arg(product_name.clone())
                    .arg(url.clone())
                    .arg(verbose.clone())
                    .arg(year.clone()))
                .arg(Arg::with_name("INPUT")
                     .help("Path to a package's manifest (Cargo.toml) file.")
                     .long_help("If not value is provided, then the current \
                        working directory (CWD) will be used to locate a package's \
                        manifest. An error will occur if a manifest cannot be \
                        found. A relative or absolute path to a package's manifest \
                        (Cargo.toml) file can be used. Only one manifest is \
                        allowed. The creation of an installer will be relative to \
                        the specified manifest.")
                     .required(false)
                     .multiple(true))
                .arg(Arg::with_name("install-version")
                    .help("A string for the Add/Remove Programs control panel's version number")
                    .long_help("Overrides the version from the package's manifest \
                        (Cargo.toml), which is used for the installer name and \
                        appears in the Add/Remove Programs control panel.")
                    .long("install-version")
                    .short("i")
                    .takes_value(true))
                .arg(Arg::with_name("locale")
                    .help("A path to a WiX localization file (.wxl)")
                    .long_help("Sets the path to a WiX localization file (wxl) \
                        which contains localized strings. Use in conjunction with \
                        the '-c,--culture' option.")
                    .long("locale")
                    .short("l")
                    .takes_value(true))
                .arg(Arg::with_name("name")
                    .help("A string for the installer's product name")
                    .long_help("Overrides the 'name' field in the package's \
                        manifest (Cargo.toml), which is used in the file name of the \
                        installer (msi). This does not change the name of the \
                        executable within the installer.")
                    .long("name")
                    .short("n")
                    .takes_value(true))
                .arg(Arg::with_name("no-build")
                    .help("Skips building the release binary")
                    .long_help("The installer is created, but the 'cargo build \
                        --release' is not executed.")
                    .long("no-build"))
                .arg(Arg::with_name("no-capture")
                    .help("Displays all output from the builder, compiler, linker, and signer")
                    .long_help("By default, this subcommand captures, or hides, \
                        all output from the builder, compiler, linker, and signer \
                        for the binary and Windows installer, respectively. Use this \
                        flag to show the output.")
                    .long("nocapture"))
                .arg(Arg::with_name("output")
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
                    .short("o")
                    .takes_value(true))
                .subcommand(SubCommand::with_name("print")
                    .version(crate_version!())
                    .about("Prints a template")
                    .long_about("Prints a template to stdout or a file. In the case \
                        of a license template, the output is in the Rich Text Format \
                        (RTF) and for a WiX Source file (wxs), the output is in XML. \
                        New GUIDs are generated for the 'UpgradeCode' and Path \
                        Component each time the 'WXS' template is printed. [values: \
                        Apache-2.0, GPL-3.0, MIT, WXS]")
                    .arg(banner)
                    .arg(binaries)
                    .arg(description)
                    .arg(dialog)
                    .arg(eula)
                    .arg(Arg::with_name("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("The selected template will be printed to \
                            stdout or a file based on the field values in this \
                            manifest. The default is to use the manifest in the \
                            current working directory (cwd). An error occurs if a \
                            manifest is not found.")
                        .index(2))
                    .arg(license)
                    .arg(manufacturer)
                    .arg(Arg::with_name("output")
                        .help("A path to a folder for generated files")
                        .long_help("Sets the destination for printing the \
                            template. The default is to print/write the rendered \
                            template to stdout. If the destination, a.k.a. file, \
                            does not exist, it will be created.")
                        .long("output")
                        .short("o")
                        .takes_value(true))
                    .arg(owner)
                    .arg(product_icon)
                    .arg(product_name.clone())
                    .arg(Arg::with_name("TEMPLATE")
                        .help("A name of a template")
                        .long_help("This is required and values are case \
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
                    .about("Deletes the 'target\\wix' and 'wix' folders")
                    .long_about("Deletes the 'target\\wix' and 'wix' folders if they \
                        exist. Use with caution!")
                    .arg(Arg::with_name("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("The 'target\\wix' and 'wix' folders that \
                            exists alongside the package's manifest will be removed. \
                            This is optional and the default is to use the current \
                            working directory (cwd).")
                        .index(1)))
                .subcommand(SubCommand::with_name("sign")
                    .version(crate_version!())
                    .about("Signs an installer")
                    .long_about("The Windows installer (msi) will be signed using the \
                        SignTool application available in the Windows 10 SDK. The \
                        signtool is invoked with the '/a' flag to automatically \
                        obtain an appropriate certificate from the Windows \
                        certificate manager. The default is to also use the Comodo \
                        timestamp server with the '/t' flag.")
                    .arg(Arg::with_name("bin-path")
                        .help("A path to the folder containing the 'signtool' application")
                        .long_help("The default is to use the PATH system environment \
                             variable to locate the application.")
                        .long("bin-path")
                        .short("b")
                        .takes_value(true))
                    .arg(Arg::with_name("description")
                        .help("A string for the extended ACL dialog")
                        .long_help("The information for the extended text of \
                            the ACL dialog that appears. This will be appended to \
                            the product name and delimited by a dash, '-'. The \
                            default is to use the description from the package's \
                            manifest (Cargo.toml). This option will override the \
                            default.")
                        .long("description")
                        .short("d")
                        .takes_value(true))
                    .arg(Arg::with_name("homepage")
                        .help("A URL for the product's homepage")
                        .long_help("This will be displayed in the ACL dialog.")
                        .long("homepage")
                        .short("u")
                        .takes_value(true))
                    .arg(Arg::with_name("INPUT")
                        .help("A path to a package's manifest (Cargo.toml)")
                        .long_help("The installer located in the 'target\\wix' \
                            folder alongside this manifest will be signed based on \
                            the metadata within the manifest.")
                        .index(1))
                    .arg(Arg::with_name("no-capture")
                        .help("Display output from the signer")
                        .long_help("By default, this subcommand captures, or \
                            hides, all output from the signer. Use this flag to \
                            show the output.")
                        .long("nocapture"))
                    .arg(product_name)
                    .arg(Arg::with_name("timestamp")
                        .help("An alias or URL to a timestamp server")
                        .long_help("Either an alias or URL can be used. Aliases \
                            are case-insenstive. [values: Comodo, Verisign]")
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
    }
    .occurrences_of("verbose");
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
        ("clean", Some(m)) => {
            let mut clean = clean::Builder::new();
            clean.input(m.value_of("INPUT"));
            clean.build().run()
        }
        ("init", Some(m)) => {
            let mut init = initialize::Builder::new();
            init.banner(m.value_of("banner"));
            init.binaries(m.values_of("binaries").map(|v| v.collect()));
            init.copyright_holder(m.value_of("owner"));
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
        }
        ("print", Some(m)) => {
            let template = value_t!(m, "TEMPLATE", Template).unwrap();
            match template {
                Template::Wxs => {
                    let mut print = print::wxs::Builder::new();
                    print.banner(m.value_of("banner"));
                    print.binaries(m.values_of("binaries").map(|v| v.collect()));
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
                }
                t => {
                    let mut print = print::license::Builder::new();
                    print.copyright_holder(m.value_of("owner"));
                    print.copyright_year(m.value_of("year"));
                    print.input(m.value_of("INPUT"));
                    print.output(m.value_of("output"));
                    print.build().run(t)
                }
            }
        }
        ("purge", Some(m)) => {
            let mut purge = purge::Builder::new();
            purge.input(m.value_of("INPUT"));
            purge.build().run()
        }
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
        }
        _ => {
            let mut create = create::Builder::new();
            create.bin_path(matches.value_of("bin-path"));
            create.capture_output(!matches.is_present("no-capture"));
            create.culture(matches.value_of("culture"));
            create.includes(matches.values_of("include").map(|a| a.collect()));
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
                writeln!(&mut stderr, "{}", e).expect("Write message to stderr");
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
