// we use ZERO_ADDRESS instead None because it is not a valid AccountId
pub const ZERO_ADDRESS: [u8; 32] = [255; 32];

// this macro use to ensure that the condition is true, if false, we will return the error
#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into())
        }
    }};
}