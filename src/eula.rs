use Error;
use Result;
use RTF_FILE_EXTENSION;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use Template;
use toml::Value;

#[derive(Clone, Debug)]
pub enum Eula {
    CommandLine(PathBuf),
    Manifest(PathBuf),
    Generate(Template),
    Disabled,
}

impl Eula {
    pub fn new(p: Option<&PathBuf>, manifest: &Value) -> Result<Self> {
        if let Some(ref path) = p {
            if path.exists() {
                trace!("The '{}' path from the command line for the EULA exists", path.display());
                Ok(Eula::CommandLine(path.into()))
            } else {
                Err(Error::Generic(format!(
                    "The '{}' path from the command line for the EULA does not exist", 
                    path.display()
                )))
            }
        } else {
            Eula::from_manifest(&manifest)
        }
    }

    pub fn from_manifest(manifest: &Value) -> Result<Self> {
        if let Some(license_file_path) = manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("license-file"))
            .and_then(|l| l.as_str())
            .map(PathBuf::from) {
            trace!("The 'license-file' field is specified in the package's manifest (Cargo.toml)");
            debug!("license_file_path = {:?}", license_file_path);
            if license_file_path.extension().and_then(|s| s.to_str()) == Some(RTF_FILE_EXTENSION) {
                trace!("The '{}' path from the 'license-file' field in the package's \
                       manifest (Cargo.toml) has a RTF file extension.",
                       license_file_path.display()); 
                if license_file_path.exists() {
                    trace!("The '{}' path from the 'license-file' field of the package's \
                           manifest (Cargo.toml) exists and has a RTF file extension.",
                           license_file_path.exists());
                    Ok(Eula::Manifest(license_file_path.into()))
                } else {
                    Err(Error::Generic(format!(
                        "The '{}' file to be used for the EULA specified in the package's \
                        manifest (Cargo.toml) using the 'license-file' field does not exist.", 
                        license_file_path.display()
                    )))
                }
            } else {
                trace!("The '{}' path from the 'license-file' field in the package's \
                       manifest (Cargo.toml) exists but it does not have a RTF file \
                       extension.",
                       license_file_path.display());
                Ok(Eula::Disabled)
            }
        } else {
            if let Some(license_name) = manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license"))
                .and_then(|n| n.as_str()) {
                trace!("The 'license' field is specified in the package's manifest (Cargo.toml)");
                debug!("license_name = {:?}", license_name);
                if let Ok(template) = Template::from_str(license_name) {
                    trace!("An embedded template for the '{}' license from the package's \
                           manifest (Cargo.toml) exists.", license_name);
                    Ok(Eula::Generate(template))
                } else {
                    trace!("The '{}' license from the package's manifest (Cargo.toml) is \
                           unknown or an embedded template does not exist for it", license_name);
                    Ok(Eula::Disabled)
                }
            } else {
                trace!("The 'license' field is not specified in the package's manifest (Cargo.toml)");
                Ok(Eula::Disabled)
            }
        }
    }
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

