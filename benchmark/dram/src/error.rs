mod internel {
    #![allow(dead_code)]
    use errore::error::*;
    use errore::kind::*;

    #[ctor::ctor]
    static _RMODULE: RModule = RModule::new("raplre-bm-dram");

    #[inline]
    pub(crate) fn new_simple(kind: RErrorKind) -> RError {
        RError::new_simple(_RMODULE.clone(), kind)
    }
    #[inline]
    pub(crate) fn new_simple_msg(kind: RErrorKind, msg: &'static str) -> RError {
        RError::new_simple_msg(_RMODULE.clone(), kind, msg)
    }
    #[inline]
    pub(crate) fn new_custom_msg(kind: RErrorKind, msg: String) -> RError {
        RError::new_custom_msg(_RMODULE.clone(), kind, msg)
    }
}

#[allow(unused_imports)]
pub use errore::error::RError;
#[allow(unused_imports)]
pub use errore::kind::*;
#[allow(unused_imports)]
pub use internel::*;

#[macro_export]
macro_rules! throw_rerr {
        ($kind:expr) => {
            return crate::new_simple($kind).to_err()
        };
        ($kind:expr,$msg:expr) => {
            return crate::new_simple_msg($kind, $msg).to_err()
        };
        ($kind:expr,$($args:tt)*) => {
            return crate::new_custom_msg($kind,format!($($args)*)).to_err()
        }
    }

#[macro_export]
macro_rules! ignore_rerr {
    ($err:expr) => {
        let _: usize = { &$err as *const errore::error::RError as usize };
        // log::warn!("{}. Ignore it.", $err);
        $err.ignore()
    };
}

#[macro_export]
macro_rules! block_rerr {
    ($err:expr) => {
        let _: usize = { &$err as *const errore::error::RError as usize };
        // log::warn!("{}. Block it.", $err);
        $err.ignore()
    };
}

#[macro_export]
macro_rules! new_rerr {
    ($kind:expr) => {
            crate::new_simple($kind)
        };
        ($kind:expr,$msg:expr) => {
            crate::new_simple_msg($kind, $msg)
        };
        ($kind:expr,$($args:tt)*) => {
            crate::new_custom_msg($kind,format!($($args)*))
        };
}
