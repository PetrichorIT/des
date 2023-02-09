use std::path::{Component, Path, PathBuf};

pub(crate) fn common_path(lhs: impl AsRef<Path>, rhs: impl AsRef<Path>) -> PathBuf {
    let lhs = lhs.as_ref().components();
    let rhs = rhs.as_ref().components();

    let mut result = PathBuf::new();

    for (l, r) in lhs.zip(rhs) {
        if l == r {
            match l {
                Component::ParentDir => {
                    assert!(result.pop(), "cannot escape scope");
                }
                l => result.push(l),
            }
        } else {
            break;
        }
    }

    result
}

pub(crate) fn strip_prefix(path: impl AsRef<Path>, prefix: impl AsRef<Path>) -> PathBuf {
    let mut lhs = prefix.as_ref().components();
    let mut rhs = path.as_ref().components();

    while let Some(l) = lhs.next() {
        let r = rhs.next().unwrap();
        assert_eq!(l, r);
    }

    canon(PathBuf::from_iter(rhs))
}

pub(crate) fn canon(path: impl AsRef<Path>) -> PathBuf {
    let comps = path.as_ref().components();
    let mut result = PathBuf::new();
    for comp in comps {
        match comp {
            Component::ParentDir => {
                assert!(result.pop())
            }
            other => result.push(other),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_parent_path() {
        assert_eq!(common_path("a/b/c/d", "a/b/o/p"), PathBuf::from("a/b"));

        assert_eq!(
            common_path("lookup/a/b", "lookup/c/d"),
            PathBuf::from("lookup")
        );

        assert_eq!(common_path("a/b/c/d", "a/b/c/d"), PathBuf::from("a/b/c/d"));

        assert_eq!(common_path("a/b/c/d", "e/a/b/o/p"), PathBuf::from(""));
        assert_eq!(common_path("a/b/c/d", ""), PathBuf::from(""));
    }

    #[test]
    fn parent_steps() {
        assert_eq!(common_path("a/../a/d", "a/../o/p"), PathBuf::from(""));

        assert_eq!(
            common_path("lookup/../b", "lookup/a/../d"),
            PathBuf::from("lookup")
        );

        assert_eq!(common_path("a/../c/d", "a/../c/d"), PathBuf::from("c/d"));

        assert_ne!(common_path("a/../c/d", "/c/d"), PathBuf::from("/c/d"));
    }

    #[test]
    fn strip_no_parent_prefix() {
        assert_eq!(strip_prefix("a/b/c/d", "a/b"), PathBuf::from("c/d"));
        assert_eq!(strip_prefix("a/b/c/d", "a/b/c"), PathBuf::from("d"));
        assert_eq!(strip_prefix("a/b/c/d", ""), PathBuf::from("a/b/c/d"));
    }

    #[test]
    fn strip_parent_steps() {
        assert_eq!(strip_prefix("a/b/c/../d", "a/b"), PathBuf::from("d"));
    }

    #[test]
    fn canon_paths() {
        assert_eq!(canon("a/../b"), PathBuf::from("b"));
        assert_eq!(canon("a/c/../../b"), PathBuf::from("b"));
        assert_eq!(canon("a/c/../b"), PathBuf::from("a/b"));
    }
}
