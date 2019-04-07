use crate::cmd;
use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::convert::Into;
use std::fmt;

// A raw request APDU.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Request {
    pub cla: u8,       // Class byte.
    pub ins: u8,       // Instruction byte.
    pub p1: u8,        // First parameter.
    pub p2: u8,        // Second parameter.
    pub data: Vec<u8>, // Command data.

    // Expected length of response; set with `expect()`. You typically don't need this,
    // as your transport will handle it automatically, unless you're building sequences for
    // offline execution.
    pub le: Option<usize>,
}

impl Request {
    pub fn new<T: Into<Vec<u8>>>(cla: u8, ins: u8, p1: u8, p2: u8, data: T) -> Self {
        Self {
            cla,
            ins,
            p1,
            p2,
            data: data.into(),
            le: None,
        }
    }

    pub fn expect(mut self, le: usize) -> Self {
        self.le = Some(le);
        self
    }
}

impl cmd::Request for Request {
    type Returns = Response;

    fn cla(&self) -> u8 {
        self.cla
    }
    fn ins(&self) -> u8 {
        self.ins
    }
    fn data(&self) -> (u8, u8, Vec<u8>) {
        (self.p1, self.p2, self.data.clone())
    }
    fn le(&self) -> Option<usize> {
        self.le
    }
}

// A raw response APDU.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: Status, // Status code.
    pub data: Vec<u8>,  // Response data.
}

impl Response {
    pub fn new<T: Into<Vec<u8>>>(status: Status, data: T) -> Self {
        return Self {
            data: data.into(),
            status,
        };
    }
}

impl cmd::Response for Response {
    fn from_apdu(res: Response) -> Result<Self> {
        Ok(res)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Status {
    OK,                              // 0x9000
    Warning(u8),                     // 0xXX00 - Generic warnings of class x.
    BytesRemaining(u8),              // 0x61XX - OK, x bytes remain.
    CardQuery(u8),                   // 0x6202-0x6280 - Card expects reply, 12.5.1
    ReturnDataMayBeCorrupted,        // 0x6281 - Part of returned data may be corrupted.
    EOF,                             // 0x6282 - EOF or unsuccessful search.
    SelectedFileDeactivated,         // 0x6283
    BadFileOrDataControlInformation, // 0x6284 - Not formatted according to 7.4.
    SelectedFileInTerminationState,  // 0x6285
    NoSensorInput,                   // 0x6286 - No input data available from a sensor on the card.
    DeactivatedReference,            // 0x6287 - At least one referenced record is deactivated.
    UnsuccessfulComparison,          // 0x6340 - Exact meaning depends on the command.
    FullByLastWrite,                 // 0x6381
    Counter(u8),                     // 0x63CX - Counter from 0 to 15 encoded by x.
    ErrError(u8),                    // 0xXX00 - Generic errors of class x.
    ErrImmediateResponseRequired,    // 0x6401 - Immediate response required by the card.
    ErrCardQuery(u8),                // 0x6402-0x6480 - ??? 12.5.1
    ErrChannelShareAccessDenied,     // 0x6481 - Logical channel shared access denied.
    ErrChannelOpenAccessDenied,      // 0x6482 - Logical channel opening access denied.
    ErrMemoryFailure,                // 0x6581
    ErrSecurity(u8),                 // 0x66XX - RFU for security errors.
    ErrMalformedAPDU, // 0x6701 - Command APDU format not compliant with this standard. (5.1.)
    ErrInvalidLc,     // 0x6702 - The value of Lc is not the one expected. - Transport bug!!
    ErrChannelUnsupported, // 0x6881 - Logical channel not supported.
    ErrSecureMessagingUnsupported, // 0x6882 - Secure messaging is not supported.
    ErrChainLastCommandExpected, // 0x6883 - Last command of the chain expected.
    ErrChainUnsupported, // 0x6884 - Command chaining not supported.
    ErrIncompatibleFileStructure, // 0x6981 - Nice.
    ErrSecurityStatus, // 0x6982 - Nice.
    ErrAuthMethodBlocked, // 0x6983 - Nice.
    ErrRefDataUnusable, // 0x6984 - Nice.
    ErrConditionsNotSatisfied, // 0x6985 - Nice.
    ErrNoCurrentEF,   // 0x6986 - Nice.
    ErrExpectedSecureMessagingDOs, // 0x6987 - Expected secure messaging DOs missing - Nice.
    ErrIncorrectSecureMessagingDOs, // 0x6988 - Incorrect secure messaging DOs - Nice.
    ErrCommandData,   // 0x6A80 - Incorrect parameters in the command data field.
    ErrFunctionNotSupported, // 0x6A81 - Function not supported.
    ErrFileOrApplicationNotFound, // 0x6A82
    ErrRecordNotFound, // 0x6A83
    ErrNotEnoughSpace, // 0x6A84
    ErrNcTLVStructure, // 0x6A85
    ErrP1P2(u8, u8),  // 0x6A86, 0x6B00
    ErrNcP1P2,        // 0x6A87
    ErrRefNotFound,   // 0x6A88
    ErrFileAlreadyExists, // 0x6A9
    ErrDFAlreadyExists, // 0x6A82
    ErrParamsP1P2,    // 0x6B00 - Wrong parameters P1-P2.
    ErrRetryWithLe(u8), // 0x6CXX - Wrong Le, retry with Le=x.
    ErrInstruction,   // 0x6D00 - Invalid or unsupported instruction.
    ErrClass,         // 0x6E00 - Unsupported class byte.
    ErrNoIdea,        // 0x6F00 - "No precise diagnosis".
    Unknown(u8, u8),  // Anything else!
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (sw1, sw2) = self.as_tuple();
        write!(f, "{:#X}{:X} - ", sw1, sw2)?;
        match self {
            Status::OK => write!(f, "OK!"),
            Status::Warning(x) => write!(f, "Warning: {:#X}", x),
            Status::BytesRemaining(x) => write!(f, "{:} bytes remaining", x),
            Status::CardQuery(x) => write!(f, "Card triggered a query: {:}", x),
            Status::ReturnDataMayBeCorrupted => write!(f, "Returned data may be corrupted"),
            Status::EOF => write!(f, "EOF"),
            Status::SelectedFileDeactivated => write!(f, "Selected file deactivated"),
            Status::BadFileOrDataControlInformation => write!(
                f,
                "File or data control information not formatted according to ISO 7816-4 s7.4"
            ),
            Status::SelectedFileInTerminationState => {
                write!(f, "Selected file in termination state")
            }
            Status::NoSensorInput => write!(f, "No sensor input"),
            Status::DeactivatedReference => {
                write!(f, "One or more referenced record is deactivated")
            }
            Status::UnsuccessfulComparison => write!(f, "Unsuccessful comparison"),
            Status::FullByLastWrite => write!(f, "File filled up by last write"),
            Status::Counter(x) => write!(f, "Counter: {:}", x),
            Status::ErrError(_) => write!(f, "Error"),
            Status::ErrImmediateResponseRequired => {
                write!(f, "Error: Immediate response required by card")
            }
            Status::ErrCardQuery(x) => write!(f, "Error: Card triggered a query: {:}", x),
            Status::ErrChannelShareAccessDenied => {
                write!(f, "Error: Logical channel shared access denied")
            }
            Status::ErrChannelOpenAccessDenied => {
                write!(f, "Error: Logical channel opening denied")
            }
            Status::ErrMemoryFailure => write!(f, "Error: Memory failure"),
            Status::ErrSecurity(x) => write!(f, "Error: Security: {:#X}", x),
            Status::ErrMalformedAPDU => write!(f, "Error: Malformed Request APDU"),
            Status::ErrInvalidLc => write!(f, "Error: The value of Lc is not the one expected"),
            Status::ErrChannelUnsupported => write!(f, "Error: Logical channel is not supported"),
            Status::ErrSecureMessagingUnsupported => {
                write!(f, "Error: Secure messaging is not supported")
            }
            Status::ErrChainLastCommandExpected => {
                write!(f, "Error: Last command of the chain expected")
            }
            Status::ErrChainUnsupported => write!(f, "Error: Command chaining is not supported"),
            Status::ErrIncompatibleFileStructure => {
                write!(f, "Error: Command incompatible with file structure")
            }
            Status::ErrSecurityStatus => write!(f, "Error: Security status not satisfied"),
            Status::ErrAuthMethodBlocked => write!(f, "Error: Authentication method blocked"),
            Status::ErrRefDataUnusable => write!(f, "Error: Reference data is not usable"),
            Status::ErrConditionsNotSatisfied => {
                write!(f, "Error: Conditions of use not satisfied")
            }
            Status::ErrNoCurrentEF => write!(f, "Error: Command not allowed: No current EF"),
            Status::ErrExpectedSecureMessagingDOs => {
                write!(f, "Error: Expected secure messaging DOs missing")
            }
            Status::ErrIncorrectSecureMessagingDOs => {
                write!(f, "Error: Incorrect secure messaging DOs")
            }
            Status::ErrCommandData => write!(f, "Error: Incorrect parameters in command data"),
            Status::ErrFunctionNotSupported => write!(f, "Error: Function not supported"),
            Status::ErrFileOrApplicationNotFound => write!(f, "Error: File/application not found"),
            Status::ErrRecordNotFound => write!(f, "Error: Record not found"),
            Status::ErrNotEnoughSpace => write!(f, "Error: Not enough space in file"),
            Status::ErrNcTLVStructure => write!(f, "Error: Nc inconsistent with TLV structure"),
            Status::ErrP1P2(_, _) => write!(f, "Error: Incorrect parameters in P1/P2"),
            Status::ErrNcP1P2 => write!(f, "Error: Nc inconsistent with parameters P1/P2"),
            Status::ErrRefNotFound => write!(f, "Error: Reference or referenced data not found"),
            Status::ErrFileAlreadyExists => write!(f, "Error: File already exists"),
            Status::ErrDFAlreadyExists => write!(f, "Error: DF already exists"),
            Status::ErrParamsP1P2 => write!(f, "Error: Incorrect parameters in P1/P2"),
            Status::ErrRetryWithLe(x) => write!(f, "Error: Retry with Le={:}", x),
            Status::ErrInstruction => write!(f, "Error: "),
            Status::ErrClass => write!(f, ""),
            Status::ErrNoIdea => write!(f, ""),
            Status::Unknown(_, _) => write!(f, "Unknown"),
        }
    }
}

impl Status {
    pub fn from_u16(xy: u16) -> Status {
        let bytes = xy.to_be_bytes();
        Status::from(bytes[0], bytes[1])
    }

    pub fn from(x: u8, y: u8) -> Status {
        match (x, y) {
            (0x90, 0x00) => Status::OK,
            (0x61, x) => Status::BytesRemaining(x),
            // Warning - "State of non-volatile memory is unchanged"
            (y @ 0x62, 0x00) => Status::Warning(y),
            (0x62, x @ 0x02...0x80) => Status::CardQuery(x),
            (0x62, 0x81) => Status::ReturnDataMayBeCorrupted,
            (0x62, 0x82) => Status::EOF,
            (0x62, 0x83) => Status::SelectedFileDeactivated,
            (0x62, 0x84) => Status::BadFileOrDataControlInformation,
            (0x62, 0x85) => Status::SelectedFileInTerminationState,
            (0x62, 0x86) => Status::NoSensorInput,
            (0x62, 0x87) => Status::DeactivatedReference,
            // Warning - "State of non-volatile memory may have changed"
            (y @ 0x63, 0x00) => Status::Warning(y),
            (0x63, 0x40) => Status::UnsuccessfulComparison,
            (0x63, 0x81) => Status::FullByLastWrite,
            (0x63, x @ 0xC0...0xCF) => Status::Counter(x - 0xC0),
            // Execution Error - "State of non-volatile memory is unchanged"
            (y @ 0x64, 0x00) => Status::Warning(y),
            (0x64, 0x01) => Status::ErrImmediateResponseRequired,
            (0x64, x @ 0x02...0x80) => Status::ErrCardQuery(x),
            (0x64, 0x81) => Status::ErrChannelShareAccessDenied,
            (0x64, 0x82) => Status::ErrChannelOpenAccessDenied,
            // Execution Error - "State of non-volatile memory may have changed"
            (y @ 0x65, 0x00) => Status::ErrError(y),
            (0x65, 0x81) => Status::ErrMemoryFailure,
            // Execution Error - Security issues
            // (There aren't actually errors in here, the whole block is RFU.)
            // Checking Error - Wrong length
            (y @ 0x67, 0x00) => Status::ErrError(y),
            (0x67, 0x01) => Status::ErrMalformedAPDU,
            (0x67, 0x02) => Status::ErrInvalidLc,
            // Checking Error - Functions in CLA not supported
            (y @ 0x68, 0x00) => Status::ErrError(y),
            (0x68, 0x81) => Status::ErrChannelUnsupported,
            (0x68, 0x82) => Status::ErrSecureMessagingUnsupported,
            (0x68, 0x83) => Status::ErrChainLastCommandExpected,
            (0x68, 0x84) => Status::ErrChainUnsupported,
            // Checking Error - Command not allowed - Nice.
            (y @ 0x69, 0x00) => Status::ErrError(y),
            (0x69, 0x81) => Status::ErrIncompatibleFileStructure,
            (0x69, 0x82) => Status::ErrSecurityStatus,
            (0x69, 0x83) => Status::ErrAuthMethodBlocked,
            (0x69, 0x84) => Status::ErrRefDataUnusable,
            (0x69, 0x85) => Status::ErrConditionsNotSatisfied,
            (0x69, 0x86) => Status::ErrNoCurrentEF,
            (0x69, 0x87) => Status::ErrExpectedSecureMessagingDOs,
            (0x69, 0x88) => Status::ErrIncorrectSecureMessagingDOs,
            // Wrong parameters
            (y @ 0x6A, 0x00) => Status::ErrError(y),
            (0x6A, 0x80) => Status::ErrCommandData,
            (0x6A, 0x81) => Status::ErrFunctionNotSupported,
            (0x6A, 0x82) => Status::ErrFileOrApplicationNotFound,
            (0x6A, 0x83) => Status::ErrRecordNotFound,
            (0x6A, 0x84) => Status::ErrNotEnoughSpace,
            (0x6A, 0x85) => Status::ErrNcTLVStructure,
            (y @ 0x6A, x @ 0x86) => Status::ErrP1P2(y, x),
            (0x6A, 0x87) => Status::ErrNcP1P2,
            (0x6A, 0x88) => Status::ErrRefNotFound,
            (0x6A, 0x89) => Status::ErrFileAlreadyExists,
            (0x6A, 0x8A) => Status::ErrDFAlreadyExists,
            (y @ 0x6B, x @ 0x00) => Status::ErrP1P2(y, x),
            (0x6C, x) => Status::ErrRetryWithLe(x),
            (0x6D, 0x00) => Status::ErrInstruction,
            (0x6E, 0x00) => Status::ErrClass,
            (0x6F, 0x00) => Status::ErrNoIdea,
            (y, x) => Status::Unknown(y, x),
        }
    }

    pub fn as_tuple(&self) -> (u8, u8) {
        match *self {
            Status::OK => (0x90, 0x00),
            Status::Warning(x) => (x, 0x00),
            Status::BytesRemaining(x) => (0x61, x),
            Status::CardQuery(x) => (0x62, x),
            Status::ReturnDataMayBeCorrupted => (0x62, 0x81),
            Status::EOF => (0x62, 0x82),
            Status::SelectedFileDeactivated => (0x62, 0x83),
            Status::BadFileOrDataControlInformation => (0x62, 0x84),
            Status::SelectedFileInTerminationState => (0x62, 0x85),
            Status::NoSensorInput => (0x62, 0x86),
            Status::DeactivatedReference => (0x62, 0x87),
            Status::UnsuccessfulComparison => (0x63, 0x40),
            Status::FullByLastWrite => (0x63, 0x81),
            Status::Counter(x) => (0x63, 0xC0 + x),
            Status::ErrError(x) => (x, 0x00),
            Status::ErrImmediateResponseRequired => (0x64, 0x01),
            Status::ErrCardQuery(x) => (0x64, x),
            Status::ErrChannelShareAccessDenied => (0x64, 0x81),
            Status::ErrChannelOpenAccessDenied => (0x64, 0x82),
            Status::ErrMemoryFailure => (0x65, 0x81),
            Status::ErrSecurity(x) => (0x66, x),
            Status::ErrMalformedAPDU => (0x67, 0x01),
            Status::ErrInvalidLc => (0x67, 0x02),
            Status::ErrChannelUnsupported => (0x68, 0x81),
            Status::ErrSecureMessagingUnsupported => (0x68, 0x82),
            Status::ErrChainLastCommandExpected => (0x68, 0x83),
            Status::ErrChainUnsupported => (0x68, 0x84),
            Status::ErrIncompatibleFileStructure => (0x69, 0x81),
            Status::ErrSecurityStatus => (0x69, 0x82),
            Status::ErrAuthMethodBlocked => (0x69, 0x83),
            Status::ErrRefDataUnusable => (0x69, 0x84),
            Status::ErrConditionsNotSatisfied => (0x69, 0x85),
            Status::ErrNoCurrentEF => (0x69, 0x86),
            Status::ErrExpectedSecureMessagingDOs => (0x69, 0x87),
            Status::ErrIncorrectSecureMessagingDOs => (0x69, 0x88),
            Status::ErrCommandData => (0x6A, 0x80),
            Status::ErrFunctionNotSupported => (0x6A, 0x81),
            Status::ErrFileOrApplicationNotFound => (0x6A, 0x82),
            Status::ErrRecordNotFound => (0x6A, 0x83),
            Status::ErrNotEnoughSpace => (0x6A, 0x84),
            Status::ErrNcTLVStructure => (0x6A, 0x85),
            Status::ErrP1P2(x, y) => (x, y),
            Status::ErrNcP1P2 => (0x6A, 0x87),
            Status::ErrRefNotFound => (0x6A, 0x88),
            Status::ErrFileAlreadyExists => (0x6A, 0x89),
            Status::ErrDFAlreadyExists => (0x6A, 0x8A),
            Status::ErrParamsP1P2 => (0x6B, 0x00),
            Status::ErrRetryWithLe(x) => (0x6C, x),
            Status::ErrInstruction => (0x6D, 0x00),
            Status::ErrClass => (0x6E, 0x00),
            Status::ErrNoIdea => (0x6F, 0x00),
            Status::Unknown(x, y) => (x, y),
        }
    }

    pub fn as_u16(&self) -> u16 {
        let tuple = self.as_tuple();
        u16::from_be_bytes([tuple.0, tuple.1])
    }

    pub fn sw1(&self) -> u8 {
        self.as_tuple().0
    }

    pub fn sw2(&self) -> u8 {
        self.as_tuple().1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_u16() {
        assert_eq!(
            format!("{:#X}", Status::from(0x90, 0x00).as_u16()),
            format!("{:#X}", 0x9000)
        );
    }

    #[test]
    fn test_from_u16() {
        assert_eq!(
            format!("{:?}", Status::from_u16(0x9000).as_tuple()),
            format!("{:?}", (0x90, 0x00))
        )
    }

    #[test]
    fn test_round_trip() {
        for i in 0x0000..0xFFFF {
            assert_eq!(
                format!("{:#X}", Status::from_u16(i).as_u16()),
                format!("{:#X}", i)
            );
        }
    }
}
