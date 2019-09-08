cardinal
========

A Swiss army knife, but metaphysical, and for smartcards.

It is two things: a Rust crate for speaking to smartcards, and a CLI interface that uses it.

About the CLI
-------------

The `cardinal` command offers a CLI interface for speaking to a smartcard. This is still very much a WIP.

First of all, the goal is to offer exploratory capabilities that don't require you to know much about the underlying protocol. Put a card in, run a command, it should tell you what's on it, extract all the data it can, and leave you to it.

Second, it offers protocol-specific commands - `cardinal emv ls` to list EMV applications, `cardinal gp install` to install an applet on a GlobalPlatform-compatible card.

About the crate
---------------

The `cardinal` crate offers building blocks for speaking to smartcards, without having to get tangled up in the low-level details of the protocol.

Want to build a payment terminal, gpg-agent or ssh-agent replacement, or a frontend for your own JavaCard hacks? The `cardinal` crate has the the tools for that.

It currently implements PCSC as a transport (pcscd on linux, native APIs on Windows and macOS), but support for other transports can be trivially added.

While the aim is to eventually support every protocol found in the wild, it currently supports:

- ISO 7816 - the basic protocol most smartcards speak
- EMV - payment cards [WIP]

