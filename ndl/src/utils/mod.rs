pub type TyResolveResult<T> = Result<T, TyResolveError<T>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyResolveError<T> {
    NoneFound,
    FoundLookalike(T, usize),
}

impl<T> TyResolveError<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> TyResolveError<U> {
        match self {
            Self::NoneFound => TyResolveError::NoneFound,
            Self::FoundLookalike(v, d) => TyResolveError::FoundLookalike(f(v), d),
        }
    }

    pub fn lookalike(&self) -> Option<(&T, usize)> {
        match self {
            Self::NoneFound => None,
            Self::FoundLookalike(v, d) => Some((v, *d)),
        }
    }
}

pub fn edit_distance(lhs: &str, rhs: &str) -> usize {
    let n = lhs.chars().count();
    let m = rhs.chars().count();

    // Ensure that 'lhs' is the smaller string (its the outer loop)
    if n < m {
        return edit_distance(rhs, lhs);
    }

    // Handle special cases with uninitalized data.
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    let mut prev;
    let mut tmp;
    let mut current = vec![0; m + 1];

    // Build edit str
    for (i, item) in current.iter_mut().enumerate().skip(1) {
        *item = i;
    }

    // Main Loop
    for (i, ca) in lhs.chars().enumerate() {
        // get first column for this row
        prev = current[0];
        current[0] = i + 1;
        for (j, cb) in rhs.chars().enumerate() {
            tmp = current[j + 1];
            current[j + 1] = std::cmp::min(
                // DEL
                tmp + 1,
                std::cmp::min(
                    // INSERTION
                    current[j] + 1,
                    // SUBST
                    prev + if ca == cb { 0 } else { 1 },
                ),
            );
            prev = tmp;
        }
    }
    current[m]
}
