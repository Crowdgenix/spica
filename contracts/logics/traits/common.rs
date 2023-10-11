pub const ZERO_ADDRESS: [u8; 32] = [255; 32];

#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into())
        }
    }};
}