//! Reverse engineered Japan Railway Cybernetics Association standards.
//!
//! These structures are used by common transit cards in Japan, but there's no official
//! public documentation, so this is all based on reverse-engineering.
//!
//! https://www.wdic.org/w/RAIL/IC%E3%82%AB%E3%83%BC%E3%83%89%E4%B9%97%E8%BB%8A%E5%88%B8
//! https://ja.osdn.net/projects/felicalib/wiki/suica
//!
//! Station codes: https://www.denno.net/SFCardFan/ (offline as of writing, but on archive.org)
use chrono::{DateTime, TimeZone, Utc};
use nom::combinator::map;
use nom::number::complete::{be_u16, be_u8};
use num_enum::FromPrimitive;

use super::IResult;

// I do not know Japanesa rail terminology, assume I've mistranslated all of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum TerminalType {
    FareAdjustmentMachine = 3, // "精算機"
    HandheldTerminal = 4,      // "携帯型端末"
    OnboardTerminal = 5,       // "車載端末"
    #[num_enum(alternatives=[8,18,20,21])]
    TicketMachine = 7, // "券売機"
    DepositMachine = 9,        // "入金機 (??)"
    FareGate = 22,             // "改札機"
    SimpleFareGate = 23,       // "簡易改札機", what's so simple about it?
    #[num_enum(alternatives=[25])]
    CounterTerminal = 24, // "窓口端末"
    FareGateTerminal = 26,     // "改札端末"
    MobilePhone = 27,          // "携帯電話"
    TransferMachine = 28,      // "乗継精算機"
    ContactFareGate = 29,      // "連絡改札機", could also be connection/connecting?
    SimpleDepositMachine = 31, // "簡易入金機"
    #[num_enum(alternatives=[72])]
    ViewAltte = 70, // "VIEW ALTTE" (???)
    ProductSalesTerminal = 199, // "物販端末"
    VendingMachine = 200,      // "自販機"
    #[num_enum(catch_all)]
    Unknown(u8),
}

// I do not know Japanesa rail terminology, assume I've mistranslated all of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum TransactionType {
    ExitFareGate = 1,                  // "運賃支払(改札出場)"
    Charge = 2,                        // "チャージ"
    TicketPurchaseMagnetic = 3,        // "券購(磁気券購入)"
    Adjustment = 4,                    // "精算"
    AdjustmentAtEntrance = 5,          // "精算 (入場精算)"
    AttendantExit = 6, // "窓出 (改札窓口処理)", asking station attendant to let you out?
    NewIssue = 7,      // "新規 (新規発行)"
    AttendantDebit = 8, // "控除 (窓口控除)", charge by station attendant?
    BusPiTaPa = 13,    // "バス (PiTaPa系)"
    BusIruCa = 15,     // "バス (IruCa系)"
    Recurring = 17,    // "再発 (再発行処理)"
    Shinkansen = 19,   // " 支払 (新幹線利用)"
    EntranceAutoCharge = 20, // "入A (入場時オートチャージ)"
    ExitAutoCharge = 21, // " 出A (出場時オートチャージ)"
    TopUpBusCharge = 31, // "入金 (バスチャージ)", refund for bus fare?
    TicketPurchaseSpecialBusTram = 35, // "券購 (バス路面電車企画券購入)"
    ProductSale = 70,  // "物販"
    Privilege = 72,    // "特典 (特典チャージ)" (??)
    TopUpCash = 73,    // "入金 (レジ入金)"
    RefundGoods = 74,  // "物販取消"
    PurchaseGoods = 75, // "入物 (入場物販)"
    Reality = 198,     // "物現 (現金併用物販)", what on earth??
    Purchase = 203,    // "入物 (入場現金併用物販)"
    AdjustmentThirdParty = 132, // "精算 (他社精算)"
    AdjustmentThirdPartyFare = 133, // "精算 (他社入場精算)"
    #[num_enum(catch_all)]
    Unknown(u8),
}

/// Historical record (also known as an Entry/Exit record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryRecord {
    pub terminal_type: TerminalType,
    pub tx_type: TransactionType,
    pub unknown: u16,        // ???
    pub date: DateTime<Utc>, // Somehow, I suspect this will be in JST, not UTC.
}

impl HistoryRecord {
    pub fn parse(data: &[u8]) -> IResult<Self> {
        let (data, terminal_type) = map(be_u8, |v| v.into())(data)?;
        let (data, tx_type) = map(be_u8, |v| v.into())(data)?;
        let (data, unknown) = be_u16(data)?;
        let (data, date) = map(be_u16, |v| {
            Utc.with_ymd_and_hms(
                (((v >> 9) & 0x007f) + 2000).into(),
                ((v >> 5) & 0x000f).into(),
                (v & 0x01f).into(),
                0,
                0,
                0,
            )
            .unwrap()
        })(data)?;
        Ok((
            data,
            Self {
                terminal_type,
                tx_type,
                unknown,
                date,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_record_vending_machine_384yen() {
        assert_eq!(
            HistoryRecord::parse(&[
                0xC8, // Vending Machine
                0x46, // Product Sale
                0x00, 0x00, // Mystery Parfait
                0x27, 0x77, // 2019-11-22
                0x31, 0x2B, // Time?
                0x20, 0x21, // Product or vending machine ID?
                0x52, 0x03, // Remaining Balance (little endian, 0x0352 => ¥850)
                0x00, 0x00, 0x72, // Transaction Sequence Number 114
                0x00  // Region
            ])
            .map(|(_, v)| v)
            .unwrap(),
            HistoryRecord {
                terminal_type: TerminalType::VendingMachine,
                tx_type: TransactionType::ProductSale,
                unknown: 0x0000_0000,
                date: Utc.with_ymd_and_hms(2019, 11, 23, 0, 0, 0).unwrap(),
            }
        )
    }

    #[test]
    fn test_history_record_travel_odakyu_line() {
        // [111] 2019-11-22: 15:00 Tokidaigaku-Mae -> 15:16 Hon-Atsugi, ¥220 (¥2.329 left).
        assert_eq!(
            HistoryRecord::parse(&[
                0x16, // Fare Gate
                0x01, // Exit Fare Gate
                0x00, 0x02, // Mystery Parfait
                0x27, 0x76, // 2019-11-22
                0xE0, 0x2E, // Enter (Line, Station)
                0xE0, 0x27, // Exit (Line, Station)
                0x19, 0x09, // Remaining Balance (little endian, 0x0919 => ¥2329)
                0x00, 0x00, 0x6F, // Transaction Sequence Number 111
                0x00  // Region
            ])
            .map(|(_, v)| v)
            .unwrap(),
            HistoryRecord {
                terminal_type: TerminalType::FareGate,
                tx_type: TransactionType::ExitFareGate,
                unknown: 0x0000_0002,
                date: Utc.with_ymd_and_hms(2019, 11, 22, 0, 0, 0).unwrap(),
            }
        )
    }
}
