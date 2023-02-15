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

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		dispatch::{DispatchResultWithPostInfo, RawOrigin},
		pallet_prelude::*,
		sp_runtime::traits::AccountIdConversion,
		traits::{
			fungibles::{Create, Inspect, Mutate, Transfer},
			Currency, ExistenceRequirement, ReservableCurrency, WithdrawReasons,
		},
		PalletId,
	};
	use frame_system::{pallet_prelude::*, Origin};
	use sp_core::sp_std;
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
		AssetDeposited(MultiLocation, MultiAsset),
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

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + orml_xtokens::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Integrate Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Multilocation to AccountId Convetor
		type AccountIdConvert: MoreConvert<MultiLocation, Self::AccountId>;
		/// Asset Manager
		type AssetManager: Create<<Self as frame_system::Config>::AccountId>
			+ Mutate<<Self as frame_system::Config>::AccountId, Balance = u128, AssetId = u128>
			+ Inspect<<Self as frame_system::Config>::AccountId>
			+ Transfer<<Self as frame_system::Config>::AccountId>;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;
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

	// Queue for enclave ingress messages
	#[pallet::storage]
	#[pallet::getter(fn ingress_messages)]
	pub(super) type IngressMessages<T: Config> =
		StorageValue<_, BoundedVec<TheaMessage, IngressMessageLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_thea_key)]
	pub(super) type ActiveTheaKey<T: Config> = StorageValue<_, [u8; 64], OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn withdraw_nonce)]
	pub(super) type WithdrawNonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Pending Withdrawals
	#[pallet::storage]
	#[pallet::getter(fn get_pending_withdrawls)]
	pub(super) type PendingWithdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<PendingWithdrawal, ConstU32<100>>,
		ValueQuery,
	>;

	/// Failed Withdrawals
	#[pallet::storage]
	#[pallet::getter(fn get_failed_withdrawls)]
	pub(super) type FailedWithdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<PendingWithdrawal, ConstU32<100>>,
		ValueQuery,
	>;

	/// Thea Assets, asset_id(u128) -> (network_id(u8), identifier_length(u8),
	/// identifier(BoundedVec<>))
	#[pallet::storage]
	#[pallet::getter(fn get_thea_assets)]
	pub type TheaAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, u128, (u8, u8, BoundedVec<u8, ConstU32<1000>>), ValueQuery>;

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
		AssetDeposited(MultiLocation, MultiAsset),
		AssetWithdrawn(T::AccountId, MultiAsset),
		/// New Asset Created [asset_id]
		TheaAssetCreated(u128),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Invalid Sender
		InvalidSender,
		/// Ingress Messages Limit Reached
		IngressMessagesLimitReached,
		/// Signature verification failed
		SignatureVerificationFailed,
		/// Public Key not set
		PublicKeyNotSet,
		/// Nonce is not valid
		NonceIsNotValid,
		/// Index not found
		IndexNotFound,
		/// Identifier Length Mismatch
		IdentifierLengthMismatch,
		/// AssetId Abstract Not Handled
		AssetIdAbstractNotHandled,
		/// Internal Error
		InternalError,
		/// Pending withdrawal Limit Reached
		PendingWithdrawalsLimitReached,
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
						} else {
							if let Err(_) = Self::handle_deposit(withdrawal.clone()) {
								failed_withdrawal.try_push(withdrawal).expect("Vector Overflow");
							}
						}
					} else {
						failed_withdrawal.try_push(withdrawal).expect("Vector Overflow");
					}
				}
			});
			<FailedWithdrawals<T>>::insert(n, failed_withdrawal);
			<IngressMessages<T>>::kill();
			Weight::default()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn withdraw_asset(
			origin: OriginFor<T>,
			payload: BoundedVec<
				(
					sp_std::boxed::Box<VersionedMultiAssets>,
					sp_std::boxed::Box<VersionedMultiLocation>,
				),
				ConstU32<10>,
			>,
			withdraw_nonce: u32,
			signature: sp_core::ecdsa::Signature,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin.clone())?;
			let current_withdraw_nonce = <WithdrawNonce<T>>::get();
			ensure!(withdraw_nonce == current_withdraw_nonce, Error::<T>::NonceIsNotValid);
			<WithdrawNonce<T>>::put(current_withdraw_nonce.saturating_add(1));
			let pubic_key = <ActiveTheaKey<T>>::get().ok_or(Error::<T>::PublicKeyNotSet)?;
			let encoded_payload = Encode::encode(&payload);
			let payload_hash = sp_io::hashing::keccak_256(&encoded_payload);
			if Self::verify_ecdsa_prehashed(&signature, &pubic_key, &payload_hash)? {
				let withdrawal_execution_block: T::BlockNumber =
					<frame_system::Pallet<T>>::block_number()
						.saturated_into::<u32>()
						.saturating_add(
							T::WithdrawalExecutionBlockDiff::get().saturated_into::<u32>(),
						)
						.into();
				for (asset, dest) in payload {
					let pending_withdrawal =
						PendingWithdrawal { asset, destination: dest, is_blocked: false };
					<PendingWithdrawals<T>>::try_mutate(
						withdrawal_execution_block,
						|pending_withdrawals| {
							pending_withdrawals
								.try_push(pending_withdrawal)
								.map_err(|_| Error::<T>::PendingWithdrawalsLimitReached)
						},
					)?;
				}
			} else {
				return Err(Error::<T>::SignatureVerificationFailed.into())
			}
			Ok(().into())
		}

		///Update Thea Key
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn change_thea_key(
			origin: OriginFor<T>,
			_new_thea_key: [u8; 64],
			_signature: sp_core::ecdsa::Signature,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin.clone())?;
			Ok(().into())
		}

		///Set Thea Key
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn set_thea_key(
			origin: OriginFor<T>,
			thea_key: [u8; 64],
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<ActiveTheaKey<T>>::put(thea_key);
			Ok(().into())
		}

		///Create Asset
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
	}

	impl<T: Config> Convert<u128, Option<MultiLocation>> for Pallet<T> {
		fn convert(asset_id: u128) -> Option<MultiLocation> {
			Self::convert_asset_id_to_location(asset_id)
		}
	}

	impl<T: Config> Convert<MultiLocation, Option<u128>> for Pallet<T> {
		fn convert(a: MultiLocation) -> Option<u128> {
			todo!()
		}
	}

	impl<T: Config> TransactAsset for Pallet<T> {
		fn deposit_asset(what: &MultiAsset, who: &MultiLocation) -> Result {
			<IngressMessages<T>>::try_mutate(|ingress_messages| {
				ingress_messages.try_push(TheaMessage::AssetDeposited(who.clone(), what.clone()))
			})
			.map_err(|_| XcmError::Trap(10))?;
			Self::deposit_event(Event::<T>::AssetDeposited(who.clone(), what.clone()));
			Ok(())
		}

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
			Self::deposit_event(Event::<T>::AssetWithdrawn(who.clone(), what.clone()));
			Ok(what.clone().into())
		}

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
		pub fn get_pallet_account() -> T::AccountId {
			T::AssetHandlerPalletId::get().into_account_truncating()
		}

		pub fn handle_deposit(withdrawal: PendingWithdrawal) -> DispatchResult {
			let PendingWithdrawal { asset, destination, is_blocked: _ } = withdrawal;
			let location =
				(*destination.clone()).try_into().map_err(|_| Error::<T>::InternalError)?;
			let destination_account =
				Self::get_destination_account(location).ok_or(Error::<T>::InternalError)?;
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
				return Err(Error::<T>::InternalError.into())
			}
			Ok(())
		}

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

		pub fn deposit_native_token(
			destination: &T::AccountId,
			amount: &Fungibility,
		) -> DispatchResult {
			if let Some(amount) = Self::get_amount(amount) {
				T::Currency::deposit_creating(destination, amount.saturated_into());
				Ok(())
			} else {
				Err(Error::<T>::InternalError.into())
			}
		}

		pub fn deposit_non_native_token(
			destination: &T::AccountId,
			asset: MultiAsset,
		) -> DispatchResult {
			let MultiAsset { id, fun } = asset;
			let asset = Self::generate_asset_id_for_parachain(id)?;
			if let Some(amount) = Self::get_amount(&fun) {
				T::AssetManager::mint_into(asset, destination, amount)
			} else {
				Err(Error::<T>::InternalError.into())
			}
		}

		pub fn is_polkadex_parachain_destination(destination: &VersionedMultiLocation) -> bool {
			let destination: Option<MultiLocation> = destination.clone().try_into().ok();
			if let Some(destination) = destination {
				destination.parents == 0
			} else {
				false
			}
		}

		pub fn is_parachain_asset(versioned_asset: &VersionedMultiAssets) -> bool {
			let native_asset = MultiLocation { parents: 0, interior: Junctions::Here };
			let assets: Option<MultiAssets> = versioned_asset.clone().try_into().ok();
			if let Some(assets) = assets {
				if let Some(asset) = assets.get(0) {
					match asset.id.clone() {
						AssetId::Concrete(location) if location == native_asset => true,
						_ => false,
					}
				} else {
					false
				}
			} else {
				false
			}
		}

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

		pub fn get_amount(fun: &Fungibility) -> Option<u128> {
			if let Fungibility::Fungible(amount) = fun {
				return Some(*amount)
			} else {
				None
			}
		}

		pub fn is_native_asset(asset: &AssetId) -> bool {
			let native_asset = MultiLocation {
				parents: 1,
				interior: Junctions::X1(Junction::Parachain(T::ParachainId::get().into())),
			};
			match asset {
				AssetId::Concrete(location) if location == &native_asset => true,
				_ => false,
			}
		}

		pub fn verify_ecdsa_prehashed(
			signature: &sp_core::ecdsa::Signature,
			public_key: &[u8; 64],
			payload_hash: &[u8; 32],
		) -> sp_std::result::Result<bool, DispatchError> {
			let recovered_pb = sp_io::crypto::secp256k1_ecdsa_recover(&signature.0, payload_hash)
				.map_err(|_| Error::<T>::SignatureVerificationFailed)?;
			ensure!(recovered_pb == *public_key, Error::<T>::SignatureVerificationFailed);
			Ok(true)
		}

		pub fn block_by_ele(block_no: T::BlockNumber, index: u32) -> DispatchResult {
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(block_no);
			let pending_withdrwal: &mut PendingWithdrawal =
				pending_withdrawals.get_mut(index as usize).ok_or(Error::<T>::IndexNotFound)?;
			pending_withdrwal.is_blocked = true;
			<PendingWithdrawals<T>>::insert(block_no, pending_withdrawals);
			Ok(())
		}

		pub fn multi_location_to_account_converter(location: MultiLocation) -> T::AccountId {
			T::AccountIdConvert::convert_ref(location).unwrap()
		}

		pub fn insert_pending_withdrawal(
			block_no: T::BlockNumber,
			pending_withdrawal: PendingWithdrawal,
		) {
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(block_no);
			pending_withdrawals.try_push(pending_withdrawal).unwrap();
			<PendingWithdrawals<T>>::insert(block_no, pending_withdrawals);
		}

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

		pub fn convert_location_to_asset_id(location: MultiLocation) -> Option<u128> {
			Self::generate_asset_id_for_parachain(AssetId::Concrete(location)).ok()
		}
	}
}
