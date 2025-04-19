use alloc::{borrow::ToOwned, string::String, vec::Vec};
use core::fmt::Display;

#[derive(PartialEq, Eq, Clone)]
pub struct PathBuf(Vec<String>);

impl Display for PathBuf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("/{}", self.0.join("/")))
    }
}

impl From<PathBuf> for String {
    fn from(value: PathBuf) -> Self {
        value.path()
    }
}

impl From<String> for PathBuf {
    fn from(value: String) -> Self {
        Self::new().join(&value)
    }
}

impl From<&str> for PathBuf {
    fn from(value: &str) -> Self {
        Self::new().join(value)
    }
}

impl PathBuf {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn join(&self, path: &str) -> Self {
        let mut pb = if path.starts_with("/") {
            PathBuf::new()
        } else {
            self.clone()
        };
        path.split("/").for_each(|x| match x {
            "." | "" => {}
            ".." => {
                pb.0.pop();
            }
            name => pb.0.push(name.to_owned()),
        });

        pb
    }

    pub fn path(&self) -> String {
        String::from("/") + &self.0.join("/")
    }

    #[inline]
    pub fn filename(&self) -> String {
        self.0.last().cloned().unwrap_or(String::from("/"))
    }

    #[inline]
    pub fn dir(&self) -> PathBuf {
        let mut paths = self.clone();
        paths.0.pop();
        paths
    }

    #[inline]
    pub fn levels(&self) -> usize {
        self.0.len()
    }

    pub fn starts_with(&self, target: &PathBuf) -> bool {
        self.0.starts_with(&target.0)
    }

    pub fn trim_start(&self, target: &PathBuf) -> Self {
        let mut pb = self.clone();
        if pb.starts_with(target) {
            pb.0.drain(..target.0.len());
        }
        pb
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }
}
