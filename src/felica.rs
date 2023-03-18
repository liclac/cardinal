// Note to self:
// Okay, the ACR 1252-U docs has a pseudo-APDU for "talk to suica" (FF 00 00 00 Lc [command]).
// I don't know how portable this is, but I'm assuming it's a CCID thing. That spec is a mess.
//
// It has a single example I've managed to reproduce with opensc-tool:
// - (Identify card from ATR).
//
// - Read the IDm (same pAPDU as reading the ISO contactless CID):
//     $ opensc-tool -s 'FF CA 00 00 00'
//     Using reader with a card: ACS ACR1252 Reader [ACR1252 Reader PICC] 00 00
//     Sending: FF CA 00 00 00
//     Received (SW1=0x90, SW2=0x00):
//     01 01 0A 10 8E 1B AD 39 .......9
//
// - Use that IDm to send it a command (wtf did I just ask it for?)
//     $ opensc-tool -s 'FF 00 00 00 10 10 06 01 01 0A 10 8E 1B AD 39 01 09 01 01 80 00'
//     Using reader with a card: ACS ACR1252 Reader [ACR1252 Reader PICC] 00 00
//     Sending: FF 00 00 00 10 10 06 01 01 0A 10 8E 1B AD 39 01 09 01 01 80 00
//     Received (SW1=0x90, SW2=0x00):
//     0C 07 01 01 0A 10 8E 1B AD 39 01 A6 .........9..
//
// Okay, so that's CLS=FF, CMD=00, P1=00, P2=00 (FeliCa wrapper pAPDU).
// Lc=0x10 (16), FeliCa payload length is also 0x10 (16)? I guess it includes itself.
// Command 0x06 (Read Without Encryption), the IDm for targeting, then 01 09 01 01 80 00.
//
// 0x06 Read Without Encryption is documented in the FeliCa Users' Manual, Section 4.4.5.
// Structure:
//   Command Code [1] = 0x06
//   IDm          [8]
//   Service Num. [1]
//   Service List [m] (repeated service_num times)
//   Block Num.   [1]
//   Block List   [n] (repeated block_num times)
//
// So I asked it to read Service 0x09, Block 0x01. 0x8000 is a checksum, Section 2.2:
//   Checksum of data length and Packet Data, based on CRC-CCITT (Big Endian)
//   Initial value: 0x0000, Generator polynomial: x^16 + x^12 + x^5 + 1
// Oh gods that equation is making my head spin. I hope it's easier than it looks.
//
// Response: 0C 07 01 01 0A 10 8E 1B AD 39 01 A6
// Length = 0x0C (12), Type = 0x07 (Read w/o Encryption Response), then the IDm again.
// Status = 0x01 0xA6, which... lol that's an error, Illegal Service Code List.
