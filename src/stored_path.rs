//! Utilities for working with paths that need to be stored in files in a cross-platform way.
//!
//! Big picture: work with paths however you want, as long as any path that gets written to
//! a file (e.g. via a template) always gets converted into a [`StoredPathBuf`][] first,
//! forcing all the platform-specific differences to get normalized out.
//!
//! Although both std and camino have excellent support for working with paths, they're both
//! designed to have host-platform-specific behaviour (which is good/correct) that makes them
//! unsuitable for working with paths that need to stored in persistent cross-platform
//! ways -- like writing them to files. The wxs files we generate contain paths to other files,
//! and we want to reliably produce the same wxs file on all machines, so we need something else!
//!
//! Most notably, std and camino handle path separators differently on different platforms:
//!
//! * valid separtors:
//!   * on windows, `\` and `/` are both valid path separators
//!   * on unix, only `/` is a valid separator, and `\` can appear in file names
//! * auto separators:
//!   * on windows, using APIs like `join` will use a `\` separator
//!   * on unix, using APIs like `join` will use a `/` separator
//!
//! Since cargo-wix is fundamentally concerned with producing things that work on windows, we
//! can comfortably force unix to be "like windows" to normalize the behaviour. This normalization
//! is handled by the [`StoredPath`][] and [`StoredPathBuf`][] types.
//!
//! These types have two flavours of entry-point:
//!
//! * When making a StoredPathBuf from a String, the input is assumed to be user-provided and is forwarded
//!   verbatim without any normalization. Windows is permissive of both kinds of path separator,
//!   so we never need to "fix" things up.
//!
//! * When making a StoredPathBuf from a Path or Utf8Path, the input is assumed to be tainted with
//!   platform-specific behaviour, and all path separators are normalized to `\`. The net effect is
//!   that on windows StoredPathBuf usually doesn't do anything, but on unix it forces
//!   many `/`'s to `\`'s. See [`StoredPathBuf::from_utf8_path`][] for the implementation.
//!
//! A StoredPath is not intended for doing actual i/o, and as such does not expose a way
//! for it to be turned back into a "real" path, and does not expose APIs like `exists`.
//! However it is useful/necessary to be able to ask questions like "is this file an RTF",
//! so we do need to implement basic path parsing functions like `file_name` and `extension`.
//!
//! Notably [`StoredPath::file_name`][]` considers both `\` and `/` to be path separators.
//! Ideally it should behave identically to std/camino on windows, making all platforms
//! behave like windows.

use std::{fmt, path::Path};

use camino::{Utf8Component, Utf8Path};

/// A PathBuf that will be in the output of print (and therefore saved to disk)
///
/// A proper PathBuf should not be used for that, as we don't want to introduce
/// platform-specific separators.
///
/// This type intentionally lacks some path functionality like `.exists()` because
/// we don't want to be using it for *actual* path stuff, only for writing it to output.
/// Most StoredPathBufs are just user-provided strings with no processing applied.
///
/// However sometimes we are forced to handle a PathBuf because of things like
/// cargo-metadata, in which case [`StoredPathBuf::from_utf8_path`][] or
/// [`StoredPathBuf::from_std_path`][] will convert the path to the windows path style.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoredPathBuf(String);

/// A borrowed [`StoredPathBuf`][]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoredPath(str);

impl StoredPathBuf {
    /// Make a new StoredPathBuf from a String
    pub fn new(v: String) -> Self {
        Self(v)
    }

    /// Make a new StoredPath from a OS-specific path
    ///
    /// This breaks the path into its components and rewrites the slashes to `\`.
    ///
    /// Generally you should avoid this and just try to preserve the user input,
    /// but it's required when using things like the output of cargo-metadata.
    pub fn from_std_path(path: &Path) -> Option<Self> {
        Utf8Path::from_path(path).map(Self::from_utf8_path)
    }

    /// Make a new StoredPath from a OS-specific Utf8Path
    ///
    /// This breaks the path into its components and rewrites the slashes to `\`.
    ///
    /// Generally you should avoid this and just try to preserve the user input,
    /// but it's required when using things like the output of cargo-metadata.
    ///
    /// Also note that this does some handling of absolute paths, but that those
    /// are kind of nonsensical to store longterm. Still, the user can hand them
    /// to us, and we have to do our best to deal with it. Mostly this just comes
    /// up in test code.
    pub fn from_utf8_path(path: &Utf8Path) -> Self {
        // The main quirk of this code is handling absolute paths.
        // `C:\a\b\c` is given to us as `["C:", "\", "a", "b", "c"]`
        let mut result = String::new();
        let mut multipart = false;
        for component in path.components() {
            // Add separator for every part but the first,
            // ignoring root prefixes like "C:\" and "/" which
            // provide their own separators.
            if multipart {
                result.push('\\');
            }
            let part = match component {
                // "C:"
                Utf8Component::Prefix(prefix) => prefix.as_str(),
                // the root slash
                // (either the one at the end of "C:\" or the one at the start of "/a/b/c")
                Utf8Component::RootDir => "\\",
                other => {
                    // Ok we're passed the weird root stuff, now should add separators
                    multipart = true;
                    other.as_str()
                }
            };
            result.push_str(part);
        }
        Self(result)
    }
}

impl StoredPath {
    /// Make a new StoredPath from a str
    pub fn new(v: &str) -> &Self {
        // SAFETY: this is the idiomatic pattern for converting between newtyped slices.
        // See the impl of std::str::from_utf8_unchecked for an example.
        unsafe { std::mem::transmute(v) }
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        self
    }

    /// Extracts the extension part of the [`self.file_name`][]
    pub fn extension(&self) -> Option<&str> {
        self.stem_and_extension().1
    }

    /// Extracts the stem (non-extension) part of the [`self.file_name`][].
    pub fn file_stem(&self) -> Option<&str> {
        self.stem_and_extension().0
    }

    // Implements `stem` and `extension` together based on the semantics defined by camino/std
    fn stem_and_extension(&self) -> (Option<&str>, Option<&str>) {
        let Some(name) = self.file_name() else {
            // both: None if there's no file name
            return (None, None);
        };
        if let Some((stem, extension)) = name.rsplit_once('.') {
            if stem.is_empty() {
                // stem: The entire file name if the file name begins with '.' and has no other '.'s within
                // extension: None, if the file name begins with '.' and has no other '.'s within;
                (Some(name), None)
            } else {
                // stem: Otherwise, the portion of the file name before the final '.'
                // extension: Otherwise, the portion of the file name after the final '.'
                (Some(stem), Some(extension))
            }
        } else {
            // stem: The entire file name if there is no embedded '.'
            // extension: None, if there is no embedded '.'
            (Some(name), None)
        }
    }

    /// Returns the final component of the path, if there is one.
    pub fn file_name(&self) -> Option<&str> {
        let mut path = self.as_str();

        // First repeatedly pop trailing slashes off to get to the actual path
        // Also pop trailing /. as this is a no-op.
        // trailing .. is however treated as opaque!
        while let Some(prefix) = path
            .strip_suffix('\\')
            .or_else(|| path.strip_suffix('/'))
            .or_else(|| path.strip_suffix("/."))
            .or_else(|| path.strip_suffix("\\."))
        {
            path = prefix;
        }

        // Look for either path separator (windows file names shouldn't include either,
        // so even though unix file names can have `\`, it won't work right on the actual
        // platform that matters, so we can ignore that consideration.)
        let name1 = path.rsplit_once('\\').map(|(_, name)| name);
        let name2 = path.rsplit_once('/').map(|(_, name)| name);

        // Decide which parse to use
        let name = match (name1, name2) {
            // trivial case, only one gave an answer
            (Some(name), None) | (None, Some(name)) => name,
            // Both matched, use whichever one came last (shortest file name)
            (Some(name1), Some(name2)) => {
                if name1.len() < name2.len() {
                    name1
                } else {
                    name2
                }
            }
            // No slashes left, the entire path is just the filename
            (None, None) => path,
        };

        // Several special "names" are in fact not names at all:
        if name.is_empty() || name == "." || name == ".." {
            None
        } else {
            Some(name)
        }
    }
}

impl std::ops::Deref for StoredPathBuf {
    type Target = StoredPath;
    fn deref(&self) -> &Self::Target {
        StoredPath::new(&self.0)
    }
}
impl std::ops::Deref for StoredPath {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::borrow::Borrow<StoredPath> for StoredPathBuf {
    fn borrow(&self) -> &StoredPath {
        self
    }
}
impl std::borrow::ToOwned for StoredPath {
    type Owned = StoredPathBuf;

    fn to_owned(&self) -> StoredPathBuf {
        StoredPathBuf::new(self.0.to_owned())
    }
}
impl std::convert::From<String> for StoredPathBuf {
    fn from(v: String) -> Self {
        Self::new(v)
    }
}
impl std::convert::From<StoredPathBuf> for String {
    fn from(v: StoredPathBuf) -> Self {
        v.0
    }
}
impl<'a> std::convert::From<&'a StoredPathBuf> for String {
    fn from(v: &'a StoredPathBuf) -> Self {
        v.0.clone()
    }
}
impl<'a> std::convert::From<&'a str> for StoredPathBuf {
    fn from(v: &'a str) -> Self {
        StoredPath::new(v).to_owned()
    }
}
impl std::fmt::Debug for StoredPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}
impl std::fmt::Display for StoredPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}
impl std::fmt::Debug for StoredPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}
impl std::fmt::Display for StoredPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

#[cfg(test)]
mod test {
    use super::StoredPathBuf;
    use camino::Utf8Path;

    #[test]
    fn absolute_windows_path_conversion() {
        // Absolute native windows format
        const INPUT: &str = "C:\\Users\\test\\AppData\\Local\\Temp\\.tmpMh0Mxg\\Example.tar.gz";
        let path = StoredPathBuf::from_utf8_path(Utf8Path::new(INPUT));
        assert_eq!(path.as_str(), INPUT);
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn verbatim_absolute_windows_path_conversion() {
        // Absolute native windows format (verbatim style)
        const INPUT: &str =
            "\\\\?\\C:\\Users\\test\\AppData\\Local\\Temp\\.tmpMh0Mxg\\Example.tar.gz";
        let path = StoredPathBuf::from_utf8_path(Utf8Path::new(INPUT));
        assert_eq!(path.as_str(), INPUT);
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn relative_windows_path_conversion() {
        // relative native windows format
        const INPUT: &str = "resource\\Example.tar.gz";
        let path = StoredPathBuf::from_utf8_path(Utf8Path::new(INPUT));
        assert_eq!(path.as_str(), INPUT);
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn absolute_unix_path_conversion() {
        // absolute native unix format
        const INPUT: &str = "/users/home/test/Example.tar.gz";
        let path = StoredPathBuf::from_utf8_path(Utf8Path::new(INPUT));
        assert_eq!(path.as_str(), "\\users\\home\\test\\Example.tar.gz");
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn relative_unix_path_conversion() {
        // relative native unix format
        const INPUT: &str = "resource/Example.tar.gz";
        let path = StoredPathBuf::from_utf8_path(Utf8Path::new(INPUT));
        assert_eq!(path.as_str(), "resource\\Example.tar.gz");
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn mixed_path_conversion1() {
        // a mix of both formats (natural when combining user input with OS input)
        const INPUT: &str = "resource/blah\\Example.tar.gz";
        let path = StoredPathBuf::from_utf8_path(Utf8Path::new(INPUT));
        assert_eq!(path.as_str(), "resource\\blah\\Example.tar.gz");
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn mixed_path_conversion2() {
        // a mix of both formats (natural when combining user input with OS input)
        const INPUT: &str = "resource\\blah/Example.tar.gz";
        let path = StoredPathBuf::from_utf8_path(Utf8Path::new(INPUT));
        assert_eq!(path.as_str(), "resource\\blah\\Example.tar.gz");
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn mixed_path_unconverted1() {
        // a mix of both formats (natural when combining user input with OS input)
        // here we're testing the verbatim `new` conversion produces a coherent value
        const INPUT: &str = "resource\\blah/Example.tar.gz";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "resource\\blah/Example.tar.gz");
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn mixed_path_unconverted2() {
        // a mix of both formats (natural when combining user input with OS input)
        // here we're testing the verbatim `new` conversion produces a coherent value
        const INPUT: &str = "resource/blah\\Example.tar.gz";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "resource/blah\\Example.tar.gz");
        assert_eq!(path.file_name(), Some("Example.tar.gz"));
        assert_eq!(path.file_stem(), Some("Example.tar"));
        assert_eq!(path.extension(), Some("gz"));
    }

    #[test]
    fn empty_path() {
        // a mix of both formats (natural when combining user input with OS input)
        // here we're testing the verbatim `new` conversion produces a coherent value
        const INPUT: &str = "";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "");
        assert_eq!(path.file_name(), None);
        assert_eq!(path.file_stem(), None);
        assert_eq!(path.extension(), None);
    }

    #[test]
    fn just_file() {
        // a mix of both formats (natural when combining user input with OS input)
        // here we're testing the verbatim `new` conversion produces a coherent value
        const INPUT: &str = "abc.txt";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_1() {
        // make sure we trim a trailing slash
        const INPUT: &str = "abc.txt/";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt/");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_2() {
        // make sure we trim a trailing slash
        const INPUT: &str = "abc.txt\\";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt\\");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_3() {
        // make sure we trim a trailing slash
        const INPUT: &str = "abc.txt\\\\\\";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt\\\\\\");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_4() {
        // make sure we trim a trailing slash
        const INPUT: &str = "abc.txt/////";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt/////");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_5() {
        // make sure we trim a trailing slash
        const INPUT: &str = "abc.txt/\\//\\//";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt/\\//\\//");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_6() {
        /// make sure we trim a trailing slash dot
        const INPUT: &str = "abc.txt/.";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt/.");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_7() {
        // make sure we trim a trailing slash dot
        const INPUT: &str = "abc.txt\\.";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt\\.");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn trail_slash_8() {
        // make sure we trim all kinds of trailing slash dot soup
        const INPUT: &str = "abc.txt/./.\\\\//\\././";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "abc.txt/./.\\\\//\\././");
        assert_eq!(path.file_name(), Some("abc.txt"));
        assert_eq!(path.file_stem(), Some("abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn just_dot() {
        // dot is not a file name, it's a relative-path directive
        const INPUT: &str = ".";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), ".");
        assert_eq!(path.file_name(), None);
        assert_eq!(path.file_stem(), None);
        assert_eq!(path.extension(), None);
    }

    #[test]
    fn just_dotfile() {
        // dotfiles are valid names, and they have no extensions
        const INPUT: &str = ".abc";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), ".abc");
        assert_eq!(path.file_name(), Some(".abc"));
        assert_eq!(path.file_stem(), Some(".abc"));
        assert_eq!(path.extension(), None);
    }

    #[test]
    fn just_dotfile_txt() {
        // dotfiles with extensions work
        const INPUT: &str = ".abc.txt";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), ".abc.txt");
        assert_eq!(path.file_name(), Some(".abc.txt"));
        assert_eq!(path.file_stem(), Some(".abc"));
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn just_dotdot() {
        // dot dot is not a file name, it's a relative-path directive
        const INPUT: &str = "..";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "..");
        assert_eq!(path.file_name(), None);
        assert_eq!(path.file_stem(), None);
        assert_eq!(path.extension(), None);
    }

    #[test]
    fn opaque_dotdot1() {
        // trailing dot dot is opaque and makes us have no filename
        const INPUT: &str = "a/b/..";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "a/b/..");
        assert_eq!(path.file_name(), None);
        assert_eq!(path.file_stem(), None);
        assert_eq!(path.extension(), None);
    }

    #[test]
    fn opaque_dotdot2() {
        // trailing dot dot is opaque and makes us have no filename
        const INPUT: &str = "a/b/../";
        let path = StoredPathBuf::new(INPUT.to_owned());
        assert_eq!(path.as_str(), "a/b/../");
        assert_eq!(path.file_name(), None);
        assert_eq!(path.file_stem(), None);
        assert_eq!(path.extension(), None);
    }
}
