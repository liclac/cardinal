use error_chain::error_chain;

error_chain! {
    foreign_links {
        IO(std::io::Error);
        PCSC(pcsc::Error);
        TryFromInt(std::num::TryFromIntError);
    }

    errors {
        UnsupportedProtocol(s: String) {
            description("card uses an unsupported protocol"),
            display("card uses an unsupported protocol: {}", s),
        }
        APDUBodyTooLong(len: usize, max: usize) {
            description("APDU body is too long"),
            display("APDU body is {} bytes long, but protocol supports only up to {}", len, max),
        }
    }
}
