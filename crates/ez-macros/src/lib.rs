#![warn(unused_crate_dependencies)]

pub use ez_proc_macro as proc;

#[macro_export]
macro_rules! throw {
    ($msg:literal $(,)?) => { {
        ::ez::__::Err(::ez::Error::msg($msg))?;
        unreachable!()
    } };

    ($msg:literal $(, $rest:tt)* $(,)?) => { {
        ::ez::__::Err(::ez::Error::msg(format!($msg $(, $rest)*)))?;
        unreachable!()
    } };

    ($error:expr $(,)?) => { {
        ::ez::__::Err($error)?;
        unreachable!()
    } };
}
