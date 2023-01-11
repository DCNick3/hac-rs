A library to read some formats used by Nintendo Switch operating system.

Mostly based on code in [LibHac](https://github.com/Thealexbarney/LibHac)

## Supported formats

### NCAs
Encrypted & container format for games and updates

Decryption & integrity verification are supported, though the header signature is not yet checked.  

No support for BKTR patching and AES-XTS encryption (which NCAs even use it?).

### PartitionFs
Filesystem used for ExeFS, metadata storage.

### RomFs
Filesystem used to store game assets. 

Note that while the RomFs itself is supported, no support is provided for BKTR patching, so currently it is not possible to read RomFs from updates.

### CNMT
CNMT are used to store metadata about titles and applications, prior to the downloading of the content.

The parsed CNMT is fully represented with nice rusty structs and enums.

### NACP
NACP are used to store metadata about titles and applications, after the downloading of the content.
They include a lot of information about what the application can and can't do.

All this information is available as rust structs.

### Ticket
Tickets are used to store metadata about the rights of a user to access a title.

There is support of extracting the rights ID and the title key from the ticket, if it is not personalized (Personalized stuff requires decryption of RSA messages and is not supported yet).

### NSP
NSP are actually just a PartitionFs with NCAs and stuff inside.
There is a `SwitchFs` struct, which allows to load NCAs from a filesystem. 
It parses all the metadata files (CNMTs and NACPs) and makes them available as a list of titles and applications.

## API

The code handling the above formats is actually agnostic of the storage it is operating on.

It uses `ReadableStorage` trait to represent raw byte storage, allowing `read(offset, size)` operations on it. There is also `ReadableBlockStorage`, which constraints operations to only work with full blocks, which is useful for crypto and integrity verification stuff.

Finally, there is a `ReadableFileSystem`, along with `ReadableFile`, `ReadableDirectory` trait, which allow to represent _some_ filesystem.

You can open sections of NCAs as `ReadableFileSystem`, without caring about the actual file system in use.

The API surface is definitely not final & stable yet, I am willing to make changes that would make it nicer.

## Cli

There are plans to make __some__ CLI tool, but what exist now is very bare-bones.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.