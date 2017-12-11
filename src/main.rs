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

extern crate ansi_term;
extern crate atty;
extern crate cargo_wix;
#[macro_use] extern crate clap;
extern crate loggerv;

use ansi_term::Colour;
use clap::{App, Arg, SubCommand};
use std::error::Error;
use std::io::Write;
use cargo_wix::Template;

const SUBCOMMAND_NAME: &str = "wix";
const ERROR_COLOR: Colour = Colour::Fixed(9); // Bright red

fn main() {
    // Based on documentation for the ansi_term crate, Windows 10 supports ANSI escape characters,
    // but it must be enabled first. The ansi_term crate provides a function for enabling ANSI
    // support in Windows, but it is conditionally compiled and only exists for Windows builds. To
    // avoid build errors on non-windows platforms, a cfg guard should be put in place.
    if atty::is(atty::Stream::Stdout) {
        #[cfg(windows)] ansi_term::enable_ansi_support().expect("Enable ANSI support on Windows");
    }
    let matches = App::new(crate_name!())
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_NAME)
                .version(crate_version!())
                .about(crate_description!())
                .arg(Arg::with_name("bin-path")
                    .help("Specifies the path to the WiX Toolset's 'bin' folder, which should contain the needed 'candle.exe' and 'light.exe' applications. The default is to use the PATH system environment variable.")
                    .long("bin-path")
                    .short("B")
                    .takes_value(true))
                .arg(Arg::with_name("binary-name")
                    .help("Overrides the 'name' field of the bin section of the package's manifest (Cargo.toml) as the name of the executable within the installer.")
                    .long("binary-name")
                    .short("b")
                    .takes_value(true))
                .arg(Arg::with_name("clean")
                    .help("Deletes the 'target\\wix' folder.")
                    .long("clean")
                    .conflicts_with("init")
                    .conflicts_with("sign")
                    .conflicts_with("print-template"))
                .arg(Arg::with_name("description")
                    .help("Overrides the 'description' field of the package's manifest (Cargo.toml) as the description within the installer.")
                    .long("description")
                    .short("d")
                    .takes_value(true))
                .arg(Arg::with_name("force")
                    .help("Overwrites any existing WiX Source files when using the '--init' flag. Use with caution.")
                    .long("force")
                    .requires("init"))
                .arg(Arg::with_name("holder")
                    .help("Sets the copyright holder for the license during initialization. The default is to use the first author from the package's manifest (Cargo.toml). This requires the '--init' flag.")
                    .long("holder")
                    .short("H")
                    .takes_value(true)
                    .requires("init"))
                .arg(Arg::with_name("init")
                    .help("Initializes the package to be used with this subcommand. This creates a 'wix` sub-folder within the root folder of the package and creates a 'main.wxs' WiX Source (wxs) file within the 'wix' sub-folder from the embedded template. The 'wix\\main.wxs' file that is created can immediately be used with this subcommand without modification to create an installer for the project.")
                    .long("init")
                    .conflicts_with("clean")
                    .conflicts_with("purge")
                    .conflicts_with("print-template"))
                .arg(Arg::with_name("license")
                    .help("Overrides the 'license-file' field of the package's manfiest (Cargo.toml) as the file to be converted to the 'License.txt' file that is added to the install location along side the 'bin' folder by the installer.")
                    .long("license")
                    .short("l")
                    .takes_value(true))
                .arg(Arg::with_name("manufacturer")
                    .help("Overrides the first author in the 'authors' field of the package's manifest (Cargo.toml) as the manufacturer within the installer.")
                    .long("manufacturer")
                    .short("m")
                    .takes_value(true))
                .arg(Arg::with_name("no-capture")
                    .help("By default, this subcommand captures, or hides, all output from the builder, compiler, linker, and signer for the binary and Windows installer, respectively. Use this flag to show the output.")
                    .long("nocapture"))
                .arg(Arg::with_name("print-template")
                     .help("Prints a template to stdout. In the case of a license template, the output is in the Rich Text Format (RTF) and for a WiX Source file, the output is in XML. New GUIDS are generated for the 'UpgradeCode' and Path Component each time the 'WXS' template is printed. Values are case insensitive. [values: Apache-2.0, GPL-3.0, MIT, WXS]")
                     .long("print-template")
                     .hide_possible_values(true)
                     .possible_values(&Template::possible_values())
                     .takes_value(true)
                     .conflicts_with("init")
                     .conflicts_with("clean")
                     .conflicts_with("purge"))
                .arg(Arg::with_name("product-name")
                    .help("Overrides the 'name' field of the package's manifest (Cargo.toml) as the product name within the installer.")
                    .long("product-name")
                    .short("p")
                    .takes_value(true))
                .arg(Arg::with_name("purge")
                    .help("Deletes the 'target\\wix' and 'wix' folders. Use with caution.")
                    .long("purge")
                    .conflicts_with("init")
                    .conflicts_with("sign")
                    .conflicts_with("clean")
                    .conflicts_with("print-template"))
                .arg(Arg::with_name("sign")
                    .help("The Windows installer (msi) will be signed using the SignTool application available in the Windows 10 SDK. The signtool is invoked with the '/a' flag to automatically obtain an appropriate certificate from the Windows certificate manager. The default is to also use the Comodo timestamp server with the '/t' flag.")
                    .short("s")
                    .long("sign"))
                .arg(Arg::with_name("sign-path")
                    .help("Specifies the path to the folder containg the 'signtool' application. The default is to use the PATH system environment variable to locate the application. This can only be used with the '-s,--sign' flag.")
                    .long("sign-path")
                    .short("S")
                    .takes_value(true)
                    .requires("sign"))
                .arg(Arg::with_name("timestamp")
                    .help("The alias or URL for the timestamp server used with the 'signtool' to sign the installer. This can only be used with the '-s,--sign' flag. Either an alias can be used or a URL. Available case-insensitive aliases include: Comodo and Verisign.")
                    .short("t")
                    .long("timestamp")
                    .takes_value(true)
                    .requires("sign"))
                .arg(Arg::with_name("year")
                     .help("Sets the copyright year for the license during initialization. The default is to use the current year. This requires the '--init' flag.")
                     .short("Y")
                     .long("year")
                     .takes_value(true)
                     .requires("init"))
                .arg(Arg::with_name("verbose")
                    .help("Sets the level of verbosity. The higher the level of verbosity, the more information that is printed and logged when the application is executed. This flag can be specified multiple times, where each occurrance increases the level and/or details written for each statement.")
                    .long("verbose")
                    .short("v")
                    .multiple(true))
                .arg(Arg::with_name("INPUT")
                    .help("A WiX Source (wxs) file. The default is to use the 'wix\\main.wxs' file.")
                    .index(1))
        ).get_matches();
    let matches = matches.subcommand_matches(SUBCOMMAND_NAME).unwrap();
    let verbosity = matches.occurrences_of("verbose");
    loggerv::Logger::new()
        .verbosity(verbosity)
        .line_numbers(verbosity > 3)
        .module_path(false)
        .level(true)
        .init()
        .expect("logger to initiate");
    let wix = cargo_wix::Wix::new()
        .bin_path(matches.value_of("bin-path"))
        .binary_name(matches.value_of("binary-name"))
        .capture_output(!matches.is_present("no-capture"))
        .copyright_holder(matches.value_of("holder"))
        .copyright_year(matches.value_of("year"))
        .description(matches.value_of("description"))
        .input(matches.value_of("INPUT"))
        .license_file(matches.value_of("license"))
        .manufacturer(matches.value_of("manufacturer"))
        .product_name(matches.value_of("product-name"))
        .sign(matches.is_present("sign"))
        .sign_path(matches.value_of("sign-path"))
        .timestamp(matches.value_of("timestamp"));
    let result = if matches.is_present("init") {
        wix.init(matches.is_present("force"))
    } else if matches.is_present("clean") {
        cargo_wix::clean()
    } else if matches.is_present("purge") {
        cargo_wix::purge()
    } else if matches.is_present("print-template") {
        wix.print_template(value_t!(matches, "print-template", Template).unwrap())
    } else {
        wix.run()
    };
    match result {
        Ok(_) => {
            std::process::exit(0);
        },
        Err(e) => {
            let mut tag = format!("Error[{}] ({})", e.code(), e.description());
            if atty::is(atty::Stream::Stderr) {
                tag = ERROR_COLOR.paint(tag).to_string()
            }
            writeln!(&mut std::io::stderr(), "{}: {}", tag, e)
                .expect("Writing to stderr");
            std::process::exit(e.code());
        }
    }
}

