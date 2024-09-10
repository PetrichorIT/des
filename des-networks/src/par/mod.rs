use std::{marker::PhantomData, ops::Deref, sync::Arc};

mod map;
mod yaml;

pub use map::ParMap;

// # External API

/// A handle to a parameter associated to a node within the simulation.
///
/// This type is parameterized by a type-state parameter `S`.
/// This parameter indicates whether the parameter is guaranteed to
/// exist `S = Exists` or this remains in question `S = Optional`.
///
/// This type provides methods to read an write parameters, based on the
/// type state. `Par<Exists>` implement `Deref<Target = str>` so parameters
/// can be extracted and perhaps parsed, as soon as the existence of the parameter
/// is confirmed.
#[derive(Debug)]
pub struct Par<S = Optional>
where
    S: private::ParState,
{
    key: String,
    value: Option<String>,
    map: Arc<ParMap>,

    _phantom: PhantomData<S>,
}

/// The state of a [`Par`] where its not decided
/// whether data is contained or not. Useful for writing data
/// to not yet initalized parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Optional;
impl private::ParState for Optional {}

/// The state of a [`Par`] where the contents are guaranteed
/// to be there, thus allowing derefs on the handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Exists;
impl private::ParState for Exists {}

/// Errors that can occur in combination with [`Par`] objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParError {
    /// This error occures if a write operation failed, since a write lock could
    /// not be aquired.
    CouldNotAquireWriteLock,
}

mod private {
    pub trait ParState {}
}

impl Par<Optional> {
    pub fn new(map: Arc<ParMap>, key: &str, module: &str) -> Par {
        if module.is_empty() {
            Par {
                key: key.to_string(),
                value: None,
                map,
                _phantom: PhantomData,
            }
        } else {
            Par {
                key: format!("{module}.{key}"),
                value: None,
                map,
                _phantom: PhantomData,
            }
        }
    }
}

impl<S> Par<S>
where
    S: private::ParState,
{
    /// Returns a handle allowing [`Deref`] on the contained
    /// value, consuming self.
    ///
    /// # Panics
    ///
    /// This function panics of the Par points to no data.
    ///
    #[must_use]
    pub fn expect(self, msg: &str) -> Par<Exists> {
        if let Some(value) = self.map.get_rlock(&self.key, 1) {
            Par {
                key: self.key.clone(),
                value: Some(value),
                map: self.map.clone(),
                _phantom: PhantomData,
            }
        } else {
            panic!("{msg}");
        }
    }

    /// Returns a handle allowing [`Deref`] on the contained
    /// value, consuming self.
    ///
    /// See [`Par::expect`] for more information.
    #[must_use]
    pub fn unwrap(self) -> Par<Exists> {
        self.expect("called `Par::unwrap` on a parameter that does not exist")
    }

    /// Indicates whether the handle contains a value.
    #[must_use]
    pub fn is_some(&self) -> bool {
        // (0) Shortciruit
        if self.value.is_some() {
            return true;
        }

        // (1) Long way around
        self.map.get_rlock(&self.key, 0).is_some()
    }

    /// Indicates whether the handle contains a value.
    #[must_use]
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    /// Returns the contained value optionaly, thereby losing the
    /// ability to set the par. This does not create a permantent
    /// read lock.
    #[must_use]
    pub fn as_option(self) -> Option<String> {
        self.map.get_rlock(&self.key, 0)
    }

    /// Sets the parameter to the given value.
    ///
    /// # Errors
    ///
    /// Returns an error if other active locks exist for the datapoint.
    #[allow(clippy::needless_pass_by_value)]
    pub fn set(self, value: impl ToString) -> Result<Par<Exists>, ParError> {
        let value = value.to_string();
        if self.map.insert(&self.key, value) {
            Ok(Par {
                key: self.key.clone(),
                value: self.map.get_rlock(&self.key, 1),
                map: self.map.clone(),
                _phantom: PhantomData,
            })
        } else {
            Err(ParError::CouldNotAquireWriteLock)
        }
    }

    /// Remove the entry from the par storage.
    ///
    /// Returns a `Par` object with optional (in this case None) content.
    #[must_use]
    pub fn unset(self) -> Par<Optional> {
        self.map.remove(&self.key);
        Par {
            value: None,
            key: self.key.clone(),
            map: self.map.clone(),
            _phantom: PhantomData,
        }
    }
}

impl Par<Exists> {
    /// Uses a custom string parser to parse a string, timming
    /// quotation marks in the process.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn into_inner(&self) -> String {
        let mut parsed = self.value.clone().unwrap();
        // Trim marks
        let mut chars = parsed.chars();
        let mut is_marked = parsed.len() >= 2;
        is_marked &= chars.next() == Some('"');
        is_marked &= chars.next_back() == Some('"');

        if is_marked {
            parsed.pop();
            parsed.remove(0);
            parsed
        } else {
            parsed
        }
    }
}

impl Deref for Par<Exists> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap()
    }
}

impl<S> Clone for Par<S>
where
    S: private::ParState,
{
    fn clone(&self) -> Self {
        if self.value.is_some() {
            self.map.get_rlock(&self.key, 1);
        }

        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            map: self.map.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S> Drop for Par<S>
where
    S: private::ParState,
{
    fn drop(&mut self) {
        // (0) Only if Par<Exists>
        if self.value.is_some() {
            self.map.release_rlock(&self.key);
        }
    }
}
