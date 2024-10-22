use std::{any::Any, fmt::Debug};

pub(crate) struct AnyBox {
    inner: Box<dyn Any>,
    ty_info: &'static str,
}

impl AnyBox {
    pub(crate) fn new<T: 'static>(val: T) -> Self {
        Self {
            inner: Box::new(val),
            ty_info: std::any::type_name::<T>(),
        }
    }
    pub(crate) fn ty(&self) -> &'static str {
        self.ty_info
    }

    pub(crate) fn try_dup<T: 'static + Clone>(&self) -> Option<Self> {
        self.inner.downcast_ref::<T>().map(|v| Self {
            inner: Box::new(v.clone()),
            ty_info: std::any::type_name::<T>(),
        })
    }

    pub(crate) fn can_cast<T: 'static>(&self) -> bool {
        self.inner.is::<T>()
    }

    pub(crate) fn try_cast<T: 'static>(self) -> Result<T, Self> {
        match self.inner.downcast::<T>() {
            Ok(v) => Ok(*v),
            Err(e) => Err(Self {
                inner: e,
                ty_info: self.ty_info,
            }),
        }
    }

    pub(crate) fn try_cast_ref<T: 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    pub(crate) fn try_cast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.inner.downcast_mut::<T>()
    }
}

impl Debug for AnyBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnyBox {{ {} }}", self.ty_info)
    }
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use super::*;

    #[test]
    #[cfg(debug_assertions)]
    fn ty_debug_fmt() {
        use std::any::type_name;

        let boxed = AnyBox::new(String::from("Hello World!"));
        assert_eq!(boxed.ty(), type_name::<String>())
    }

    #[test]
    fn ty_dup() {
        let boxed = AnyBox::new(String::from("Hello World!"));
        let duped = boxed
            .try_dup::<String>()
            .expect("failed to dup as 'String'");

        assert_ne!(
            boxed.inner.as_ref() as *const dyn Any,
            duped.inner.as_ref() as *const dyn Any,
        );
    }

    #[test]
    fn can_cast() {
        let boxed = AnyBox::new(1i64);
        assert!(boxed.can_cast::<i64>());
        assert!(!boxed.can_cast::<i32>());
        assert!(!boxed.can_cast::<String>());
        assert!(!boxed.can_cast::<&i64>());
    }

    #[test]
    fn fmt() {
        assert_eq!(
            format!("{:?}", AnyBox::new(String::from("Hello World!"))),
            format!("AnyBox {{ {} }}", type_name::<String>())
        );
    }
}
