use std::{any::Any, fmt::Debug};

pub(crate) struct AnyBox {
    inner: Box<dyn Any>,

    #[cfg(debug_assertions)]
    ty_info: &'static str,
}

impl AnyBox {
    pub(crate) fn new<T: 'static>(val: T) -> Self {
        Self {
            inner: Box::new(val),

            #[cfg(debug_assertions)]
            ty_info: std::any::type_name::<T>(),
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn ty(&self) -> &'static str {
        self.ty_info
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn ty(&self) -> &'static str {
        "no ty info"
    }

    pub(crate) fn try_dup<T: 'static + Clone>(&self) -> Option<Self> {
        self.inner.downcast_ref::<T>().map(|v| Self {
            inner: Box::new(v.clone()),

            #[cfg(debug_assertions)]
            ty_info: std::any::type_name::<T>(),
        })
    }

    pub(crate) fn can_cast<T: 'static>(&self) -> bool {
        self.inner.is::<T>()
    }

    // pub(crate) fn try_cast<T: 'static + Send>(self) -> Result<T, Self> {
    //     match self.inner.downcast::<T>() {
    //         Ok(v) => Ok(Box::into_inner(v)),
    //         Err(e) => Err(Self {
    //             inner: e,
    //             #[cfg(debug_assertions)]
    //             ty_info: self.ty_info,
    //         }),
    //     }
    // }

    pub(crate) unsafe fn try_cast_unsafe<T: 'static>(self) -> Result<T, Self> {
        match self.inner.downcast::<T>() {
            Ok(v) => Ok(*v),
            Err(e) => Err(Self {
                inner: e,
                #[cfg(debug_assertions)]
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
        #[cfg(not(debug_assertions))]
        return write!(f, "AnyBox");
        #[cfg(debug_assertions)]
        return write!(f, "AnyBox {{ {} }}", self.ty_info);
    }
}
