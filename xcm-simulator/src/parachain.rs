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

use std::marker::PhantomData;
use codec::{Decode, Encode};
use frame_support::{construct_runtime, log, match_types, parameter_types, traits::{Everything, Nothing}, weights::{constants::WEIGHT_PER_SECOND, Weight}};
use frame_support::dispatch::RawOrigin;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{Hash, IdentityLookup},
    AccountId32,
};
use sp_std::prelude::*;
use xcm::latest::{prelude::*, Weight as XCMWeight};

use pallet_xcm::XcmPassthrough;
use polkadot_core_primitives::BlockNumber as RelayBlockNumber;
use polkadot_parachain::primitives::{
    DmpMessageHandler, Id as ParaId, Sibling, XcmpMessageFormat, XcmpMessageHandler,
};
use xcm::VersionedXcm;
use xcm_builder::{AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset, ParentIsPreset, SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit};
use xcm_executor::{Config, XcmExecutor};
use xcm_executor::traits::ShouldExecute;
use crate::parachain;
use frame_support::PalletId;

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

pub type LocalAssetTransactor =
XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, LocationToAccountId, AccountId, ()>;

pub type XcmRouter = super::ParachainXcmRouter<MsgQueue>;
pub type Barrier = DenyThenTry<
    DenyReserveTransferToRelayChain,
    (
        TakeWeightCredit,
        AllowTopLevelPaidExecutionFrom<Everything>,
        AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
        // ^^^ Parent and its exec plurality get free execution
    ),
>;

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
        Deny::should_execute(origin, message, max_weight, weight_credit)?;
        Allow::should_execute(origin, message, max_weight, weight_credit)
    }
}

pub struct DepositEvent {
    pub deposit_amount: u128,
    pub recipient: AccountId32
}

impl DepositEvent {
    pub fn new() -> Self {
        Self {
            deposit_amount: 0,
            recipient: AccountId32::new([0;32])
        }
    }

    pub fn set_deposit_amount(&mut self, deposit_amount: u128) {
        self.deposit_amount = deposit_amount;
    }

    pub fn get_recipient(asset: &MultiLocation) -> Option<AccountId32> {
        match asset {
            MultiLocation {parents:_, interior: X1(Junction::AccountId32{network:_, id })} => {
                Some(AccountId32::from(id.clone()))
            }
            _ => {
                None
            }
        }
    }

    pub fn set_recipient(&mut self, recipient: AccountId32) {
        self.recipient = recipient;
    }
}

// See issue #5233
pub struct DenyReserveTransferToRelayChain;
impl ShouldExecute for DenyReserveTransferToRelayChain {
    fn should_execute<RuntimeCall>(
        origin: &MultiLocation,

        message: &mut Xcm<RuntimeCall>,
        _max_weight: XCMWeight,
        _weight_credit: &mut XCMWeight,
    ) -> Result<(), ()> {
        if message.0.iter().any(|inst| {
            matches!(
				inst,
				InitiateReserveWithdraw {
					reserve: MultiLocation { parents: 1, interior: Here },
					..
				} | DepositReserveAsset { dest: MultiLocation { parents: 1, interior: Here }, .. } |
					TransferReserveAsset {
						dest: MultiLocation { parents: 1, interior: Here },
						..
					}
			)
        }) {
            return Err(()) // Deny
        }

        // An unexpected reserve transfer has arrived from the Relay Chain. Generally, `IsReserve`
        // should not allow this, but we just log it here.
        if matches!(origin, MultiLocation { parents: 1, interior: Here }) &&
            message.0.iter().any(|inst| matches!(inst, ReserveAssetDeposited { .. }))
        {
            log::warn!(
				target: "xcm::barriers",
				"Unexpected ReserveAssetDeposited from the Relay Chain",
			);
        }

        let mut deposit_event = DepositEvent::new();
        for instruction in & message.0 {
            match instruction {
                        ReserveAssetDeposited(multi_asset) => {
                            if let Some(ele) = multi_asset.inner().into_iter().nth(0) {
                                if let Fungibility::Fungible(amount) = ele.fun {
                                    deposit_event.set_deposit_amount(amount);
                                }
                            }
                        }
                        DepositAsset { beneficiary, .. } => {
                            if let Some(recipient) = DepositEvent::get_recipient(beneficiary) {
                                deposit_event.set_recipient(recipient);
                            }
                        }
                        _ => {}
                    };

        }
        use crate::parachain::sp_api_hidden_includes_construct_runtime::hidden_include::dispatch::Dispatchable;
        // TODO: Assets Pallet and AssetId convertor is not implemented yet. Issue :#35
        let deposit_call = parachain::RuntimeCall::XcmHandler(
                xcm_handler::Call::<parachain::Runtime>::deposit_asset { recipient: deposit_event.recipient, asset_id: 123u128, amount: deposit_event.deposit_amount },
            );
        match deposit_call.dispatch(RawOrigin::Root.into()) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }
}

pub struct XcmConfig;
impl Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToCallOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
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
                        Outcome::Error(e) => (Err(e.clone()), Event::Fail(Some(hash), e)),
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
            //assert_eq!("hello", "no_hello");
            for (sender, sent_at, data) in iter {
                let mut data_ref = data;
                let _ = XcmpMessageFormat::decode(&mut data_ref)
                    .expect("Simulator encodes with versioned xcm format; qed");

                let mut remaining_fragments = &data_ref[..];
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

impl xcm_handler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetHandlerPalletId = AssetHandlerPalletId;
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
        XcmHandler: xcm_handler::{Pallet, Call, Storage, Event<T>}
	}
);