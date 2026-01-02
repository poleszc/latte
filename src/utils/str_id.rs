use serde::{Serialize, Serializer};
use std::fmt;
use std::ops::Deref;
use ustr::Ustr;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StrID(Ustr);

impl StrID {
    pub fn new(s: &str) -> Self {
        StrID(Ustr::from(s))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for StrID {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl From<&str> for StrID {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for StrID {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl fmt::Debug for StrID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for StrID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl Serialize for StrID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let s1 = StrID::from("hello");
        let s2 = StrID::from("hello");
        let s3 = StrID::from("world");

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
        assert_eq!(s1.as_str().as_ptr(), s2.as_str().as_ptr());
    }
}
