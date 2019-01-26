use crate::transport::Transport;

// Interfaces wrap Transports and provide higher-level, application-specific APIs.
pub trait Interface: Sized {
    // Instantiates the selectable with a certain transport.
    fn with<T: Transport>(t: &T) -> Self;

    // Returns the wrapped transport.
    fn transport<T: Transport>() -> T;
}
