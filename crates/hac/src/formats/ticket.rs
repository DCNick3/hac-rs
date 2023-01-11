use crate::crypto::keyset::KeySet;
use crate::crypto::TitleKey;
use crate::hexstring::HexData;
use crate::ids::RightsId;
use binrw::{BinRead, BinWrite, NullString};
use bitflags::bitflags;

#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[repr(u32)]
pub enum Signature {
    #[brw(magic = 0x10000u32)]
    Rsa4096Sha1(#[brw(pad_after = 0x3c)] HexData<0x200>),
    #[brw(magic = 0x10001u32)]
    Rsa2048Sha1(#[brw(pad_after = 0x3c)] HexData<0x100>),
    #[brw(magic = 0x10002u32)]
    EcdsaSha1(#[brw(pad_after = 0x40)] HexData<0x3c>),
    #[brw(magic = 0x10003u32)]
    Rsa4096Sha256(#[brw(pad_after = 0x3c)] HexData<0x200>),
    #[brw(magic = 0x10004u32)]
    Rsa2048Sha256(#[brw(pad_after = 0x3c)] HexData<0x100>),
    #[brw(magic = 0x10005u32)]
    EcdsaSha256(#[brw(pad_after = 0x40)] HexData<0x3c>),
}

#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum TitleKeyType {
    Common,
    Personalized,
}

#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum LicenseType {
    Permanent,
    Demo,
    Trial,
    Rental,
    Subscription,
    Service,
}

bitflags! {
    #[derive(BinRead, BinWrite)]
    pub struct PropertyFlags: u32 {
        const PRE_INSTALL = 1 << 0;
        const SHARED_TITLE = 1 << 1;
        const ALLOW_ALL_CONTENT = 1 << 2;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct Ticket {
    pub signature: Signature,
    #[brw(pad_size_to = 0x40)]
    pub issuer: NullString,
    pub title_key_block: HexData<0x100>,
    pub format_version: u8,
    pub title_key_type: TitleKeyType,
    pub ticket_version: u16,
    pub license_type: LicenseType,
    pub crypto_type: u8,
    pub property_flags: PropertyFlags,
    #[brw(pad_before = 0x6)]
    pub ticket_id: u64,
    pub device_id: u64,
    pub rights_id: RightsId,
    pub account_id: u32,
    pub sect_total_size: u32,
    pub sect_header_offset: u32,
    pub sect_num: u16,
    pub sect_entry_size: u16,
}

impl Ticket {
    pub fn title_key(&self, _keyset: &KeySet) -> TitleKey {
        match self.title_key_type {
            TitleKeyType::Common => {
                let mut title_key = [0; 0x10];
                title_key.copy_from_slice(&self.title_key_block.0[..0x10]);
                TitleKey::from(title_key)
            }
            TitleKeyType::Personalized => todo!("Decrypt personalized title key"),
        }
    }
}
