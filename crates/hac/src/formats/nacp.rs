use crate::hexstring::HexData;
use crate::ids::AnyId;
use binrw::{BinRead, BinWrite};
use bitflags::bitflags;
use enum_map::{Enum, EnumMap};

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct ProgramTitle {
    #[brw(pad_size_to = 0x200)]
    #[br(try_map = |s: binrw::NullString| String::from_utf8(s.0))]
    #[bw(map = |s| binrw::NullString(s.clone().into_bytes()))]
    pub name: String,
    #[brw(pad_size_to = 0x100)]
    #[br(try_map = |s: binrw::NullString| String::from_utf8(s.0))]
    #[bw(map = |s| binrw::NullString(s.clone().into_bytes()))]
    pub publisher: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct ApplicationNeighborDetectionClientConfiguration {
    pub send_group_configuration: ApplicationNeighborDetectionGroupConfiguration,
    pub receivable_group_configurations: [ApplicationNeighborDetectionGroupConfiguration; 0x10],
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct ApplicationNeighborDetectionGroupConfiguration {
    pub group_id: u64,
    pub key: HexData<0x10>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct ApplicationJitConfiguration {
    pub flags: JitConfigurationFlag,
    pub memory_size: u64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct RequiredAddOnContentsSetBinaryDescriptor {
    pub descriptors: [u16; 0x20],
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
pub struct AccessibleLaunchRequiredVersionValue {
    pub application_id: [u64; 8],
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum)]
pub enum Language {
    AmericanEnglish = 0,
    BritishEnglish = 1,
    Japanese = 2,
    French = 3,
    German = 4,
    LatinAmericanSpanish = 5,
    Spanish = 6,
    Italian = 7,
    Dutch = 8,
    CanadianFrench = 9,
    Portuguese = 10,
    Russian = 11,
    Korean = 12,
    TraditionalChinese = 13,
    SimplifiedChinese = 14,
    BrazilianPortuguese = 15,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Enum)]
pub enum Organization {
    CERO = 0,
    GRACGCRB = 1,
    GSRMR = 2,
    ESRB = 3,
    ClassInd = 4,
    USK = 5,
    PEGI = 6,
    PEGIPortugal = 7,
    PEGIBBFC = 8,
    Russian = 9,
    ACB = 10,
    OFLC = 11,
    IARCGeneric = 12,
    Unused13 = 13,
    Unused14 = 14,
    Unused15 = 15,
    Unused16 = 16,
    Unused17 = 17,
    Unused18 = 18,
    Unused19 = 19,
    Unused20 = 20,
    Unused21 = 21,
    Unused22 = 22,
    Unused23 = 23,
    Unused24 = 24,
    Unused25 = 25,
    Unused26 = 26,
    Unused27 = 27,
    Unused28 = 28,
    Unused29 = 29,
    Unused30 = 30,
    Unused31 = 31,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum StartupUserAccountValue {
    None = 0,
    Required = 1,
    RequiredWithNetworkServiceAccountAvailable = 2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum UserAccountSwitchLockValue {
    Disable = 0,
    Enable = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum AddOnContentRegistrationTypeValue {
    AllOnLaunch = 0,
    OnDemand = 1,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct AttributeFlagValue(u32);

bitflags! {
    impl AttributeFlagValue : u32
    {
        const DEMO = 1 << 0;
        const RETAIL_INTERACTIVE_DISPLAY = 1 << 1;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u32)]
pub enum ParentalControlFlagValue {
    None = 0,
    FreeCommunication = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum ScreenshotValue {
    Allow = 0,
    Deny = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum VideoCaptureValue {
    Disable = 0,
    Manual = 1,
    Enable = 2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum DataLossConfirmationValue {
    None = 0,
    Required = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum PlayLogPolicyValue {
    Open = 0,
    LogOnly = 1,
    None = 2,
    Closed = 3,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum LogoTypeValue {
    LicensedByNintendo = 0,
    DistributedByNintendo = 1,
    Nintendo = 2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum LogoHandlingValue {
    Auto = 0,
    Manual = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum RuntimeAddOnContentInstallValue {
    Deny = 0,
    AllowAppend = 1,
    AllowAppendButDontDownloadWhenUsingNetwork = 2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum RuntimeParameterDeliveryValue {
    Always = 0,
    AlwaysIfUserStateMatched = 1,
    OnRestart = 2,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum CrashReportValue {
    Deny = 0,
    Allow = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum HdcpValue {
    None = 0,
    Required = 1,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct StartupUserAccountOptionFlagValue(u8);

bitflags! {
    impl StartupUserAccountOptionFlagValue : u8
    {
        const IS_OPTIONAL = 1 << 0;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum PlayLogQueryCapabilityValue {
    None = 0,
    WhiteList = 1,
    All = 2,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct RepairFlagValue(u8);

bitflags! {
  impl RepairFlagValue : u8 {
    const SUPPRESS_GAME_CARD_ACCESS = 1 << 0;
  }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct RequiredNetworkServiceLicenseOnLaunchValue(u8);

bitflags! {
  impl RequiredNetworkServiceLicenseOnLaunchValue : u8 {
    const COMMON = 1 << 0;
  }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct JitConfigurationFlag(u64);

bitflags! {
  impl JitConfigurationFlag : u64 {
    const ENABLED = 1 << 0;
  }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BinRead, BinWrite)]
pub struct PlayReportPermissionValue(u8);

bitflags! {
  impl PlayReportPermissionValue : u8 {
    const TARGET_MARKETING = 1 << 0;
  }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum CrashScreenshotForProdValue {
    Deny = 0,
    Allow = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum CrashScreenshotForDevValue {
    Deny = 0,
    Allow = 1,
}

#[derive(Debug, Clone, Eq, PartialEq, BinRead, BinWrite)]
#[brw(little)]
pub struct ApplicationControlProperty {
    // titles, one for each language
    #[br(map = EnumMap::from_array)]
    #[bw(map = |x| x.clone().into_array())]
    pub title: EnumMap<Language, ProgramTitle>,
    pub isbn: HexData<37>,
    pub startup_user_account: StartupUserAccountValue,
    pub user_account_switch_lock: UserAccountSwitchLockValue,
    pub add_on_content_registration_type: AddOnContentRegistrationTypeValue,
    pub attribute_flag: AttributeFlagValue,
    pub supported_language_flag: u32,
    pub parental_control_flag: ParentalControlFlagValue,
    pub screenshot: ScreenshotValue,
    pub video_capture: VideoCaptureValue,
    pub data_loss_confirmation: DataLossConfirmationValue,
    pub play_log_policy: PlayLogPolicyValue,
    pub presence_group_id: u64,
    #[br(map = EnumMap::from_array)]
    #[bw(map = |x| x.into_array())]
    pub rating_age: EnumMap<Organization, i8>,
    pub display_version: HexData<16>, // TODO: this is a string
    pub add_on_content_base_id: AnyId,
    pub save_data_owner_id: AnyId,
    pub user_account_save_data_size: i64,
    pub user_account_save_data_journal_size: i64,
    pub device_save_data_size: i64,
    pub device_save_data_journal_size: i64,
    pub bcat_delivery_cache_storage_size: i64,
    pub application_error_code_category: HexData<8>,
    pub local_communication_id: [u64; 8],
    pub logo_type: LogoTypeValue,
    pub logo_handling: LogoHandlingValue,
    pub runtime_add_on_content_install: RuntimeAddOnContentInstallValue,
    pub runtime_parameter_delivery: RuntimeParameterDeliveryValue,
    pub reserved30f4: HexData<2>,
    pub crash_report: CrashReportValue,
    pub hdcp: HdcpValue,
    pub seed_for_pseudo_device_id: u64,
    pub bcat_passphrase: HexData<65>,
    pub startup_user_account_option: StartupUserAccountOptionFlagValue,
    pub reserved_for_user_account_save_data_operation: HexData<6>,
    pub user_account_save_data_size_max: i64,
    pub user_account_save_data_journal_size_max: i64,
    pub device_save_data_size_max: i64,
    pub device_save_data_journal_size_max: i64,
    pub temporary_storage_size: i64,
    pub cache_storage_size: i64,
    pub cache_storage_journal_size: i64,
    pub cache_storage_data_and_journal_size_max: i64,
    pub cache_storage_index_max: u16,
    pub reserved318a: u8,
    pub runtime_upgrade: u8,
    pub supporting_limited_licenses: u32,
    pub play_log_queryable_application_id: [u64; 16],
    pub play_log_query_capability: PlayLogQueryCapabilityValue,
    pub repair_flag: RepairFlagValue,
    pub program_index: u8,
    pub required_network_service_license_on_launch_flag: RequiredNetworkServiceLicenseOnLaunchValue,
    pub reserved3214: HexData<4>,
    pub neighbor_detection_client_configuration: ApplicationNeighborDetectionClientConfiguration,
    pub jit_configuration: ApplicationJitConfiguration,
    pub required_add_on_contents_set_binary_descriptors: RequiredAddOnContentsSetBinaryDescriptor,
    pub play_report_permission: PlayReportPermissionValue,
    pub crash_screenshot_for_prod: CrashScreenshotForProdValue,
    pub crash_screenshot_for_dev: CrashScreenshotForDevValue,
    pub contents_availability_transition_policy: u8,
    pub reserved3404: HexData<4>,
    pub accessible_launch_required_version: AccessibleLaunchRequiredVersionValue,
    pub reserved3448: HexData<0xbb8>,
}

impl ApplicationControlProperty {
    pub fn any_title(&self) -> Option<&ProgramTitle> {
        self.title.values().find(|x| !x.name.is_empty())
    }
}
