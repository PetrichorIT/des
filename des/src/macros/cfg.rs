#![allow(unused_macros)]

macro_rules! cfg_net {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "net")]
            #[cfg_attr(docsrs, doc(cfg(feature = "net")))]
            $item
        )*
    }
}

macro_rules! cfg_async {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "async")]
            #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
            $item
        )*
    }
}

macro_rules! cfg_not_async {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "async"))]
            #[cfg_attr(docsrs, doc(cfg(not(feature = "async"))))]
            $item
        )*
    }
}

macro_rules! cfg_metrics {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "metrics")]
            #[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
            $item
        )*
    }
}

macro_rules! cfg_metrics_rt_full {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "metrics-rt-full")]
            #[cfg_attr(docsrs, doc(cfg(feature = "metrics-rt-full")))]
            $item
        )*
    }
}

macro_rules! cfg_metrics_module_time {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "metrics-module-time")]
            #[cfg_attr(docsrs, doc(cfg(feature = "metrics-module-time")))]
            $item
        )*
    }
}

macro_rules! cfg_cqueue {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "cqueue")]
            #[cfg_attr(docsrs, doc(cfg(feature = "cqueue")))]
            $item
        )*
    }
}

macro_rules! cfg_not_cqueue {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "cqueue"))]
            #[cfg_attr(docsrs, doc(cfg(not(feature = "cqueue"))))]
            $item
        )*
    }
}
