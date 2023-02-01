use crate as thea_council;
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
};
use frame_system as system;
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use sp_core::{traits::RuntimeCode, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

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
		XcmHnadler: xcm_handler,
		TheaCouncil: thea_council,
		XToken: orml_xtokens
	}
);

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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl thea_council::Config for Test {
	type RuntimeEvent = RuntimeEvent;
}

use frame_support::PalletId;

parameter_types! {
	pub const AssetHandlerPalletId: PalletId = PalletId(*b"XcmHandl");
	pub const WithdrawalExecutionBlockDiff: u32 = 1000;
}

impl xcm_handler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AssetHandlerPalletId = AssetHandlerPalletId;
	type WithdrawalExecutionBlockDiff = WithdrawalExecutionBlockDiff;
}

parameter_type_with_key! {
	pub ParachainMinFee: |location: MultiLocation| -> Option<u128> {
		Some(1u128)
	};
}

use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
	EnsureXcmOrigin, FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset, ParentIsPreset,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	UsingComponents,
};

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
