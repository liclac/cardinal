use crate::Result;
use anyhow::Context;
use cardinal::atr;
use owo_colors::{colors, OwoColorize};
use pcsc::Card;
use tap::TapOptional;
use tracing::{debug, trace_span};

pub fn probe(card: &mut Card) -> Result<()> {
    let mut wbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Request buffer.
    let mut rbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Response buffer.

    println!("---------- IDENTIFYING CARD ----------");
    let _atr = probe_atr(card, &mut rbuf)?;

    Ok(())
}

type ATRColorTS = colors::Cyan;
type ATRColorTDnMask = colors::Yellow;
type ATRColorTDnProtocol = colors::Green;
type ATRColorTXn = colors::Yellow;
type ATRColorHB = colors::Magenta;
type ATRColorTck = colors::Cyan;

fn probe_atr(card: &mut Card, rbuf: &mut [u8]) -> Result<atr::ATR> {
    let span = trace_span!("probe_atr");
    let _enter = span.enter();

    let raw = card
        .get_attribute(pcsc::Attribute::AtrString, rbuf)
        .context("Couldn't read ATR: {}")?;
    debug!(atr = format!("{:02X?}", raw), "Raw ATR");

    // Colourise the raw ATR.
    let atr = atr::parse(raw).with_context(|| format!("couldn't parse ATR: {:02X?}", raw))?;
    print!(
        "┏╸{}╺ {:02X} {:01X}{:01X}",
        "ATR".italic(),
        u8::from(atr.ts).fg::<ATRColorTS>(),
        atr.t0.tx1.fg::<ATRColorTDnMask>(),
        atr.t0.k.fg::<ATRColorHB>(),
    );
    for txn in [atr.tx1, atr.tx2, atr.tx3] {
        if txn.ta.is_some() || txn.tb.is_some() || txn.tc.is_some() || txn.td.is_some() {
            print!(" ");
        }
        for ob in [txn.ta, txn.tb, txn.tc] {
            if let Some(b) = ob {
                print!("{:02X}", b.fg::<ATRColorTXn>());
            }
        }
        if let Some(td) = txn.td {
            print!(
                "{:01X}{:01X}",
                td.txn.fg::<ATRColorTDnMask>(),
                u8::from(td.protocol).fg::<ATRColorTDnProtocol>(),
            );
        }
    }
    if let Some(hb) = atr.historical_bytes.as_ref() {
        match hb {
            atr::HistoricalBytes::Status(atr::HistoricalBytesStatus { status, sw1sw2 }) => {
                print!(" {:02X}", 0x10.fg::<ATRColorHB>());
                status.tap_some(|v| print!(" {:02X}", v));
                sw1sw2.tap_some(|v| print!(" {:02X}", v));
            }
            atr::HistoricalBytes::TLV(atr::HistoricalBytesTLV { raw, .. }) => print!(
                " {:02X} {}",
                (0x80.fg::<ATRColorHB>()),
                hex::encode_upper(raw).fg::<ATRColorHB>()
            ),
            atr::HistoricalBytes::Unknown(tag, data) => print!(
                " {:02X} {}",
                tag.fg::<ATRColorHB>(),
                hex::encode_upper(data).fg::<ATRColorHB>()
            ),
        }
    }
    println!(" {:02X}", atr.tck.fg::<ATRColorTck>());

    // TS, T0 are always there.
    println!(
        "┗┱─╴TS {:02X} — {:?} Mode",
        u8::from(atr.ts).fg::<ATRColorTS>(),
        atr.ts.fg::<ATRColorTS>()
    );
    println!(
        " ┠─╴T0 {:01X}{:01X} — {} historical bytes",
        atr.t0.tx1.fg::<ATRColorTDnMask>(),
        atr.t0.k.fg::<ATRColorHB>(),
        atr.t0.k.fg::<ATRColorHB>()
    );

    // Tx1
    if let Some(v) = atr.tx1.ta {
        println!(" ┠╴Ta1 {:02X} — voltage modifier", v.fg::<ATRColorTXn>());
    }
    if let Some(v) = atr.tx1.tb {
        println!(" ┠╴Tb1 {:02X} — timing modifier", v.fg::<ATRColorTXn>());
    }
    if let Some(v) = atr.tx1.tc {
        println!(" ┠╴Tc1 {:02X} — extra guard time", v.fg::<ATRColorTXn>());
    }
    if let Some(v) = atr.tx1.td {
        println!(
            " ┠╴Td1 {:01X}{:01X} — protocol: T={}",
            v.txn.fg::<ATRColorTDnMask>(),
            u8::from(v.protocol).fg::<ATRColorTDnProtocol>(),
            u8::from(v.protocol).fg::<ATRColorTDnProtocol>(),
        );
    }

    // Tx2
    if let Some(v) = atr.tx2.ta {
        println!(" ┠╴Ta2 {:02X} — mode negoation", v.fg::<ATRColorTXn>());
    }
    if let Some(v) = atr.tx2.tb {
        println!(" ┠╴Tb2 {:02X} — voltage modifier", v.fg::<ATRColorTXn>());
    }
    if let Some(v) = atr.tx2.tc {
        println!(
            " ┠╴Tc2 {:02X} — leading edge time [T=0]",
            v.fg::<ATRColorTXn>()
        );
    }
    if let Some(v) = atr.tx2.td {
        println!(
            " ┠╴Td2 {:01X}{:01X} — protocol: T={}",
            v.txn.fg::<ATRColorTDnMask>(),
            u8::from(v.protocol).fg::<ATRColorTDnProtocol>(),
            u8::from(v.protocol).fg::<ATRColorTDnProtocol>(),
        );
    }

    // Tx3
    if let Some(v) = atr.tx3.ta {
        println!(" ┠╴Ta3 {:02X} — IFS [T=1]", v.fg::<ATRColorTXn>());
    }
    if let Some(v) = atr.tx3.tb {
        println!(" ┠╴Tb3 {:02X} — CWI [T=1]", v.fg::<ATRColorTXn>());
    }
    if let Some(v) = atr.tx3.tc {
        println!(
            " ┠╴Tc3 {:02X} — Error detection code [T=1]",
            v.fg::<ATRColorTXn>()
        );
    }
    // Td3 should never be present!
    if let Some(v) = atr.tx3.td {
        println!(
            " ┠╴Td3 {:01X}{:01X} — protocol: T={} {}",
            v.txn.red(),
            u8::from(v.protocol).red(),
            u8::from(v.protocol).fg::<ATRColorTDnProtocol>(),
            "[INVALID!]".red()
        );
    }

    // Historical Bytes - TODO: There's a lot of duplicated magic numbers in here rn.
    if let Some(hb) = atr.historical_bytes.as_ref() {
        match hb {
            atr::HistoricalBytes::Status(atr::HistoricalBytesStatus { status, sw1sw2 }) => {
                print!(" ┠┬╴HB {:02X}", 0x10.fg::<ATRColorHB>());
                status.tap_some(|v| print!(" {:02X}", v));
                sw1sw2.tap_some(|v| print!(" {:02X}", v));
                println!("");

                print!(" ┃└");
                status.tap_some(|v| print!(" status: {:02X}", v));
                sw1sw2.tap_some(|v| print!(" SW1SW2: {:04X}", v));
            }
            atr::HistoricalBytes::TLV(atr::HistoricalBytesTLV {
                raw,
                service_data,
                initial_access,
                pre_issuing_data,
                status,
            }) => {
                println!(
                    " ┠┬╴HB {:02X} {}",
                    (0x80.fg::<ATRColorHB>()),
                    hex::encode_upper(raw).fg::<ATRColorHB>()
                );
                println!(" ┃└──┬ {:02X} — TLV", 0x80.fg::<ATRColorHB>());
                service_data.tap_some(|v| {
                    println!(
                        " ┃   ├──┬ {:} — services: {:02X}",
                        "3X".fg::<ATRColorHB>(),
                        v.fg::<ATRColorHB>()
                    );
                    if v & 0b1000_0000 > 0 {
                        println!(" ┃   │  ├── [1--- ----] — Selection by Full DF Name");
                    }
                    if v & 0b0100_0000 > 0 {
                        println!(" ┃   │  ├── [-1-- ----] — Selection by Partial DF Name");
                    }
                    if v & 0b0010_0000 > 0 {
                        println!(" ┃   │  ├── [--1- ----] — Data available in DIR file");
                    }
                    if v & 0b0001_0000 > 0 {
                        println!(" ┃   │  ├── [---1 ----] — Data available in ATR file");
                    }
                    if v & 0b0000_1000 > 0 {
                        println!(" ┃   │  ├── [---- 1---] — File I/O by READ BINARY");
                    }
                    if v & 0b0000_0100 > 0 {
                        println!(" ┃   │  ├── [---- -1--] — {}", "RESERVED".red());
                    }
                    if v & 0b0000_0010 > 0 {
                        println!(" ┃   │  ├── [---- --1-] — {}", "RESERVED".red());
                    }
                    if v & 0b0000_0001 > 0 {
                        println!(" ┃   │  ├── [---- ---1] — {}", "RESERVED".red());
                    }
                });

                if let Some(ia) = initial_access.as_ref() {
                    println!(" ┃   ├──┬ {:} — initial access", "4X".fg::<ATRColorHB>());

                    // Provider.
                    println!(
                        " ┃   │  ├── {} — provider: {}",
                        hex::encode_upper(ia.rid.id()).fg::<ATRColorHB>(),
                        ia.rid.fg::<ATRColorHB>()
                    );
                    println!(
                        " ┃   │  ├── {:02X} — standard: {}",
                        u8::from(ia.standard).fg::<ATRColorHB>(),
                        ia.standard.fg::<ATRColorHB>()
                    );
                    println!(
                        " ┃   │  ├── {:04X} — card name: {}",
                        u16::from(ia.card_name).fg::<ATRColorHB>(),
                        ia.card_name.fg::<ATRColorHB>()
                    );
                    println!(
                        " ┃   │  └── {:04X} — reserved for future use",
                        ia.rfu.fg::<ATRColorHB>()
                    );
                }
                if let Some(pi) = pre_issuing_data.as_ref() {
                    println!(
                        " ┃   ├─── {:} — pre-issuing data: {}",
                        "6X".fg::<ATRColorHB>(),
                        hex::encode_upper(pi)
                    );
                }
                if let Some(atr::HistoricalBytesStatus { status, sw1sw2 }) = status.as_ref() {
                    print!(" ┃   └─── {:} — status:", "8X".fg::<ATRColorHB>());
                    status.tap_some(|v| print!(" {:02X}", v));
                    sw1sw2.tap_some(|v| print!(" {:02X}", v));
                    println!("");
                }
                println!(" ┃");
            }
            atr::HistoricalBytes::Unknown(tag, data) => {
                println!(
                    " ┠┬╴HB {:02X} {}",
                    tag.fg::<ATRColorHB>(),
                    hex::encode_upper(data).fg::<ATRColorHB>()
                );
                println!(" ┃└╴ {}", "unknown data".red());
            }
        }
    }

    println!(
        " ┖ Tck: {:02X} — checksum",
        u8::from(atr.tck).fg::<ATRColorTck>()
    );
    Ok(atr)
}
