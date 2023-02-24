// Copyright 2021 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Parachain runtime mock.

use codec::{Decode, Encode};
use frame_support::{
	construct_runtime, log, match_types, parameter_types,
	traits::{
		fungibles::{Inspect, Mutate},
		Everything, Nothing,
	},
	weights::{constants::WEIGHT_PER_SECOND, Weight, WeightToFee as WeightToFeeT},
};
use sp_core::{ByteArray, ConstU32, H256};
use sp_runtime::{
	testing::Header,
	traits::{Hash, IdentityLookup},
	AccountId32, Perbill, Permill, SaturatedConversion,
};
use sp_std::prelude::*;
use std::marker::PhantomData;
use xcm::latest::{prelude::*, Weight as XCMWeight};

use frame_support::{
	traits::AsEnsureOriginWithArg,
	weights::{
		constants::ExtrinsicBaseWeight, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use orml_xcm_support::MultiNativeAsset;
use pallet_xcm::XcmPassthrough;
use polkadot_core_primitives::{BlockNumber as RelayBlockNumber, BlockNumber};
use polkadot_parachain::primitives::{
	DmpMessageHandler, Id as ParaId, Sibling, XcmpMessageFormat, XcmpMessageHandler,
};
use sp_runtime::traits::{AccountIdConversion, Convert};
use xcm::VersionedXcm;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedRateOfFungible,
	FixedWeightBounds, LocationInverter, NativeAsset, ParentIsPreset, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue,
	TakeWeightCredit, UsingComponents,
};
use xcm_executor::{traits::ShouldExecute, Assets, Config, XcmExecutor};

pub type AccountId = AccountId32;
pub type Balance = u128;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = WEIGHT_PER_SECOND.saturating_div(4);
	pub const ReservedDmpWeight: Weight = WEIGHT_PER_SECOND.saturating_div(4);
}

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub Ancestry: MultiLocation = Parachain(MsgQueue::parachain_id().into()).into();
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToCallOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const UnitWeightCost: u64 = 1;
	pub KsmPerSecond: (AssetId, u128) = (Concrete(Parent.into()), 1);
	pub const MaxInstructions: u32 = 100;
}

parameter_types! {
	pub const AssetHandlerPalletId: PalletId = PalletId(*b"XcmHandl");
}

pub type XcmRouter = super::ParachainXcmRouter<MsgQueue>;

pub type Barrier = (

	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

//TODO: move DenyThenTry to polkadot's xcm module.
/// Deny executing the xcm message if it matches any of the Deny filter regardless of anything else.
/// If it passes the Deny, and matches one of the Allow cases then it is let through.
pub struct DenyThenTry<Deny, Allow>(PhantomData<Deny>, PhantomData<Allow>)
where
	Deny: ShouldExecute,
	Allow: ShouldExecute;

impl<Deny, Allow> ShouldExecute for DenyThenTry<Deny, Allow>
where
	Deny: ShouldExecute,
	Allow: ShouldExecute,
{
	fn should_execute<RuntimeCall>(
		origin: &MultiLocation,
		message: &mut Xcm<RuntimeCall>,
		max_weight: XCMWeight,
		weight_credit: &mut XCMWeight,
	) -> Result<(), ()> {
		panic!("here");
		Deny::should_execute(origin, message, max_weight, weight_credit)?;
		Allow::should_execute(origin, message, max_weight, weight_credit)
	}
}

use smallvec::smallvec;
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// Extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
		let p = 10_000_000_000;
		let q = 10 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

parameter_types! {
	pub PdexLocation: MultiLocation = Here.into();
}

use polkadot_runtime_common::impls::ToAuthor;
pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = XcmHandler;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = (
		UsingComponents<WeightToFee, PdexLocation, AccountId, Balances, ()>,
		ForeignAssetFeeHandler<
			WeightToFee,
			RevenueCollector<
				AssetsPallet,
				XcmHandler,
				MockedAMM<AccountId, u128, u128, u64>,
				TypeConv,
				TypeConv,
			>,
			MockedAMM<AccountId, u128, u128, u64>,
			XcmHandler,
		>,
	);
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = ();
}

#[frame_support::pallet]
pub mod mock_msg_queue {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn parachain_id)]
	pub(super) type ParachainId<T: Config> = StorageValue<_, ParaId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn received_dmp)]
	/// A queue of received DMP messages
	pub(super) type ReceivedDmp<T: Config> = StorageValue<_, Vec<Xcm<T::RuntimeCall>>, ValueQuery>;

	impl<T: Config> Get<ParaId> for Pallet<T> {
		fn get() -> ParaId {
			Self::parachain_id()
		}
	}

	pub type MessageId = [u8; 32];

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// XCMP
		/// Some XCM was executed OK.
		Success(Option<T::Hash>),
		/// Some XCM failed.
		Fail(Option<T::Hash>, XcmError),
		/// Bad XCM version used.
		BadVersion(Option<T::Hash>),
		/// Bad XCM format used.
		BadFormat(Option<T::Hash>),

		// DMP
		/// Downward message is invalid XCM.
		InvalidFormat(MessageId),
		/// Downward message is unsupported version of XCM.
		UnsupportedVersion(MessageId),
		/// Downward message executed with the given outcome.
		ExecutedDownward(MessageId, Outcome),
	}

	impl<T: Config> Pallet<T> {
		pub fn set_para_id(para_id: ParaId) {
			ParachainId::<T>::put(para_id);
		}

		fn handle_xcmp_message(
			sender: ParaId,
			_sent_at: RelayBlockNumber,
			xcm: VersionedXcm<T::RuntimeCall>,
			max_weight: Weight,
		) -> Result<Weight, XcmError> {
			//assert_eq!("hello", "no_hello");
			let hash = Encode::using_encoded(&xcm, T::Hashing::hash);
			let (result, event) = match Xcm::<T::RuntimeCall>::try_from(xcm) {
				Ok(xcm) => {
					let location = (1, Parachain(sender.into()));
					match T::XcmExecutor::execute_xcm(location, xcm, max_weight.ref_time()) {
						Outcome::Error(e) => (Err(e), Event::Fail(Some(hash), e)),
						Outcome::Complete(w) =>
							(Ok(Weight::from_ref_time(w)), Event::Success(Some(hash))),
						// As far as the caller is concerned, this was dispatched without error, so
						// we just report the weight used.
						Outcome::Incomplete(w, e) =>
							(Ok(Weight::from_ref_time(w)), Event::Fail(Some(hash), e)),
					}
				},
				Err(()) => (Err(XcmError::UnhandledXcmVersion), Event::BadVersion(Some(hash))),
			};
			Self::deposit_event(event);
			result
		}
	}

	impl<T: Config> XcmpMessageHandler for Pallet<T> {
		fn handle_xcmp_messages<'a, I: Iterator<Item = (ParaId, RelayBlockNumber, &'a [u8])>>(
			iter: I,
			max_weight: Weight,
		) -> Weight {
			for (sender, sent_at, data) in iter {
				let mut data_ref = data;
				let _ = XcmpMessageFormat::decode(&mut data_ref)
					.expect("Simulator encodes with versioned xcm format; qed");

				let mut remaining_fragments = data_ref;
				while !remaining_fragments.is_empty() {
					if let Ok(xcm) =
						VersionedXcm::<T::RuntimeCall>::decode(&mut remaining_fragments)
					{
						let _ = Self::handle_xcmp_message(sender, sent_at, xcm, max_weight);
					} else {
						debug_assert!(false, "Invalid incoming XCMP message data");
					}
				}
			}
			max_weight
		}
	}

	impl<T: Config> DmpMessageHandler for Pallet<T> {
		fn handle_dmp_messages(
			iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>,
			limit: Weight,
		) -> Weight {
			//assert_eq!("hello", "no_hello");
			for (_i, (_sent_at, data)) in iter.enumerate() {
				let id = sp_io::hashing::blake2_256(&data[..]);
				let maybe_msg = VersionedXcm::<T::RuntimeCall>::decode(&mut &data[..])
					.map(Xcm::<T::RuntimeCall>::try_from);
				match maybe_msg {
					Err(_) => {
						Self::deposit_event(Event::InvalidFormat(id));
					},
					Ok(Err(())) => {
						Self::deposit_event(Event::UnsupportedVersion(id));
					},
					Ok(Ok(x)) => {
						let outcome =
							T::XcmExecutor::execute_xcm(Parent, x.clone(), limit.ref_time());
						<ReceivedDmp<T>>::append(x);
						Self::deposit_event(Event::ExecutedDownward(id, outcome));
					},
				}
			}
			limit
		}
	}
}

impl mock_msg_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

parameter_types! {
	pub const WithdrawalExecutionBlockDiff: u32 = 7000;
	pub const XcmHandlerId: PalletId = PalletId(*b"XcmHandl");
	pub ParachainId: u32 = MsgQueue::parachain_id().into();
	pub const ParachainNetworkId: u8 = 1;
}

impl xcm_handler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type AccountIdConvert = LocationToAccountId;
	type AssetManager = AssetsPallet;
	type AssetCreateUpdateOrigin = EnsureRoot<AccountId>;
	type AssetHandlerPalletId = AssetHandlerPalletId;
	type WithdrawalExecutionBlockDiff = WithdrawalExecutionBlockDiff;
	type ParachainId = ParachainId;
	type ParachainNetworkId = ParachainNetworkId;
}

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u128;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(Junction::AccountId32 { network: NetworkId::Any, id: account.into() }).into()
	}
}

parameter_types! {
	//pub ParaId: ParaId = mock_msg_queue::<Runtime>::get();
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(MsgQueue::parachain_id().into()))); //TODO: CHnage to our Parachin Id
	pub const BaseXcmWeight: XCMWeight = 100_000_000; // TODO: recheck this
	pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(1u128)
	};
}

pub struct CurrencyIdConvert;

impl Convert<u128, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(_a: u128) -> Option<MultiLocation> {
		Some(MultiLocation::default())
	}
}

impl Convert<MultiLocation, Option<u128>> for CurrencyIdConvert {
	fn convert(_a: MultiLocation) -> Option<u128> {
		Some(200)
	}
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = u128;
	type CurrencyIdConvert = CurrencyIdConvert;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = SelfLocation;
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type MultiLocationsFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = AbsoluteReserveProvider;
}

//Install Swap pallet
parameter_types! {
	pub const SwapPalletId: PalletId = PalletId(*b"sw/accnt");
	pub DefaultLpFee: Permill = Permill::from_rational(30u32, 10000u32);
	pub OneAccount: AccountId = AccountId::from([1u8; 32]);
	pub DefaultProtocolFee: Permill = Permill::from_rational(0u32, 10000u32);
	pub const MinimumLiquidity: u128 = 1_000u128;
	pub const MaxLengthRoute: u8 = 10;
}

impl pallet_amm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Assets = AssetHandler;
	type PalletId = SwapPalletId;
	type LockAccountId = OneAccount;
	type CreatePoolOrigin = EnsureRoot<Self::AccountId>;
	type ProtocolFeeUpdateOrigin = EnsureRoot<Self::AccountId>;
	type LpFee = DefaultLpFee;
	type MinimumLiquidity = MinimumLiquidity;
	type MaxLengthRoute = MaxLengthRoute;
	type GetNativeCurrencyId = NativeCurrencyId;
}

parameter_types! {
	pub const NativeCurrencyId: u128 = 0;
}

impl asset_handler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MultiCurrency = AssetsPallet;
	type NativeCurrencyId = NativeCurrencyId;
}

//Install Router pallet
parameter_types! {
	pub const RouterPalletId: PalletId = PalletId(*b"rw/accnt");
}

impl router::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = RouterPalletId;
	type AMM = Swap;
	type MaxLengthRoute = MaxLengthRoute;
	type GetNativeCurrencyId = NativeCurrencyId;
	type Assets = AssetHandler;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		MsgQueue: mock_msg_queue::{Pallet, Storage, Event<T>},
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
		XTokens: orml_xtokens::{Pallet, Call, Event<T>},
		XcmHandler: xcm_handler::{Pallet, Call, Storage, Event<T>},
		AssetsPallet: pallet_assets::{Pallet, Call, Storage, Event<T>},
		Swap: pallet_amm::pallet::{Pallet, Call, Storage, Event<T>},
		Router: router::pallet::{Pallet, Call, Storage, Event<T>},
		AssetHandler: asset_handler::pallet::{Pallet, Storage, Event<T>}
	}
);

pub struct ForeignAssetFeeHandler<T, R, AMM, AC>
where
	T: WeightToFeeT<Balance = u128>,
	R: TakeRevenue,
	AMM: support::AMM<AccountId, u128, Balance, u64>,
	AC: AssetIdConverter,
{
	/// Total used weight
	weight: u64,
	/// Total consumed assets
	consumed: u128,
	/// Asset Id (as MultiLocation) and units per second for payment
	asset_location_and_units_per_second: Option<(MultiLocation, u128)>,
	_pd: PhantomData<(T, R, AMM, AC)>,
}

use crate::mock_amm::MockedAMM;
use sp_std::vec;
use xcm_executor::traits::WeightTrader;
use xcm_handler::AssetIdConverter;

impl<T, R, AMM, AC> WeightTrader for ForeignAssetFeeHandler<T, R, AMM, AC>
where
	T: WeightToFeeT<Balance = u128>,
	R: TakeRevenue,
	AMM: support::AMM<AccountId, u128, Balance, u64>,
	AC: AssetIdConverter,
{
	fn new() -> Self {
		Self { weight: 0, consumed: 0, asset_location_and_units_per_second: None, _pd: PhantomData }
	}

	fn buy_weight(
		&mut self,
		weight: u64,
		payment: Assets,
	) -> sp_std::result::Result<Assets, XcmError> {
		// Calculate weight to fee
		let fee_in_native_token =
			T::weight_to_fee(&frame_support::weights::Weight::from_ref_time(weight));
		// Check if
		let payment_asset = payment.fungible_assets_iter().next().ok_or(XcmError::TooExpensive)?;
		if let AssetId::Concrete(location) = payment_asset.id {
			let foreign_currency_asset_id =
				AC::convert_location_to_asset_id(location.clone()).ok_or(XcmError::TooExpensive)?;
			let path = vec![NativeCurrencyId::get(), foreign_currency_asset_id];

			let expected_fee_in_foreign_currency = AMM::get_amounts_in(fee_in_native_token, path)
				.map_err(|_| XcmError::TooExpensive)?;
			let expected_fee_in_foreign_currency =
				expected_fee_in_foreign_currency.iter().next().ok_or(XcmError::TooExpensive)?;
			let unused = payment
				.checked_sub((location.clone(), *expected_fee_in_foreign_currency).into())
				.map_err(|_| XcmError::TooExpensive)?;
			self.weight = self.weight.saturating_add(weight);
			if let Some((old_asset_location, _)) = self.asset_location_and_units_per_second.clone()
			{
				if old_asset_location == location.clone() {
					self.consumed = self
						.consumed
						.saturating_add((*expected_fee_in_foreign_currency).saturated_into());
				}
			} else {
				self.consumed = self
					.consumed
					.saturating_add((*expected_fee_in_foreign_currency).saturated_into());
				self.asset_location_and_units_per_second = Some((location, 0));
			}
			Ok(unused)
		} else {
			Err(XcmError::TooExpensive)
		}
	}
}

impl<T, R, AMM, AC> Drop for ForeignAssetFeeHandler<T, R, AMM, AC>
where
	T: WeightToFeeT<Balance = u128>,
	R: TakeRevenue,
	AMM: support::AMM<AccountId, u128, Balance, u64>,
	AC: AssetIdConverter,
{
	fn drop(&mut self) {
		if let Some((asset_location, _)) = self.asset_location_and_units_per_second.clone() {
			if self.consumed > 0 {
				R::take_revenue((asset_location, self.consumed).into());
			}
		}
	}
}

pub struct TypeConv;
impl<Source: TryFrom<Dest> + Clone, Dest: TryFrom<Source> + Clone>
	xcm_executor::traits::Convert<Source, Dest> for TypeConv
{
	fn convert(value: Source) -> Result<Dest, Source> {
		Dest::try_from(value.clone()).map_err(|_| value)
	}
}

pub struct RevenueCollector<AM, AC, AMM, AssetConv, BalanceConv>
where
	AM: Mutate<sp_runtime::AccountId32> + Inspect<sp_runtime::AccountId32>,
	AC: AssetIdConverter,
	AMM: support::AMM<sp_runtime::AccountId32, u128, Balance, u64>,
	AssetConv: xcm_executor::traits::Convert<u128, AM::AssetId>,
	BalanceConv: xcm_executor::traits::Convert<u128, AM::Balance>,
{
	_pd: sp_std::marker::PhantomData<(AM, AC, AMM, AssetConv, BalanceConv)>,
}

impl<AM, AC, AMM, AssetConv, BalanceConv> TakeRevenue
	for RevenueCollector<AM, AC, AMM, AssetConv, BalanceConv>
where
	AM: Mutate<sp_runtime::AccountId32> + Inspect<sp_runtime::AccountId32>,
	AC: AssetIdConverter,
	AMM: support::AMM<sp_runtime::AccountId32, u128, Balance, u64>,
	AssetConv: xcm_executor::traits::Convert<u128, AM::AssetId>,
	BalanceConv: xcm_executor::traits::Convert<u128, AM::Balance>,
{
	fn take_revenue(revenue: MultiAsset) {
		if let AssetId::Concrete(location) = revenue.id {
			if let (Some(asset_id_u128), Fungibility::Fungible(amount)) =
				(AC::convert_location_to_asset_id(location), revenue.fun)
			{
				let asset_handler_account = AssetHandlerPalletId::get().into_account_truncating(); //TODO: Change account
				let asset_id = AssetConv::convert_ref(asset_id_u128).unwrap();
				AM::mint_into(
					asset_id,
					&asset_handler_account,
					BalanceConv::convert_ref(1_000_000_000_000_000).unwrap(),
				)
				.expect("TODO: panic message"); //TODO: Print Error log
				AMM::swap(&asset_handler_account, (asset_id_u128, NativeCurrencyId::get()), amount)
					.expect("TODO: panic message"); // TODO Print Error log
			}
		}
	}
}
