use crate::probe::pcsc_get_data;
use crate::Result;
use cardinal::felica::{self, Command};
use owo_colors::OwoColorize;
use pcsc::Card;
use tap::TapFallible;
use tracing::{debug, error, trace_span, warn};

pub fn probe_felica(card: &mut Card, wbuf: &mut [u8], rbuf: &mut [u8], cid: &[u8]) -> Result<()> {
    let span = trace_span!("felica");
    let _enter = span.enter();
    println!("┏╸{}", "FeliCa".italic());

    // Hm, the lower 2 bytes of the IDm are the Manufacturer Code, can we decode that?
    let idm0 = felica::cid_to_idm(cid).tap_err(|err| {
        error!(
            ?err,
            cid = hex::encode_upper(cid),
            "CID is not a valid IDm?? this should be impossible??"
        )
    })?;
    println!("┠─╴IDm: {:016X}", idm0);

    // The PMm is a whole thing we can definitely decode.
    pcsc_get_data(card, wbuf, rbuf, 0x01)
        .tap_err(|err| warn!(?err, "Couldn't query PMm? (Not important.)"))
        .tap_ok(|pmm| {
            println!("┠┬╴PMm: {}", hex::encode_upper(pmm));
            println!("┃└┬╴ROM Type: {:02X}", pmm[0]);
            println!("┃ └╴IC Type: {}", felica::ICType::from(pmm[1]));
        })?;

    // A physical FeliCa card can have multiple virtual cards, or Systems.
    println!("┃");
    debug!("Listing services...");
    match (felica::RequestSystemCode { idm: idm0 }.call(card, wbuf, rbuf)) {
        Ok(sys_rsp) => probe_felica_systems(card, wbuf, rbuf, idm0, sys_rsp),
        Err(err) => {
            debug!(
                ?err,
                "Couldn't list services, assuming this is a FeliCa Lite (S)"
            );
            probe_felica_lite_s(card, wbuf, rbuf, idm0)
        }
    }
}

pub fn probe_felica_systems(
    card: &mut Card,
    wbuf: &mut [u8],
    rbuf: &mut [u8],
    idm0: u64,
    sys_rsp: felica::RequestSystemCodeResponse,
) -> Result<()> {
    for (i, sys) in sys_rsp.systems.iter().copied().enumerate() {
        assert!(i < 0b0000_1111); // We can't stuff IDs larger than 4 bits into the IDm.
        if i == 0 {
            print!("┗┳");
        } else {
            print!(" ┣");
        }
        println!("┯╸{} {:04X}╺╸{}", "System".italic(), u16::from(sys), sys);

        let idm = felica::idm_for_service(idm0, i as u8);
        println!(" ┃└┬╴IDm: {:016X}", idm);

        // This should always return Mode 0, but it's a good test command.
        debug!(system = i, "Pinging card...");
        let _ = felica::RequestResponse { idm }
            .call(card, wbuf, rbuf)
            .tap_err(|err| warn!(?err, "Couldn't ping card (RequestResponse)"))
            .tap_ok(|rsp| {
                if rsp.mode != 0 {
                    warn!(mode = rsp.mode, "Expected card to be in Mode 0")
                }
            });

        // Loop through Areas and Services.
        let mut last_service_num = None;
        for idx in 0.. {
            debug!(system = i, idx, "Requesting next area or service...");
            match (felica::SearchServiceCode { idm, idx }.call(card, wbuf, rbuf)?).result {
                Some(felica::SearchServiceCodeResult::Area { code, end }) => {
                    if last_service_num.is_some() {
                        println!(" ┃ │╵");
                        last_service_num = None;
                    }
                    print!(
                        " ┃ ├╴{:04X}-{:04X}╶╴{}",
                        code.number,
                        end.number,
                        "Area".italic()
                    );
                    if code.can_subdivide {
                        print!(" +");
                    }
                    println!("");
                }
                Some(felica::SearchServiceCodeResult::Service(code)) => {
                    // Print the header once per distinct service number.
                    if last_service_num != Some(code.number) {
                        if last_service_num.is_some() {
                            println!(" ┃ │╵");
                        }
                        last_service_num = Some(code.number);
                        println!(" ┃ ├┬╴{:04X} Service: {}", code.number, code.kind);
                    }

                    // Print the subtitle once per access mode (1+ times).
                    if code.is_authenticated {
                        // Request a key for the service. Mostly a sanity check for the Service Code.
                        debug!(code = code.code, "Requesting key for service...");
                        let svcrsp = felica::RequestService {
                            idm,
                            node_codes: vec![code.code],
                        }
                        .call(card, wbuf, rbuf)?;

                        println!(
                            " ┃ │├─╴{:04X}╶╴{}╶╴{}{}",
                            code.code,
                            code.access,
                            "authenticated, key ".italic(),
                            svcrsp
                                .key_versions
                                .first()
                                .copied()
                                .unwrap_or_default()
                                .italic()
                        );
                    } else {
                        println!(" ┃ │├┬╴{:04X}╶╴{}", code.code, code.access);
                        for block_num in 0.. {
                            debug!(svc = code.code, blk = block_num, "Reading block...");
                            let rsp = felica::ReadWithoutEncryption {
                                idm,
                                services: vec![code.code],
                                blocks: vec![felica::BlockListElement {
                                    mode: felica::AccessMode::Normal,
                                    service_idx: 0,
                                    block_num,
                                }],
                            }
                            .call(card, wbuf, rbuf)?;
                            for block in rsp.blocks {
                                if block_num == 0 {
                                    println!(" ┃ ││└┤ {}", hex::encode_upper(&block));
                                } else {
                                    println!(" ┃ ││ │ {}", hex::encode_upper(&block));
                                }
                            }
                            if rsp.status != (0x00, 0x00) {
                                debug!("No more blocks!");
                                break;
                            }
                        }
                    }
                }
                None => {
                    debug!("No more services!");
                    break;
                }
            }
        }

        println!(" ┃ │╵");
        println!(" ┃ ╵");
    }

    Ok(())
}

fn probe_felica_lite_s(card: &mut Card, wbuf: &mut [u8], rbuf: &mut [u8], idm0: u64) -> Result<()> {
    let sys = felica::SystemCode::FeliCaLiteS;
    let idm = felica::idm_for_service(idm0, 0);
    println!("┗┳┯╸{} {:04X}╺╸{}", "System".italic(), u16::from(sys), sys);
    println!(" ┃└┬╴IDm: {:016X}", idm);

    // FeliCa Lite(S) chips have two hardcoded service codes, and can't tell you about them.
    let svc_sys = felica::ServiceCode {
        code: 0x000B,
        number: 1,
        kind: felica::ServiceKind::Random,
        access: felica::ServiceAccess::ReadOnly,
        is_authenticated: false,
    };
    let svc_usr = felica::ServiceCode {
        code: 0x0009,
        number: 2,
        kind: felica::ServiceKind::Random,
        access: felica::ServiceAccess::ReadWrite,
        is_authenticated: false,
    };
    for (i, svc) in [&svc_sys, &svc_usr].iter().enumerate() {
        if i > 0 {
            println!(" ┃ │╵");
        }
        println!(" ┃ ├┬╴{:04X} Service: {}", svc.number, svc.kind);
        println!(" ┃ │├┬╴{:04X}╶╴{}", svc.code, svc.access);
        let blocks = [
            (0x00, "S_PAD0"),
            (0x01, "S_PAD1"),
            (0x02, "S_PAD2"),
            (0x03, "S_PAD3"),
            (0x04, "S_PAD4"),
            (0x05, "S_PAD5"),
            (0x06, "S_PAD6"),
            (0x07, "S_PAD7"),
            (0x08, "S_PAD8"),
            (0x09, "S_PAD9"),
            (0x0A, "S_PAD10"),
            (0x0B, "S_PAD11"),
            (0x0C, "S_PAD12"),
            (0x0D, "S_PAD13"),
            (0x0E, "REG"),
            (0x80, "RC"),
            (0x81, "MAC"),
            (0x82, "ID"),
            (0x83, "D_ID"),
            (0x84, "SER_C"),
            (0x85, "SYS_C"),
            (0x86, "CKV"),
            (0x87, "CK"),
            (0x88, "MC"),
            (0x90, "WCNT"),
            (0x91, "MAC_A"),
            (0x92, "STATE"),
            (0xA0, "CRC_CHK"),
        ];
        for (block_num, block_name) in blocks {
            debug!(
                svc = svc.code,
                blk = block_num,
                name = block_name,
                "Reading block..."
            );
            let rsp = felica::ReadWithoutEncryption {
                idm,
                services: vec![svc.code],
                blocks: vec![felica::BlockListElement {
                    mode: felica::AccessMode::Normal,
                    service_idx: 0,
                    block_num,
                }],
            }
            .call(card, wbuf, rbuf)?;
            if rsp.status == (0x00, 0x00) {
                for block in rsp.blocks {
                    if block_num == 0 {
                        println!(" ┃ ││└┤ [{:7}] {}", block_name, hex::encode_upper(&block));
                    } else {
                        println!(" ┃ ││ │ [{:7}] {}", block_name, hex::encode_upper(&block));
                    }
                }
            } else {
                let placeholder = String::from_utf8(vec![b'?'; 32]).unwrap();
                if block_num == 0 {
                    println!(" ┃ ││└┤ [{:7}] {}", block_name, placeholder);
                } else {
                    println!(" ┃ ││ │ [{:7}] {}", block_name, placeholder);
                }
            }
        }
    }
    println!(" ┃ │╵");
    println!(" ┃ ╵");
    Ok(())
}
