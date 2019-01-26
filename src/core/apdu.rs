use std::convert::Into;

// A raw request APDU.
#[derive(Debug, Clone, PartialEq, Eq)]
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

// A raw response APDU.
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status(pub u8, pub u8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusClass {
    OK,                              // 0x9000
    Generic(u8),                     // 0xXX00 - Generic errors/warnings of class x.
    BytesRemaining(u8),              // 0x61XX - OK, x bytes remain.
    CardQuery(u8),                   // 0x6202-0x6280 - Card expects reply, 12.5.1
    EOF,                             // 0x6282 - EOF or unsuccessful search.
    SelectedFileDeactivated,         // 0x6283
    BadFileOrDataControlInformation, // 0x6284 - Not formatted according to 7.4.
    SelectedFileInTerminationState,  // 0x6285
    NoSensorInput,                   // 0x6286 - No input data available from a sensor on the card.
    DeactivatedReference,            // 0x6287 - At least one referenced record is deactivated.
    UnsuccessfulComparison,          // 0x6340 - Exact meaning depends on the command.
    FullByLastWrite,                 // 0x6381
    Counter(u8),                     // 0x63CX - Counter from 0 to 15 encoded by x.
    ErrImmediateResponseRequired,    // 0x6401 - Immediate response required by the card.
    ErrCardQuery(u8),                // 0x6402-0x6480 - ??? 12.5.1
    ErrChannelShareAccessDenied,     // 0x6481 - Logical channel shared access denied.
    ErrChannelOpenAccessDenied,      // 0x6482 - Logical channel opening access denied.
    ErrMemoryFailure,                // 0x6581
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
    ErrMissingSecureMessagingDOs, // 0x6987 - Expected secure messaging DOs missing - Nice.
    ErrIncorrectSecureMessagingDOs, // 0x6988 - Incorrect secure messaging DOs - Nice.
    ErrParamsData,    // 0x6A80 - Incorrect parameters in the command data field.
    ErrParamsP1P2,    // 0x6B00 - Wrong parameters P1-P2.
    ErrRetryWithLe(u8), // 0x6CXX - Wrong Le, retry with Le=x.
    ErrInstruction,   // 0x6D00 - Invalid or unsupported instruction.
    ErrClass,         // 0x6E00 - Unsupported class byte.
    ErrNoIdea,        // 0x6F00 - "No precise diagnosis".
    Unknown(u8, u8),  // Anything else!
}

impl Status {
    pub fn class(&self) -> StatusClass {
        match (self.0, self.1) {
            (0x90, _) => StatusClass::OK,
            (0x61, x) => StatusClass::BytesRemaining(x),
            // Warning - "State of non-volatile memory is unchanged"
            (y @ 0x62, 0x00) => StatusClass::Generic(y),
            (0x62, x @ 0x02...0x80) => StatusClass::CardQuery(x),
            (0x62, 0x82) => StatusClass::EOF,
            (0x62, 0x83) => StatusClass::SelectedFileDeactivated,
            (0x62, 0x84) => StatusClass::BadFileOrDataControlInformation,
            (0x62, 0x85) => StatusClass::SelectedFileInTerminationState,
            (0x62, 0x86) => StatusClass::NoSensorInput,
            (0x62, 0x87) => StatusClass::DeactivatedReference,
            // Warning - "State of non-volatile memory may have changed"
            (y @ 0x63, 0x00) => StatusClass::Generic(y),
            (0x63, 0x40) => StatusClass::UnsuccessfulComparison,
            (0x63, 0x81) => StatusClass::FullByLastWrite,
            (0x63, x @ 0xC0...0xCF) => StatusClass::Counter(x),
            // Execution Error - "State of non-volatile memory is unchanged"
            (y @ 0x64, 0x00) => StatusClass::Generic(y),
            (0x64, 0x01) => StatusClass::ErrImmediateResponseRequired,
            (0x64, x @ 0x02...0x80) => StatusClass::ErrCardQuery(x),
            (0x64, 0x81) => StatusClass::ErrChannelShareAccessDenied,
            (0x64, 0x82) => StatusClass::ErrChannelOpenAccessDenied,
            // Execution Error - "State of non-volatile memory may have changed"
            (y @ 0x65, 0x00) => StatusClass::Generic(y),
            (0x65, 0x81) => StatusClass::ErrMemoryFailure,
            // Execution Error - Security issues
            // (There aren't actually errors in here, the whole block is RFU.)
            // Checking Error - Wrong length
            (y @ 0x67, 0x00) => StatusClass::Generic(y),
            (0x67, 0x01) => StatusClass::ErrMalformedAPDU,
            (0x67, 0x02) => StatusClass::ErrInvalidLc,
            // Checking Error - Functions in CLA not supported
            (y @ 0x68, 0x00) => StatusClass::Generic(y),
            (0x68, 0x81) => StatusClass::ErrChannelUnsupported,
            (0x68, 0x82) => StatusClass::ErrSecureMessagingUnsupported,
            (0x68, 0x83) => StatusClass::ErrChainLastCommandExpected,
            (0x68, 0x84) => StatusClass::ErrChainUnsupported,
            // Checking Error - Command not allowed - Nice.
            (y @ 0x69, 0x00) => StatusClass::Generic(y),
            (0x69, 0x81) => StatusClass::ErrIncompatibleFileStructure,
            (0x69, 0x82) => StatusClass::ErrSecurityStatus,
            (0x69, 0x83) => StatusClass::ErrAuthMethodBlocked,
            (0x69, 0x84) => StatusClass::ErrRefDataUnusable,
            (0x69, 0x85) => StatusClass::ErrConditionsNotSatisfied,
            (0x69, 0x86) => StatusClass::ErrNoCurrentEF,
            (0x69, 0x87) => StatusClass::ErrMissingSecureMessagingDOs,
            (0x69, 0x88) => StatusClass::ErrIncorrectSecureMessagingDOs,
            // Wrong parameters
            (y @ 0x6A, 0x00) => StatusClass::Generic(y),
            (0x6A, 0x80) => StatusClass::ErrParamsData,
            (0x6B, 0x00) => StatusClass::ErrParamsP1P2,
            (0x6C, x) => StatusClass::ErrRetryWithLe(x),
            (0x6D, 0x00) => StatusClass::ErrInstruction,
            (0x6E, 0x00) => StatusClass::ErrClass,
            (0x6F, 0x00) => StatusClass::ErrNoIdea,
            (y, x) => StatusClass::Unknown(y, x),
        }
    }
}
