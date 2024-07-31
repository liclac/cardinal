use crate::Result;
use anyhow::Context;
use cardinal::{atr, emv, iso7816, util};
use owo_colors::{colors, OwoColorize};
use pcsc::Card;
use tap::{TapFallible, TapOptional};
use tracing::{debug, error, trace_span, warn};

pub fn probe(args: &crate::Args, card: &mut Card) -> Result<()> {
    let mut wbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Request buffer.
    let mut rbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Response buffer.

    println!("------------ READER STATE ------------");
    probe_reader(card, &mut rbuf);

    println!("---------- IDENTIFYING CARD ----------");
    let cid = probe_cid(card, &mut wbuf, &mut rbuf)
        .tap_err(|err| warn!("couldn't probe CID: {}", err))
        .ok();
    let atr = probe_atr(card, &mut rbuf)?;

    match args
        .force_standard
        .tap_some(|std| debug!(?std, "Ignoring ATR, using --force-standard"))
        .unwrap_or_else(|| get_atr_card_standard(&atr))
    {
        atr::Standard::FeliCa => {
            println!("--------------- FeliCa ---------------");
            if let Some(cid) = cid {
                crate::probe_felica::probe_felica(card, &mut wbuf, &mut rbuf, &cid)
                    .tap_err(|err| warn!("couldn't probe FeliCa: {}", err))
                    .unwrap_or(());
            } else {
                error!("trying to probe FeliCa card, but we have no CID!");
            }
        }
        _ => {
            println!("-------------- ISO 14443 -------------");
            probe_emv(card, &mut wbuf, &mut rbuf)
                .tap_err(|err| warn!("couldn't probe EMV: {}", err))
                .unwrap_or(false);
        }
    }

    Ok(())
}

fn probe_reader(card: &mut Card, rbuf: &mut [u8]) {
    for attr in [
        pcsc::Attribute::VendorName,
        pcsc::Attribute::VendorIfdType,
        pcsc::Attribute::VendorIfdVersion,
        pcsc::Attribute::VendorIfdSerialNo,
        pcsc::Attribute::ChannelId,
        pcsc::Attribute::AsyncProtocolTypes,
        pcsc::Attribute::DefaultClk,
        pcsc::Attribute::MaxClk,
        pcsc::Attribute::DefaultDataRate,
        pcsc::Attribute::MaxDataRate,
        pcsc::Attribute::MaxIfsd,
        pcsc::Attribute::SyncProtocolTypes,
        pcsc::Attribute::PowerMgmtSupport,
        pcsc::Attribute::UserToCardAuthDevice,
        pcsc::Attribute::UserAuthInputDevice,
        pcsc::Attribute::Characteristics,
        pcsc::Attribute::CurrentProtocolType,
        pcsc::Attribute::CurrentClk,
        pcsc::Attribute::CurrentF,
        pcsc::Attribute::CurrentD,
        pcsc::Attribute::CurrentN,
        pcsc::Attribute::CurrentW,
        pcsc::Attribute::CurrentIfsc,
        pcsc::Attribute::CurrentIfsd,
        pcsc::Attribute::CurrentBwt,
        pcsc::Attribute::CurrentCwt,
        pcsc::Attribute::CurrentEbcEncoding,
        pcsc::Attribute::ExtendedBwt,
        pcsc::Attribute::IccPresence,
        pcsc::Attribute::IccInterfaceStatus,
        pcsc::Attribute::CurrentIoState,
        pcsc::Attribute::AtrString,
        pcsc::Attribute::IccTypePerAtr,
        pcsc::Attribute::EscReset,
        pcsc::Attribute::EscCancel,
        pcsc::Attribute::EscAuthrequest,
        pcsc::Attribute::Maxinput,
        pcsc::Attribute::DeviceUnit,
        pcsc::Attribute::DeviceInUse,
        pcsc::Attribute::DeviceFriendlyName,
        pcsc::Attribute::DeviceSystemName,
        pcsc::Attribute::SupressT1IfsRequest,
    ] {
        if let Ok(v) = card
            .get_attribute(attr, rbuf)
            .tap_err(|err| debug!(?attr, ?err, "Couldn't query reader attribute"))
        {
            match attr {
                _ => println!("{:?} => {}", attr, hex::encode_upper(v)),
            }
        }
    }
}

pub fn pcsc_get_data<'r>(
    card: &mut Card,
    wbuf: &mut [u8],
    rbuf: &'r mut [u8],
    p1: u8,
) -> Result<&'r [u8]> {
    // PCSC pseudo-APDU, doesn't actually talk to the card.
    Ok(util::call_le(card, wbuf, rbuf, 0xFF, 0xCA, p1, 0x00, 0)?)
}

/// Probes the ISO 14443-4 card ID. Only for contactless cards.
/// TODO: This shouldn't print a warning when using a contact reader.
fn probe_cid(card: &mut Card, wbuf: &mut [u8], rbuf: &mut [u8]) -> Result<Vec<u8>> {
    let span = trace_span!("probe_cid");
    let _enter = span.enter();

    let cid = pcsc_get_data(card, wbuf, rbuf, 0x00)
        .context("couldn't query CID")
        .map(|v| v.to_owned())?;
    println!("Card ID: {}", hex::encode_upper(&cid));
    Ok(cid)
}

fn get_atr_card_standard(atr: &atr::ATR) -> atr::Standard {
    // Am I doing Rust right?
    if let Some(atr::HistoricalBytes::TLV(atr::HistoricalBytesTLV {
        initial_access: Some(atr::InitialAccess { standard, .. }),
        ..
    })) = atr.historical_bytes
    {
        standard
    } else {
        atr::Standard::Iso14443a3
    }
}

type ATRColorTS = colors::Cyan;
type ATRColorTDnMask = colors::Yellow;
type ATRColorTDnProtocol = colors::Green;
type ATRColorTXn = colors::Yellow;
type ATRColorHB = colors::Magenta;
type ATRColorTck = colors::Cyan;

/// Probes the ISO 7816 ATR (Answer-to-Reset).
fn probe_atr(card: &mut Card, rbuf: &mut [u8]) -> Result<atr::ATR> {
    let span = trace_span!("probe_atr");
    let _enter = span.enter();

    let raw = card
        .get_attribute(pcsc::Attribute::AtrString, rbuf)
        .context("couldn't read ATR")?;
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
                if let Some(v) = service_data {
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
                };

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

/// Probes the card to figure out if it's an EMV payment card.
fn probe_emv(card: &mut Card, wbuf: &mut [u8], rbuf: &mut [u8]) -> Result<bool> {
    let span = trace_span!("EMV");
    let _enter = span.enter();

    // TODO: Some cards don't have directories; we should fall back to AID spamming.
    println!("┏╸{}", "EMV".italic());
    for app in probe_emv_directory(card, wbuf, rbuf)? {
        debug!(
            adf_name = hex::encode_upper(&app.adf_name),
            label = app.app_label,
            "Probing application..."
        );
        probe_emv_application(card, wbuf, rbuf, app.adf_name)?;
    }
    Ok(false)
}

/// Probes the EMV directory and returns a list of application entries.
fn probe_emv_directory(
    card: &mut Card,
    wbuf: &mut [u8],
    rbuf: &mut [u8],
) -> Result<Vec<emv::DirectoryApplication>> {
    let span = trace_span!("directory");
    let _enter = span.enter();

    debug!("Trying to select EMV directory...");
    let dir = emv::Directory::select(card, wbuf, rbuf)?;

    println!("┗┱─┬╴{}", "Directory".italic());
    println!(" ┃ ├─╴SFI for Elementary File: {}", dir.ef_sfi);
    dir.lang_prefs.as_ref().tap_some(|s| {
        print!(" ┃ ├─╴Preferred Language(s):");
        let mut cursor: &str = s.as_str();
        while cursor.len() >= 2 {
            let (lang, rest) = cursor.split_at(2);
            cursor = rest;
            print!(" {}", lang);
        }
        println!("");
    });
    dir.issuer_code_table_idx
        .tap_some(|v| println!(" ┃ ├─╴Charset: ISO-8859-{}", v));
    dir.fci_issuer_discretionary_data
        .as_ref()
        .tap_some(|v| print_fci_issuer_discretionary_data(v));

    // This should be an iterator, but I immediately start struggling with lifetimes if I try.
    let mut apps: Vec<emv::DirectoryApplication> = vec![];
    for i in 1.. {
        println!(" ┃ │");
        debug!(sfi = dir.ef_sfi, num = i, "Trying next record...");
        match (iso7816::ReadRecord {
            sfi: dir.ef_sfi,
            id: iso7816::RecordID::Number(i),
        })
        .call(card, wbuf, rbuf)
        {
            Err(cardinal::Error::APDU(0x6A, 0x83)) => {
                debug!(sfi = dir.ef_sfi, num = i, "No more records");
                break;
            }
            Err(err) => warn!(sfi = dir.ef_sfi, num = i, "Couldn't query record: {}", err),
            Ok(rsp) => {
                debug!(sfi = dir.ef_sfi, num = i, "Got a record!");
                let rec = emv::DirectoryRecord::parse(rsp.data, &dir)?;
                println!(" ┃ ├┬╴{}", format!("Record #{}", i).italic());
                for (i, app) in rec.entry.applications.iter().enumerate() {
                    apps.push(app.clone());
                    println!(" ┃ │└┬╴{}", format!("Application #{}", i + 1).italic());
                    println!(
                        " ┃ │ ├─╴Application ID: {}",
                        hex::encode_upper(&app.adf_name)
                    );
                    println!(" ┃ │ ├─╴Label: {}", app.app_label);
                    app.app_preferred_name
                        .as_ref()
                        .tap_some(|v| println!(" ┃ │ ├─╴Preferred Name: {}", v));
                    app.app_priority.tap_some(|v| {
                        println!(
                            " ┃ │ ├─╴Priority: {} — needs confirmation: {}",
                            v & 0b0000_1111,
                            (v & 0b1000_0000) >> 7 > 0
                        )
                    });
                    app.dir_discretionary_template.as_ref().tap_some(|v| {
                        println!(
                            " ┃ │ ├─╴Directory Discretionary Template: {}",
                            hex::encode_upper(&v)
                        )
                    });
                }
            }
        };
    }

    println!(" ┃ ╵");
    Ok(apps)
}

fn probe_emv_application(
    card: &mut Card,
    wbuf: &mut [u8],
    rbuf: &mut [u8],
    adf_name: Vec<u8>,
) -> Result<bool> {
    let span = trace_span!("application");
    let _enter = span.enter();

    debug!(
        adf_name = hex::encode_upper(&adf_name),
        "Selecting application..."
    );
    let app = emv::Application::select(card, wbuf, rbuf, &adf_name)?;
    println!(
        " ┠─┬╴Application╺╸{}",
        hex::encode_upper(&adf_name).italic()
    );
    println!(" ┃ ├─╴Label: {}", app.app_label);
    app.app_priority.tap_some(|v| {
        println!(
            " ┃ ├─╴Priority: {} — needs confirmation: {}",
            v & 0b0000_1111,
            (v & 0b1000_0000) >> 7 > 0
        )
    });
    app.lang_prefs.tap_some(|s| {
        print!(" ┃ ├─╴Preferred Language(s):");
        let mut cursor: &str = s.as_str();
        while cursor.len() >= 2 {
            let (lang, rest) = cursor.split_at(2);
            cursor = rest;
            print!(" {}", lang);
        }
        println!("");
    });
    app.issuer_code_table_idx
        .tap_some(|v| println!(" ┃ ├─╴Charset: ISO-8859-{}", v));
    app.app_preferred_name
        .as_ref()
        .tap_some(|v| println!(" ┃ ├─╴Preferred Name: {}", v));

    if app.pdol.is_some() || app.fci_issuer_discretionary_data.is_some() {
        println!(" ┃ │");
    }
    app.pdol.tap_some(|v| {
        println!(" ┃ ├┬╴Data Objects for Processing Options");
        for (tag, _) in v {
            let name = match tag {
                // From: https://neapay.com/online-tools/emv-tags-list.html
                0x9F5C => "DS Requested Operator ID",
                _ => "???",
            };
            println!(" ┃ │├─╴[{:04X}] {}", tag, name);
        }
        println!(" ┃ │╵");
    });
    app.fci_issuer_discretionary_data
        .tap_some(print_fci_issuer_discretionary_data);
    println!(" ┃ ╵");

    Ok(true)
}

fn print_fci_issuer_discretionary_data(v: &emv::FCIIssuerDiscretionaryData) {
    println!(" ┃ ├┬╴FCI Issuer Discretionary Data");
    v.log_entry.tap_some(|(sfi, num)| {
        println!(" ┃ │├─╴Log Entries — SFI: {} — {} records", sfi, num);
    });
    v.app_capability_info.tap_some(|(v1, v2, v3)| {
        println!(
            " ┃ │├─╴Application Capabilitiy Info: {:02X} {:02X} {:02X}",
            v1, v2, v3
        );
    });
    v.ds_id.as_ref().tap_some(|v| {
        println!(" ┃ │├─╴Card Number + Sequence: {}", hex::encode_upper(v));
    });
    v.unknown_9f6e.as_ref().tap_some(|v| {
        println!(" ┃ │├─╴Unknown (9F6E): {}", hex::encode_upper(v));
    });
    v.app_selection_reg_propr_data.as_ref().tap_some(|v| {
        println!(" ┃ │├┬╴Application Selection Proprietary Data");
        for (tag, val) in v.iter() {
            println!(" ┃ ││├─╴{:04X} — {}", tag, hex::encode_upper(val));
        }
        println!(" ┃ ││╵");
    });
    println!(" ┃ │╵");
}
