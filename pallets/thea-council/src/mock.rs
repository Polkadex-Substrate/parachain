use crate as thea_council;
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
};
use frame_system as system;
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use thea_primitives::{AuthorityId, AuthoritySignature};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		XcmHnadler: xcm_helper,
		TheaCouncil: thea_council,
		XToken: orml_xtokens,
		TheaMessageHandler: thea_message_handler
	}
);

parameter_types! {
	pub const TheaMaxAuthorities: u32 = 10;
}

impl thea_message_handler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type TheaId = AuthorityId;
	type Signature = AuthoritySignature;
	type MaxAuthorities = TheaMaxAuthorities;
	type Executor = XcmHnadler;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl thea_council::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MinimumActiveCouncilSize = frame_support::traits::ConstU8<2>;
	type RetainPeriod = ConstU64<7200>; // 24h
}

use frame_support::{traits::AsEnsureOriginWithArg, PalletId};
use frame_system::EnsureSigned;

pub const TOKEN: u128 = 1_000_000_000_000;

parameter_types! {
	pub const ExistentialDeposit: u128 = 1 * TOKEN;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	pub const AssetHandlerPalletId: PalletId = PalletId(*b"XcmHandl");
	pub const WithdrawalExecutionBlockDiff: u32 = 1000;
	pub ParachainId: u32 = 2040;
}

impl xcm_helper::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type AccountIdConvert = ();
	type AssetManager = Assets;
	type AssetCreateUpdateOrigin = EnsureSigned<Self::AccountId>;
	type Executor = TheaMessageHandler;
	type AssetHandlerPalletId = AssetHandlerPalletId;
	type WithdrawalExecutionBlockDiff = WithdrawalExecutionBlockDiff;
	type ParachainId = ParachainId;
	type ParachainNetworkId = frame_support::traits::ConstU8<0>;
}

parameter_types! {
	pub const AssetDeposit: u128 = 100;
	pub const ApprovalDeposit: u128 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u128 = 10;
	pub const MetadataDepositPerByte: u128 = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = u128;
	type AssetIdParameter = codec::Compact<u128>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<Self::AccountId>>;
	type ForceOrigin = EnsureSigned<Self::AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(1u128)
	};
}

use xcm_builder::{FixedWeightBounds, LocationInverter};

use xcm::v1::MultiLocation;

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: u64 = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
	pub Ancestry: xcm::v1::MultiLocation = MultiLocation::default();
}

impl orml_xtokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type CurrencyId = u128;
	type CurrencyIdConvert = ();
	type AccountIdToMultiLocation = ();
	type SelfLocation = ();
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = ();
	type MultiLocationsFilter = ();
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = ();
	type ReserveProvider = AbsoluteReserveProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
