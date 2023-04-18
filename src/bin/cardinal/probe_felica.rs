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
    let sys_rsp = felica::RequestSystemCode { idm: idm0 }.call(card, wbuf, rbuf)?;
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
