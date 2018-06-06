use RTF_FILE_EXTENSION;
use std::fmt;
use std::path::PathBuf;
use Template;

#[derive(Clone, Debug)]
pub enum Eula {
    CommandLine(PathBuf),
    Manifest(PathBuf),
    Generate(Template),
    Disabled,
}

impl fmt::Display for Eula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Eula::CommandLine(ref path) => path.display().fmt(f),
            Eula::Manifest(ref path) => path.display().fmt(f),
            Eula::Generate(..) => write!(f, "License.{}", RTF_FILE_EXTENSION),
            Eula::Disabled => write!(f, "Disabled"),
        }
    }
}

