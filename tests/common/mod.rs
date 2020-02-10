#![allow(dead_code)]

extern crate assert_fs;
extern crate env_logger;
extern crate log;
extern crate sxd_document;
extern crate sxd_xpath;

use assert_fs::prelude::*;

use self::sxd_document::parser;
use self::sxd_xpath::{Context, Factory};

use assert_fs::TempDir;

use env_logger::fmt::Color as LogColor;
use env_logger::Builder;

use log::{Level, LevelFilter};

use std::env;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

pub const TARGET_NAME: &str = "target";

// Cannot use dashes. WiX Toolset only allows A-Z, a-z, digits, underscores (_), or periods (.)
// for attribute IDs.
pub const PACKAGE_NAME: &str = "cargowixtest";

pub const NO_CAPTURE_VAR_NAME: &str = "CARGO_WIX_TEST_NO_CAPTURE";

pub const PERSIST_VAR_NAME: &str = "CARGO_WIX_TEST_PERSIST";

pub const MISC_NAME: &str = "misc";

/// Create a new cargo project/package for a binary project in a temporary
/// directory.
///
/// This provides a unique, isolated Cargo project/package for testing. A
/// temporary directory is created. Then, a cargo project is initialized within
/// the temporary directory. The package/project is initialized without any
/// verison control system (vcs). The command that is ultimately executed to
/// create the cargo project in the temporary directory is:
///
/// ```
/// > cargo init --bin --quiet --vcs none --name cargowixtest "C:\Users\<username>\AppData\Local\Temp\cargo_wix_text_######"
/// ```
///
/// where `<username>` is replaced with the current logged in user for the
/// Windows Operating System (OS) and `######` is a hash ID that guarentees the
/// folder is unique.
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
pub fn create_test_package() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let cargo_init_status = Command::new("cargo")
        .arg("init")
        .arg("--bin")
        .arg("--quiet")
        .arg("--vcs")
        .arg("none")
        .arg("--name")
        .arg(PACKAGE_NAME)
        .arg(temp_dir.path())
        .status()
        .expect("Creation of test Cargo package");
    assert!(cargo_init_status.success());
    temp_dir.persist_if(env::var(PERSIST_VAR_NAME).is_ok())
}

/// Create a new cargo project/package for a project with multiple binaries in a
/// temporary directory. See the [create_test_package] function for more
/// information.
///
/// Following creation of the project, the manifest file (Cargo.toml) is
/// modified to include multiple `[[bin]]` sections for multiple binaries. The
/// original `main.rs` file that is created for the first binary is copied for
/// each of the other binaries. A total of three (3) binaries will be created
/// and added to the manifest file.
///
/// [create_test_package]: fn.create_test_package.html
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
///
/// It will also panic if it cannot modify the manifest file (Cargo.toml) or the
/// project layout for multiple binaries.
pub fn create_test_package_multiple_binaries() -> TempDir {
    let package = create_test_package();
    let package_manifest = package.child("Cargo.toml");
    let package_src = package.child("src");
    {
        let mut cargo_toml_handle = OpenOptions::new()
            .read(true)
            .append(true)
            .open(package_manifest.path())
            .unwrap();
        cargo_toml_handle
            .write_all(
                r#"[[bin]]
name = "main1"
path = "src/main1.rs"

[[bin]]
name = "main2"
path = "src/main2.rs"

[[bin]]
name = "main3"
path = "src/main3.rs"
"#
                .as_bytes(),
            )
            .unwrap();
    }
    let package_original_main = package_src.child("main.rs");
    fs::copy(
        package_original_main.path(),
        package_src.child("main1.rs").path(),
    )
    .unwrap();
    fs::copy(
        package_original_main.path(),
        package_src.child("main2.rs").path(),
    )
    .unwrap();
    fs::copy(
        package_original_main.path(),
        package_src.child("main3.rs").path(),
    )
    .unwrap();
    fs::remove_file(package_original_main.path()).unwrap();
    package
}

/// Create a new cargo project/package for a project with a
/// `[package.metadata.wix]` section.
///
/// Following creation of the project, the manifest file (Cargo.toml) is
/// modified to include a `[package.metadata.wix]` section.
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
///
/// It will also panic if it cannot modify the manifest file (Cargo.toml) or the
/// project layout for multiple binaries.
pub fn create_test_package_metadata() -> TempDir {
    let package = create_test_package();
    let package_manifest = package.child("Cargo.toml");
    let mut cargo_toml_handle = OpenOptions::new()
        .read(true)
        .append(true)
        .open(package_manifest.path())
        .unwrap();
    cargo_toml_handle
        .write_all(
            r#"[package.metadata.wix]
name = "Metadata"
version = "2.1.0"
compiler-args = ["-nologo", "-wx", "-arch", "x64"]
linker-args = ["-nologo"]
"#
            .as_bytes(),
        )
        .unwrap();
    package
}

/// Create a new cargo project/package for a project with multiple WXS files.
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
///
/// It will also panic if it cannot modify the manifest file (Cargo.toml) or the
/// project layout for multiple binaries.
///
/// This function will panic if the `wix` sub-folder could not be created.
pub fn create_test_package_multiple_wxs_sources() -> TempDir {
    let one_wxs = include_str!("one.wxs");
    let two_wxs = include_str!("two.wxs");
    let three_wxs = include_str!("three.wxs");
    let package = create_test_package();
    let mut misc_dir = package.path().join(MISC_NAME);
    fs::create_dir(&misc_dir).unwrap();
    misc_dir.push("one.wxs");
    let mut one_wxs_handle = File::create(&misc_dir).unwrap();
    one_wxs_handle.write_all(one_wxs.as_bytes()).unwrap();
    misc_dir.pop();
    misc_dir.push("two.wxs");
    let mut two_wxs_handle = File::create(&misc_dir).unwrap();
    two_wxs_handle.write_all(two_wxs.as_bytes()).unwrap();
    misc_dir.pop();
    misc_dir.push("three.wxs");
    let mut three_wxs_handle = File::create(&misc_dir).unwrap();
    three_wxs_handle.write_all(three_wxs.as_bytes()).unwrap();
    package
}

/// Evaluates an XPath expression for a WiX Source file.
///
/// This registers the WiX XML namespace with the `wix` prefix. So, XPath
/// expressions should use `/wix:Wix/` as the start and prefix all element/node
/// names with the `wix:` prefix. Note, attributes should _not_ have the `wix:`
/// prefix.
///
/// All values are currently returned as strings.
pub fn evaluate_xpath(wxs: &Path, xpath: &str) -> String {
    let mut wxs = File::open(wxs).expect("Open Wix Source file");
    let mut wxs_content = String::new();
    wxs.read_to_string(&mut wxs_content)
        .expect("Read WiX Source file");
    let wxs_package = parser::parse(&wxs_content).expect("Parsing WiX Source file");
    let wxs_document = wxs_package.as_document();
    let mut context = Context::new();
    context.set_namespace("wix", "http://schemas.microsoft.com/wix/2006/wi");
    let xpath = Factory::new().build(xpath).unwrap().unwrap();
    xpath
        .evaluate(&context, wxs_document.root())
        .unwrap()
        .string()
}

/// Initializes the logging for the integration tests.
///
/// When a test fails, it is useful to re-run the tests with logging statements
/// enabled to debug the failed test. This initializes the logging based on the
/// `CARGO_WIX_TEST_LOG` environment variable, which takes an integer as a
/// value. A `0` value, or not setting the environment variable, turns off
/// logging. Each increment of the integer value will increase the number of
/// statements that are logged up to 5 (Trace).
///
/// If the `CARGO_WIX_TEST_LOG` value is greater than zero (0), then log
/// statements will be emitted to the terminal/console regardless of the
/// `--nocapture` option for cargo tests. In other words, log statements are
/// *not* captured by cargo's testing framework with this implementation. Thus,
/// it is recommended to *not* activate logging if running all of the tests.
/// Logging should be done for isolated tests. Not capturing the log statements
/// by cargo's test framework keeps the formatting and coloring. There might be
/// a decrease in performance as well.
///
/// Log statements are formated the same as the verbosity format for the CLI.
///
/// # Examples
///
/// Enabling logging for tests in Powershell requires two commands and an
/// optional third command to undo:
///
/// ```powershell
/// PS C:\Path\to\Cargo\Wix> $env:CARGO_WIX_TEST_LOG=5
/// PS C:\Path\to\Cargo\Wix> cargo test
/// PS C:\Path\to\Cargo\Wix> Remove-Item Env:\CARGO_WIX_TEST_LOG
/// ```
///
/// This can be collapsed into a single line as:
///
/// ```powershell
/// PS C:\Path\to\Cargo\Wix> $env:CARGO_WIX_TEST_LOG=5; cargo test; Remove-Item Env:\CARGO_WIX_TEST_LOG
/// ```
///
/// But again, logging should only be activated for isolated tests to avoid
/// relatively large number of statements being written:
///
/// ```powershell
/// PS C:\Path\to\Cargo\Wix> $env:CARGO_WIX_TEST_LOG=5; cargo test <TEST_NAME>; Remove-Item Env:\CARGO_WIX_TEST_LOG
/// ```
///
/// where `<TEST_NAME>` is the name of a test, a.k.a. function name with the `#[test]` attribute.
pub fn init_logging() {
    let log_level = match std::env::var("CARGO_WIX_TEST_LOG") {
        Ok(level) => level
            .parse::<i32>()
            .expect("Integer for CARGO_WIX_TEST_LOG value"),
        Err(_) => 0,
    };
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
            match log_level {
                0 => LevelFilter::Off,
                1 => LevelFilter::Error,
                2 => LevelFilter::Warn,
                3 => LevelFilter::Info,
                4 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            },
        )
        .try_init()
        .ok();
}
