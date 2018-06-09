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
use cargo_wix::purge;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const SUBCOMMAND_NAME: &str = "wix";

fn main() {
    // The "global" verbose flag for all subcommands.
    let verbose = Arg::with_name("verbose")
        .help("Sets the level of verbosity. The higher the level of verbosity, the more \
              information that is printed and logged when the application is executed. \
              This flag can be specified multiple times, where each occurrance \
              increases the level and/or details written for each statement.")
        .long("verbose")
        .short("v")
        .multiple(true);
    // The binary-name option for the `init` and `print` subcommands.
    let binary_name = Arg::with_name("binary-name")
        .help("Overrides the 'name' field of the 'bin' section of the package's \
              manifest (Cargo.toml) as the name of the executable within the \
              installer.")
        .long("binary-name")
        .short("b")
        .takes_value(true);
    // The description option for the `init` and `print` subcommands.
    let description = Arg::with_name("description")
        .help("Overrides the 'description' field of the package's manifest (Cargo.toml) \
              as the description within the installer. The description can be \
              changed after initialization by directly modifying the WiX Source file \
              (wxs) with a text editor.")
        .long("description")
        .short("d")
        .takes_value(true);
    // The eula option for the `init` and `print` subcommands.
    let eula = Arg::with_name("eula")
        .help("Specifies a RTF file to use as the EULA for the license agreement \
          dialog of the installer. The default is to disable the license \
          agreement dialog unless one of the supported licenses (GPL-3.0, \
          Apache-2.0, or MIT) is generated based on the value of the 'license' \
          field in the package's manifest (Cargo.toml). An EULA can be enabled \
          after initialization by directly modifying the WiX Source (wxs) file \
          with a text editor.")
        .long("eula")
        .short("E")
        .takes_value(true);
    // The license option for the `init` and `print` subcommands.
    let license = Arg::with_name("license")
        .help("Overrides the 'license-file' field of the package's manifest \
              (Cargo.toml). This requires the '--init' flag. If an appropriate license \
              file does not exist, cannot be found, or is not specified, then no \
              license file is included in the installer. A file containing the license, \
              such as a TXT, PDF, or RTF  file, can later be added by directly editing \
              the generated WiX Source file (wxs) in a text editor.")
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
        .short("U")
        .takes_value(true);
    // The holder option for the `init` and `print` subcommands
    let holder = Arg::with_name("holder")
        .help("Sets the copyright holder for the license during initialization. The \
              default is to use the first author from the package's manifest \
              (Cargo.toml). This is only used when generate a license based on the \
              value of the 'license' field in the package's manifest.")
        .long("holder")
        .short("H")
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
    // The product name option for the `init` and `print` subcommands
    let product_name = Arg::with_name("product-name")
        .help("Overrides the 'name' field of the package's manifest (Cargo.toml) as \
              the product name within the installer. The product name can be \
              changed after initialization by directly modifying the WiX Source \
              file (wxs) with a text editor.")
        .long("product-name")
        .short("p")
        .takes_value(true);
    let year = Arg::with_name("year")
         .help("Sets the copyright year for the license during initialization. The \
               default is to use the current year. This is only used if a license \
               is generated from one of the supported licenses based on the value \
               of the 'license' field in the package's manifest (Cargo.toml).")
         .short("Y")
         .long("year")
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
                    .about("Deletes the 'target\\wix' folder.")
                    .arg(Arg::with_name("INPUT")
                        .help("A package's manifest (Cargo.toml). The 'target\\wix' folder that \
                              exists alongside the package's manifest will be removed. This is \
                              optional and the default is to use the current working directory.")
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
                    .about("Uses a package's manifest (Cargo.toml) to generate a Wix Source (wxs) \
                           file that can be used immediately without modification to create an \
                           installer for the package. This will also generate an EULA in the Rich \
                           Text Format (RTF) if the 'license' field is specified with a supported \
                           license (GPL-3.0, Apache-2.0, or MIT). All generated files are placed in \
                           the 'wix' sub-folder by default.")
                    .arg(binary_name.clone())
                    .arg(description.clone())
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
                    .arg(license.clone()))
                    .arg(manufacturer.clone())
                    .arg(Arg::with_name("output")
                        .help("Sets the destination for all files generated during initialization. \
                              The default is to create a 'wix' folder within the project then \
                              generate all files in the 'wix' sub-folder.")
                        .long("output")
                        .short("o")
                        .takes_value(true))
                    .arg(product_name.clone())
                    .arg(url.clone())
                    .arg(verbose.clone())
                    .arg(year.clone())
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
                           Source file (wxs), the output is in XML. New GUIDS are generated for the \
                           'UpgradeCode' and Path Component each time the 'WXS' template is \
                           printed. [values: Apache-2.0, GPL-3.0, MIT, WXS]")
                    .arg(binary_name)
                    .arg(description)
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
                    .arg(verbose.clone()))
                .subcommand(SubCommand::with_name("purge")
                    .version(crate_version!())
                    .about("Deletes the 'target\\wix' and 'wix' folders. Use with caution.")
                    .arg(Arg::with_name("INPUT")
                        .help("A package's manifest (Cargo.toml). The 'target\\wix' and 'wix' \
                              folders that exists alongside the package's manifest will be removed. \
                              This is optional and the default is to use the current working \
                              directory.")
                        .index(1)))
                .arg(Arg::with_name("sign")
                    .help("The Windows installer (msi) will be signed using the SignTool \
                          application available in the Windows 10 SDK. The signtool is invoked with \
                          the '/a' flag to automatically obtain an appropriate certificate from the \
                          Windows certificate manager. The default is to also use the Comodo \
                          timestamp server with the '/t' flag.")
                    .short("s")
                    .long("sign"))
                .arg(Arg::with_name("sign-path")
                    .help("Specifies the path to the folder containg the 'signtool' application. \
                          The default is to use the PATH system environment variable to locate the \
                          application. This can only be used with the '-s,--sign' flag.")
                    .long("sign-path")
                    .short("S")
                    .takes_value(true)
                    .requires("sign"))
                .arg(Arg::with_name("timestamp")
                    .help("The alias or URL for the timestamp server used with the 'signtool' to \
                          sign the installer. This can only be used with the '-s,--sign' flag. \
                          Either an alias can be used or a URL. Available case-insensitive aliases \
                          include: Comodo and Verisign.")
                    .short("t")
                    .long("timestamp")
                    .takes_value(true)
                    .requires("sign"))
                .arg(verbose)
        ).get_matches();
    let matches = matches.subcommand_matches(SUBCOMMAND_NAME).unwrap();
    let verbosity = match matches.subcommand() {
        ("init", Some(m)) => m,
        ("print", Some(m)) => m,
        _ => matches,
    }.occurrences_of("verbose");
    // Using the `Builder::new` instead of the `Builder::from_env` or `Builder::from_default_env`
    // skips reading the configuration from any environment variable, i.e. `RUST_LOG`. The log
    // level is later configured with the verbosity using the `filter` method. There are many
    // questions related to implementing  support for environment variables: 
    //
    // 1. What should the environment variable be called, WIX_LOG, CARGO_WIX_LOG, CARGO_LOG, etc.?
    //    WIX_LOG might conflict with a system variable that is used for the Wix Toolset. CARGO_LOG
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
    let wix = cargo_wix::Wix::new()
        .bin_path(matches.value_of("bin-path"))
        .binary_name(matches.value_of("binary-name"))
        .capture_output(!matches.is_present("no-capture"))
        .copyright_holder(matches.value_of("holder"))
        .copyright_year(matches.value_of("year"))
        .culture(value_t!(matches, "culture", Cultures).unwrap_or_else(|e| e.exit()))
        .description(matches.value_of("description"))
        .input(matches.value_of("INPUT"))
        .license_file(matches.value_of("license"))
        .locale(matches.value_of("locale"))
        .manufacturer(matches.value_of("manufacturer"))
        .no_build(matches.is_present("no-build"))
        .output(matches.value_of("output"))
        .product_name(matches.value_of("product-name"))
        .sign(matches.is_present("sign"))
        .sign_path(matches.value_of("sign-path"))
        .timestamp(matches.value_of("timestamp"));
    // TODO: Change to use the `subcommand_name` method and a match expression once all of the
    // flags have been converted to subcommands.
    let result = if matches.is_present("init") {
        let m = matches.subcommand_matches("init").unwrap();
        let mut init = initialize::Builder::new();
        init.binary_name(m.value_of("binary-name"));
        init.copyright_holder(m.value_of("holder"));
        init.copyright_year(m.value_of("year"));
        init.description(m.value_of("description"));
        init.eula(m.value_of("eula"));
        init.force(m.is_present("force"));
        init.help_url(m.value_of("url"));
        init.input(m.value_of("INPUT"));
        init.license(m.value_of("license"));
        init.manufacturer(m.value_of("manufacturer"));
        init.output(m.value_of("output"));
        init.product_name(m.value_of("product-name"));
        init.build().run()    
    } else if matches.is_present("clean") {
        let m = matches.subcommand_matches("clean").unwrap();
        let mut clean = clean::Builder::new();
        clean.input(m.value_of("INPUT"));
        clean.build().run()
    } else if matches.is_present("purge") {
        let m = matches.subcommand_matches("purge").unwrap();
        let mut purge = purge::Builder::new();
        purge.input(m.value_of("INPUT"));
        purge.build().run()
    } else if matches.is_present("print-template") {
        wix.print_template(value_t!(matches, "print-template", Template).unwrap())
    } else {
        let mut create = create::Builder::new();
        create.bin_path(matches.value_of("bin-path"));
        create.capture_output(matches.is_present("capture-output"));
        create.culture(value_t!(matches, "culture", Cultures).unwrap_or_else(|e| e.exit()));
        create.input(matches.value_of("INPUT"));
        create.locale(matches.value_of("locale"));
        create.name(matches.value_of("name"));
        create.no_build(matches.is_present("no-build"));
        create.output(matches.value_of("output"));
        create.version(matches.value_of("install-version"));
        create.build().run()
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

