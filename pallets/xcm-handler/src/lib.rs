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
	use frame_support::{
		dispatch::{DispatchResultWithPostInfo, RawOrigin},
		pallet_prelude::*,
		sp_runtime::traits::AccountIdConversion,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::sp_std;
	use sp_runtime::SaturatedConversion;
	use xcm::{
		latest::{Error as XcmError, MultiAsset, MultiLocation, Result},
		v2::WeightLimit,
		VersionedMultiAssets, VersionedMultiLocation,
	};
	use xcm_executor::{traits::TransactAsset, Assets};

	//TODO Replace this with TheaMessages #Issue: 38
	#[derive(Encode, Decode, TypeInfo)]
	pub enum TheaMessage {
		/// AssetDeposited(Recipient, Asset & Amount)
		AssetDeposited(MultiLocation, MultiAsset),
		/// Thea Key Set by Sudo
		TheaKeySetBySudo([u8; 64]),
		/// New Thea Key Set by Current Relayer Set
		TheaKeyChanged([u8; 64])
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
		asset: sp_std::boxed::Box<VersionedMultiAssets>,
		destination: sp_std::boxed::Box<VersionedMultiLocation>,
		is_blocked: bool,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + orml_xtokens::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Pallet Id
		#[pallet::constant]
		type AssetHandlerPalletId: Get<PalletId>;
		/// Pallet Id
		#[pallet::constant]
		type WithdrawalExecutionBlockDiff: Get<Self::BlockNumber>;
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
		AssetWithdrawn(MultiLocation, MultiAsset),
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
		/// Ingress Message Vector full
		IngressMessagesFull
		/// Nonce is not valid
		NonceIsNotValid,
		/// Index not found
		IndexNotFound,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let mut failed_withdrawal: BoundedVec<PendingWithdrawal, ConstU32<100>> =
				BoundedVec::default();
			<PendingWithdrawals<T>>::mutate(n, |withdrawals| {
				while let Some(withdrawal) = withdrawals.pop() {
					if !withdrawal.is_blocked {
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
								.map_err(|_| Error::<T>::IngressMessagesLimitReached) //TODO: Change the error
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
			let pubic_key = <ActiveTheaKey<T>>::get().ok_or(Error::<T>::PublicKeyNotSet)?;
			// let encoded_payload = Encode::encode(&new_thea_key);
			let payload_hash = sp_io::hashing::keccak_256(&new_thea_key.to_vec());
			if Self::verify_ecdsa_prehashed(&signature, &pubic_key, &payload_hash)? {
				<ActiveTheaKey<T>>::set(Some(new_thea_key));
				<IngressMessages<T>>::try_mutate(|ingress_messages| {
					ingress_messages.try_push(
						TheaMessage::TheaKeyChanged(new_thea_key)
					)
				}).map_err(|_| Error::<T>::IngressMessagesFull)?;
			} else {
				return Err(Error::<T>::SignatureVerificationFailed.into());
			}
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
			<IngressMessages<T>>::try_mutate(|ingress_messages| {
				ingress_messages.try_push(
					TheaMessage::TheaKeySetBySudo(thea_key)
				)
			}).map_err(|_| Error::<T>::IngressMessagesFull)?;
			Ok(().into())
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
			Self::deposit_event(Event::<T>::AssetWithdrawn(who.clone(), what.clone()));
			Ok(what.clone().into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_pallet_account() -> T::AccountId {
			T::AssetHandlerPalletId::get().into_account_truncating()
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
	}
}
