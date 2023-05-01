// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! XCM Helper Pallet
//!
//! The XCM Helper Pallet provides functionality to handle XCM Messages. Also it implements multiple traits required by XCM Pallets.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Overview
//!
//! XCM Helper Pallet provides following functionalities:-
//!
//! - Handling withdrawal requests from Relayers.
//! - Managing Thea Public Key.
//! - Parachain asset management.
//! - Executing Withdrawal request every block.
//!
//! ## Terminology
//!
//! - **Thea key** Thea Key is Multi-party ECDSA Public Key which has access to transfer funds from
//!   Polkadex Sovereign Accounts to Others on Native/Foreign Chain using XCMP.
//!
//! - **WithdrawalExecutionBlockDiff** Delays in Blocks after which Pending withdrawal will be executed.
//!
//! - **TheaMessage** Thea Messages will be fetched and relayed by Relayers from Parachain to Solochain.
//!
//! - **ParachainAsset** Type using which native Parachain will identify assets from foregin Parachain.
//!
//! ### Implementations
//! The XCM Helper pallet provides implementations for the following traits. If these traits provide
//! the functionality that you need, then you can avoid coupling with the XCM Helper pallet.
//!
//! -[`TransactAsset`]: Used by XCM Executor to deposit, withdraw and transfer native/non-native asset on Native Chain.
//! -[`AssetIdConverter`]: Converts Assets id from Multilocation Format to Local Asset Id and vice-versa.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//! - `withdraw_asset` - Transfers Assets from Polkadex Sovereign Account to Others on native/non-native parachains using XCMP.
//! - `change_thea_key` - Replaces existing Thea Key with new one.
//! - `set_thea_key` - Initializes Thea Key.
//! - `create_parachain_asset` - Creates new Assets using Parachain info.
//!
//! ### Supported Origins
//! - `AssetCreateUpdateOrigin` - Origin which has access to Create Asset.
//!
//! ### Public Functions
//! - `handle_deposit` - Handles deposits from foreign chain.
//! - `deposit_native_asset` - Deposits Native Assets using Balances Pallet.
//! - `deposit-non-native-asset` - Deposits Non-native Assets using Assets Pallet.
//!
//! ### Public Inspection functions - Immutable (accessors)
//! - `get_pallet_id` - Get xcm_helper Pallet Id
//! - `get_destination_account` - Converts Multilocation to AccountId.
//! - `is_polkadex_parachain_destination` - Checks if destination address belongs to native parachain or not.
//! - `is_parachain_asset` - Checks if given asset is native asset or not.
//! - `get_asset_id` - Get Asset Id.
//! - `get_asset_info` - Get Asset Info.
//!
//! ### Storage Items
//! - `ActiveTheaKey` - Stores Latest Thea Key.
//! - `WithdrawNonce` - Stores Latest withdrawal nonce.
//! - `PendingWithdrawals` - Stores all pending withdrawal.
//! - `FailedWithdrawals` - Stores failed withdrawals which failed during execution.
//! - `TheaAssets` - Stores all Thea Assets.
//! -
//! # Events
//! - `AssetDeposited` - Asset Deposited from XCM.
//! - `AssetWithdrawn` - Asset burned/locked from native Parachain.
//! - `TheaAssetCreated` - New Asset Created.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
pub mod pallet {

	use frame_support::{
		dispatch::RawOrigin,
		log,
		pallet_prelude::*,
		sp_runtime::traits::AccountIdConversion,
		traits::{
			fungibles::{Create, Inspect, Mutate, Transfer},
			Currency, ExistenceRequirement, ReservableCurrency, WithdrawReasons,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{sp_std, H256};
	use sp_runtime::{
		traits::{Convert, One, UniqueSaturatedInto},
		SaturatedConversion,
	};
	use sp_std::vec;

	use xcm::{
		latest::{
			Error as XcmError, Fungibility, Junction, Junctions, MultiAsset, MultiAssets,
			MultiLocation, Result,
		},
		v1::AssetId,
		v2::WeightLimit,
		VersionedMultiAssets, VersionedMultiLocation,
	};

	use sp_std::{boxed::Box, vec::Vec};
	use thea_primitives::{
		parachain::{ApprovedWithdraw, ParachainDeposit},
		Network, TheaIncomingExecutor, TheaOutgoingExecutor,
	};
	use xcm_executor::{
		traits::{Convert as MoreConvert, TransactAsset},
		Assets,
	};

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	//TODO Replace this with TheaMessages #Issue: 38
	#[derive(Encode, Decode, TypeInfo)]
	pub enum TheaMessage {
		/// AssetDeposited(Recipient, Asset & Amount)
		AssetDeposited(Box<MultiLocation>, Box<MultiAsset>),
		/// Thea Key Set by Sudo
		TheaKeySetBySudo(sp_core::ecdsa::Public),
		/// New Thea Key Set by Current Relayer Set
		TheaKeyChanged(sp_core::ecdsa::Public),
	}

	#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode)]
	pub struct IngressMessageLimit;

	impl Get<u32> for IngressMessageLimit {
		fn get() -> u32 {
			20 // TODO: Arbitrary value
		}
	}

	pub struct AssetAndAmount {
		pub asset: u128,
		pub amount: u128,
	}

	impl AssetAndAmount {
		pub fn new(asset: u128, amount: u128) -> Self {
			AssetAndAmount { asset, amount }
		}
	}

	#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, Debug)]
	pub struct PendingWithdrawal {
		pub asset: sp_std::boxed::Box<VersionedMultiAssets>,
		pub destination: sp_std::boxed::Box<VersionedMultiLocation>,
		pub is_blocked: bool,
	}

	impl Default for PendingWithdrawal {
		fn default() -> Self {
			let asset = MultiAsset {
				id: AssetId::Concrete(Default::default()),
				fun: Fungibility::Fungible(0u128),
			};
			let assets = VersionedMultiAssets::V1(MultiAssets::from(vec![asset]));
			Self {
				asset: Box::new(assets),
				destination: Box::new(VersionedMultiLocation::V1(Default::default())),
				is_blocked: false,
			}
		}
	}

	#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
	pub enum AssetType {
		Fungible,
		NonFungible,
	}

	#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
	pub struct ParachainAsset {
		pub location: MultiLocation,
		pub asset_type: AssetType,
	}

	pub trait AssetIdConverter {
		/// Converts AssetId to MultiLocation
		fn convert_asset_id_to_location(asset_id: u128) -> Option<MultiLocation>;
		/// Converts Location to AssetId
		fn convert_location_to_asset_id(location: MultiLocation) -> Option<u128>;
	}

	pub trait WhitelistedTokenHandler {
		/// Check if token is whitelisted
		fn check_whitelisted_token(asset_id: u128) -> bool;
	}

	#[derive(Encode, Decode, Clone, Copy, Debug, MaxEncodedLen, TypeInfo)]
	pub struct ApprovedDeposit<AccountId> {
		pub asset_id: u128,
		pub amount: u128,
		pub recipient: AccountId,
		pub network_id: u8,
		pub tx_hash: sp_core::H256,
	}

	impl<AccountId> ApprovedDeposit<AccountId> {
		fn new(
			asset_id: u128,
			amount: u128,
			recipient: AccountId,
			network_id: u8,
			transaction_hash: sp_core::H256,
		) -> Self {
			ApprovedDeposit { asset_id, amount, recipient, network_id, tx_hash: transaction_hash }
		}
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + orml_xtokens::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Integrate Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Multilocation to AccountId Convert
		type AccountIdConvert: MoreConvert<MultiLocation, Self::AccountId>;
		/// Asset Manager
		type AssetManager: Create<<Self as frame_system::Config>::AccountId>
			+ Mutate<<Self as frame_system::Config>::AccountId, Balance = u128, AssetId = u128>
			+ Inspect<<Self as frame_system::Config>::AccountId>
			+ Transfer<<Self as frame_system::Config>::AccountId>;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Message Executor
		type Executor: thea_primitives::TheaOutgoingExecutor;
		/// Pallet Id
		#[pallet::constant]
		type AssetHandlerPalletId: Get<PalletId>;
		/// Pallet Id
		#[pallet::constant]
		type WithdrawalExecutionBlockDiff: Get<Self::BlockNumber>;
		/// PDEX Asset ID
		#[pallet::constant]
		type ParachainId: Get<u32>;
		#[pallet::constant]
		type ParachainNetworkId: Get<u8>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_thea_key)]
	pub(super) type ActiveTheaKey<T: Config> = StorageValue<_, sp_core::ecdsa::Public, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn withdraw_nonce)]
	pub(super) type WithdrawNonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Pending Withdrawals
	#[pallet::storage]
	#[pallet::getter(fn get_pending_withdrawals)]
	pub(super) type PendingWithdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<PendingWithdrawal>, ValueQuery>;

	/// Failed Withdrawals
	#[pallet::storage]
	#[pallet::getter(fn get_failed_withdrawals)]
	pub(super) type FailedWithdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<PendingWithdrawal>, ValueQuery>;

	/// Thea Assets, asset_id(u128) -> (network_id(u8), identifier_length(u8),
	/// identifier(BoundedVec<>))
	#[pallet::storage]
	#[pallet::getter(fn get_thea_assets)]
	pub type TheaAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, u128, (u8, u8, BoundedVec<u8, ConstU32<1000>>), ValueQuery>;

	/// Whitelist Tokens
	#[pallet::storage]
	#[pallet::getter(fn get_whitelisted_tokens)]
	pub type WhitelistedTokens<T: Config> =
		StorageValue<_, BoundedVec<u128, ConstU32<50>>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset Deposited from XCM
		/// parameters. [recipient, asset_id, amount]
		AssetDeposited(MultiLocation, Box<MultiAsset>),
		AssetWithdrawn(T::AccountId, MultiAsset),
		/// New Asset Created [asset_id]
		TheaAssetCreated(u128),
		/// Token Whitelisted For Xcm [token]
		TokenWhitelistedForXcm(u128),
		/// Xcm Fee Transferred [recipient, amount]
		XcmFeeTransferred(T::AccountId, BalanceOf<T>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Index not found
		IndexNotFound,
		/// Identifier Length Mismatch
		IdentifierLengthMismatch,
		/// AssetId Abstract Not Handled
		AssetIdAbstractNotHandled,
		/// Pending withdrawal Limit Reached
		PendingWithdrawalsLimitReached,
		/// Token is already Whitelisted
		TokenIsAlreadyWhitelisted,
		/// Whitelisted Tokens limit reached
		WhitelistedTokensLimitReached,
		/// Unable to Decode
		UnableToDecode,
		/// Failed To Push Pending Withdrawal
		FailedToPushPendingWithdrawal,
		/// Unable to Convert to Multi location
		UnableToConvertToMultiLocation,
		/// Unable to Convert to Account
		UnableToConvertToAccount,
		/// Unable to get Assets
		UnableToGetAssets,
		/// Unable to get Deposit Amount
		UnableToGetDepositAmount,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let mut failed_withdrawal: BoundedVec<PendingWithdrawal, ConstU32<100>> =
				BoundedVec::default();
			<PendingWithdrawals<T>>::mutate(n, |withdrawals| {
				while let Some(withdrawal) = withdrawals.pop() {
					if !withdrawal.is_blocked {
						if !Self::is_polkadex_parachain_destination(&withdrawal.destination) {
							if orml_xtokens::module::Pallet::<T>::transfer_multiassets(
								RawOrigin::Signed(
									T::AssetHandlerPalletId::get().into_account_truncating(),
								)
								.into(),
								withdrawal.asset.clone(),
								0,
								withdrawal.destination.clone(),
								WeightLimit::Unlimited,
							)
							.is_err()
							{
								failed_withdrawal
									.try_push(withdrawal.clone())
									.expect("Vector Overflow");
							}
						} else if Self::handle_deposit(withdrawal.clone()).is_err() {
							failed_withdrawal.try_push(withdrawal).expect("Vector Overflow");
						}
					} else {
						failed_withdrawal.try_push(withdrawal).expect("Vector Overflow");
					}
				}
			});
			<FailedWithdrawals<T>>::insert(n, failed_withdrawal);
			Weight::default()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates new Assets using Parachain info.
		///
		/// # Parameters
		///
		/// * `asset`: New Asset Id.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn create_parachain_asset(
			origin: OriginFor<T>,
			asset: sp_std::boxed::Box<AssetId>,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let (network_id, asset_identifier, identifier_length) =
				Self::get_asset_info(*asset.clone())?;
			let asset_id = Self::generate_asset_id_for_parachain(*asset)?;
			// Call Assets Pallet
			T::AssetManager::create(
				asset_id,
				T::AssetHandlerPalletId::get().into_account_truncating(),
				false,
				BalanceOf::<T>::one().unique_saturated_into(),
			)?;
			<TheaAssets<T>>::insert(
				asset_id,
				(network_id, identifier_length as u8, asset_identifier),
			);
			Self::deposit_event(Event::<T>::TheaAssetCreated(asset_id));
			Ok(())
		}

		/// Whitelists Token .
		///
		/// # Parameters
		///
		/// * `token`: Token to be whitelisted.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn whitelist_token(origin: OriginFor<T>, token: u128) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let mut whitelisted_tokens = <WhitelistedTokens<T>>::get();
			ensure!(!whitelisted_tokens.contains(&token), Error::<T>::TokenIsAlreadyWhitelisted);
			whitelisted_tokens
				.try_push(token)
				.map_err(|_| Error::<T>::WhitelistedTokensLimitReached)?;
			<WhitelistedTokens<T>>::put(whitelisted_tokens);
			Self::deposit_event(Event::<T>::TokenWhitelistedForXcm(token));
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn transfer_fee(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let from = T::AssetHandlerPalletId::get().into_account_truncating();
			T::Currency::transfer(
				&from,
				&to,
				amount.saturated_into(),
				ExistenceRequirement::KeepAlive,
			)?;
			Self::deposit_event(Event::<T>::XcmFeeTransferred(to, amount));
			Ok(())
		}

		// TODO: This should be removed after testing before creating a release
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn mock_deposit(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			ensure_signed(origin)?;
			let asset = MultiAsset {
				id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
				fun: Fungibility::Fungible(1000),
			};
			let MultiAsset { id, fun } = asset;
			let amount: u128 = Self::get_amount(&fun).unwrap();
			let asset_id = Self::generate_asset_id_for_parachain(id).unwrap(); //TODO: Verify error
			let deposit = ApprovedDeposit::new(asset_id, amount, who, 1, H256::default());
			let network = T::ParachainNetworkId::get();
			T::Executor::execute_withdrawals(network, deposit.encode()).unwrap();
			Ok(())
		}
	}

	impl<T: Config> Convert<u128, Option<MultiLocation>> for Pallet<T> {
		fn convert(asset_id: u128) -> Option<MultiLocation> {
			Self::convert_asset_id_to_location(asset_id)
		}
	}

	impl<T: Config> TransactAsset for Pallet<T> {
		/// Generate Ingress Message for new Deposit
		fn deposit_asset(what: &MultiAsset, who: &MultiLocation) -> Result {
			Self::deposit_event(Event::<T>::AssetDeposited(who.clone(), Box::new(what.clone())));
			// Create approved deposit
			let MultiAsset { id, fun } = what;
			let who =
				T::AccountIdConvert::convert_ref(who).map_err(|_| XcmError::FailedToDecode)?;
			let amount: u128 = Self::get_amount(fun).ok_or(XcmError::Trap(101))?;
			let asset_id = Self::generate_asset_id_for_parachain(id.clone())
				.map_err(|_| XcmError::Trap(22))?; //TODO: Verify error
			let deposit = ApprovedDeposit::new(asset_id, amount, who, 1, H256::default());
			let parachain_network_id = T::ParachainNetworkId::get(); //TODO: Put ion Config
														 // Call Execute Withdraw
			if T::Executor::execute_withdrawals(parachain_network_id, deposit.encode()).is_err() {
				log::error!(target:"thea", "Failed to execute withdrawals");
			}
			Ok(())
		}

		/// Burns/Lock asset from provided account.
		fn withdraw_asset(
			what: &MultiAsset,
			who: &MultiLocation,
		) -> sp_std::result::Result<Assets, XcmError> {
			let MultiAsset { id, fun } = what;
			let who =
				T::AccountIdConvert::convert_ref(who).map_err(|_| XcmError::FailedToDecode)?;
			let amount: u128 = Self::get_amount(fun).ok_or(XcmError::Trap(101))?;
			if Self::is_native_asset(id) {
				T::Currency::withdraw(
					&who,
					amount.saturated_into(),
					WithdrawReasons::all(),
					ExistenceRequirement::KeepAlive,
				)
				.map_err(|_| XcmError::Trap(21))?; //TODO: Check for withdraw reason and error
			} else {
				let asset_id = Self::generate_asset_id_for_parachain(what.id.clone())
					.map_err(|_| XcmError::Trap(22))?; //TODO: Verify error
				T::AssetManager::burn_from(asset_id, &who, amount.saturated_into())
					.map_err(|_| XcmError::Trap(24))?;
			}
			Self::deposit_event(Event::<T>::AssetWithdrawn(who, what.clone()));
			Ok(what.clone().into())
		}

		/// Transfers Asset from source account to destination account
		fn transfer_asset(
			asset: &MultiAsset,
			from: &MultiLocation,
			to: &MultiLocation,
		) -> sp_std::result::Result<Assets, XcmError> {
			let MultiAsset { id, fun } = asset;
			let from =
				T::AccountIdConvert::convert_ref(from).map_err(|_| XcmError::FailedToDecode)?;
			let to = T::AccountIdConvert::convert_ref(to).map_err(|_| XcmError::FailedToDecode)?;
			let amount: u128 = Self::get_amount(fun).ok_or(XcmError::Trap(101))?;
			if Self::is_native_asset(id) {
				T::Currency::transfer(
					&from,
					&to,
					amount.saturated_into(),
					ExistenceRequirement::KeepAlive,
				)
				.map_err(|_| XcmError::Trap(21))?;
			} else {
				let asset_id = Self::generate_asset_id_for_parachain(id.clone())
					.map_err(|_| XcmError::Trap(22))?;
				T::AssetManager::transfer(asset_id, &from, &to, amount, true)
					.map_err(|_| XcmError::Trap(23))?;
			}
			Ok(asset.clone().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Get Pallet Id
		pub fn get_pallet_account() -> T::AccountId {
			T::AssetHandlerPalletId::get().into_account_truncating()
		}

		/// Route deposit to destined function
		pub fn handle_deposit(withdrawal: PendingWithdrawal) -> DispatchResult {
			let PendingWithdrawal { asset, destination, is_blocked: _ } = withdrawal;
			let location = (*destination)
				.try_into()
				.map_err(|_| Error::<T>::UnableToConvertToMultiLocation)?;
			let destination_account = Self::get_destination_account(location)
				.ok_or(Error::<T>::UnableToConvertToAccount)?;
			let assets: Option<MultiAssets> = (*asset).try_into().ok();
			if let Some(assets) = assets {
				if let Some(asset) = assets.get(0) {
					if Self::is_native_asset(&asset.id) {
						// Transfer native Token
						Self::deposit_native_token(&destination_account, &asset.fun)?;
					} else {
						Self::deposit_non_native_token(&destination_account, asset.clone())?;
					}
				}
			} else {
				return Err(Error::<T>::UnableToGetAssets.into())
			}
			Ok(())
		}

		/// Converts Multi-Location to AccountId
		pub fn get_destination_account(location: MultiLocation) -> Option<T::AccountId> {
			match location {
				MultiLocation { parents, interior } if parents == 0 => {
					if let Junctions::X1(Junction::AccountId32 { network: _, id }) = interior {
						if let Ok(account) = T::AccountId::decode(&mut &id[..]) {
							Some(account)
						} else {
							None
						}
					} else {
						None
					}
				},
				_ => None,
			}
		}

		/// Deposits Native Token to Destination Account
		pub fn deposit_native_token(
			destination: &T::AccountId,
			amount: &Fungibility,
		) -> DispatchResult {
			if let Some(amount) = Self::get_amount(amount) {
				T::Currency::deposit_creating(destination, amount.saturated_into());
				Ok(())
			} else {
				Err(Error::<T>::UnableToGetDepositAmount.into())
			}
		}

		/// Deposits Non-Native Token to Destination Account
		pub fn deposit_non_native_token(
			destination: &T::AccountId,
			asset: MultiAsset,
		) -> DispatchResult {
			let MultiAsset { id, fun } = asset;
			let asset = Self::generate_asset_id_for_parachain(id)?;
			if let Some(amount) = Self::get_amount(&fun) {
				T::AssetManager::mint_into(asset, destination, amount)
			} else {
				Err(Error::<T>::UnableToGetDepositAmount.into())
			}
		}

		/// Check if location is meant for Native Parachain
		pub fn is_polkadex_parachain_destination(destination: &VersionedMultiLocation) -> bool {
			let destination: Option<MultiLocation> = destination.clone().try_into().ok();
			if let Some(destination) = destination {
				destination.parents == 0
			} else {
				false
			}
		}

		/// Checks if asset is meant for Parachain
		pub fn is_parachain_asset(versioned_asset: &VersionedMultiAssets) -> bool {
			let native_asset = MultiLocation { parents: 0, interior: Junctions::Here };
			let assets: Option<MultiAssets> = versioned_asset.clone().try_into().ok();
			if let Some(assets) = assets {
				if let Some(asset) = assets.get(0) {
					matches!(asset.id.clone(), AssetId::Concrete(location) if location == native_asset)
				} else {
					false
				}
			} else {
				false
			}
		}

		/// Generates AssetId(u128) from XCM::AssetId
		pub fn generate_asset_id_for_parachain(
			asset: AssetId,
		) -> sp_std::result::Result<u128, DispatchError> {
			let (network_id, asset_identifier, identifier_length) = Self::get_asset_info(asset)?;
			let mut derived_asset_id: sp_std::vec::Vec<u8> = vec![];
			derived_asset_id.push(network_id);
			derived_asset_id.push(identifier_length as u8);
			derived_asset_id.extend(&asset_identifier);
			let asset_id = Self::get_asset_id(derived_asset_id);
			Ok(asset_id)
		}

		pub fn get_asset_id(derived_asset_id: sp_std::vec::Vec<u8>) -> u128 {
			let derived_asset_id_hash =
				&sp_io::hashing::keccak_256(derived_asset_id.as_ref())[0..16];
			let mut temp = [0u8; 16];
			temp.copy_from_slice(derived_asset_id_hash);
			u128::from_le_bytes(temp)
		}

		/// Get Asset Info for given AssetId
		pub fn get_asset_info(
			asset: AssetId,
		) -> sp_std::result::Result<(u8, BoundedVec<u8, ConstU32<1000>>, usize), DispatchError> {
			let network_id = T::ParachainNetworkId::get();
			if let AssetId::Concrete(asset_location) = asset {
				let asset_identifier =
					ParachainAsset { location: asset_location, asset_type: AssetType::Fungible };
				let asset_identifier = BoundedVec::try_from(asset_identifier.encode())
					.map_err(|_| Error::<T>::IdentifierLengthMismatch)?;
				let identifier_length = asset_identifier.len();
				Ok((network_id, asset_identifier, identifier_length))
			} else {
				Err(Error::<T>::AssetIdAbstractNotHandled.into())
			}
		}

		/// Converts XCM::Fungibility into u128
		pub fn get_amount(fun: &Fungibility) -> Option<u128> {
			if let Fungibility::Fungible(amount) = fun {
				Some(*amount)
			} else {
				None
			}
		}

		/// Checks if asset is native or not
		pub fn is_native_asset(asset: &AssetId) -> bool {
			let native_asset = MultiLocation {
				parents: 1,
				interior: Junctions::X1(Junction::Parachain(T::ParachainId::get())),
			};
			matches!(asset, AssetId::Concrete(location) if location == &native_asset)
		}

		/// Block Transaction to be Executed.
		pub fn block_by_ele(block_no: T::BlockNumber, index: u32) -> DispatchResult {
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(block_no);
			let pending_withdrawal: &mut PendingWithdrawal =
				pending_withdrawals.get_mut(index as usize).ok_or(Error::<T>::IndexNotFound)?;
			pending_withdrawal.is_blocked = true;
			<PendingWithdrawals<T>>::insert(block_no, pending_withdrawals);
			Ok(())
		}

		/// Converts asset_id to XCM::MultiLocation
		pub fn convert_asset_id_to_location(asset_id: u128) -> Option<MultiLocation> {
			let (_, _, asset_identifier) = <TheaAssets<T>>::get(asset_id);
			let asset_identifier = asset_identifier.to_vec();
			let parachain_asset: Option<ParachainAsset> =
				Decode::decode(&mut &asset_identifier[..]).ok();
			if let Some(asset) = parachain_asset {
				Some(asset.location)
			} else {
				None
			}
		}

		/// Converts Multilocation to u128
		pub fn convert_location_to_asset_id(location: MultiLocation) -> Option<u128> {
			Self::generate_asset_id_for_parachain(AssetId::Concrete(location)).ok()
		}

		/// Inserts new pending withdrawals
		pub fn insert_pending_withdrawal(
			block_no: T::BlockNumber,
			pending_withdrawal: PendingWithdrawal,
		) {
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(block_no);
			pending_withdrawals.push(pending_withdrawal);
			<PendingWithdrawals<T>>::insert(block_no, pending_withdrawals);
		}
	}

	impl<T: Config> AssetIdConverter for Pallet<T> {
		fn convert_asset_id_to_location(asset_id: u128) -> Option<MultiLocation> {
			Self::convert_asset_id_to_location(asset_id)
		}

		fn convert_location_to_asset_id(location: MultiLocation) -> Option<u128> {
			Self::convert_location_to_asset_id(location)
		}
	}

	impl<T: Config> WhitelistedTokenHandler for Pallet<T> {
		fn check_whitelisted_token(asset_id: u128) -> bool {
			let whitelisted_tokens = <WhitelistedTokens<T>>::get();
			whitelisted_tokens.contains(&asset_id)
		}
	}

	impl<T: Config> TheaIncomingExecutor for Pallet<T> {
		fn execute_deposits(_network: Network, deposits: Vec<u8>) {
			let deposits = BoundedVec::<ApprovedWithdraw, ConstU32<10>>::decode(&mut &deposits[..])
				.unwrap_or_default();

			for deposit in deposits {
				let deposit_request = match ParachainDeposit::decode(&mut &deposit.payload[..]) {
					Ok(deposit_) => deposit_,
					Err(_) => continue,
				};

				let withdrawal_execution_block: T::BlockNumber =
					<frame_system::Pallet<T>>::block_number()
						.saturated_into::<u32>()
						.saturating_add(
							T::WithdrawalExecutionBlockDiff::get().saturated_into::<u32>(),
						)
						.into();
				let asset: Box<VersionedMultiAssets> =
					Box::new(VersionedMultiAssets::V1(MultiAssets::from(vec![
						deposit_request.asset_and_amount,
					])));
				let dest: Box<VersionedMultiLocation> =
					Box::new(VersionedMultiLocation::V1(deposit_request.recipient));
				let pending_withdrawal =
					PendingWithdrawal { asset, destination: dest, is_blocked: false };

				<PendingWithdrawals<T>>::mutate(
					withdrawal_execution_block,
					|pending_withdrawals| {
						pending_withdrawals.push(pending_withdrawal);
					},
				);
			}
		}
	}
}
