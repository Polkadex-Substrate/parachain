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
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
	use frame_support::sp_runtime::traits::AccountIdConversion;

	//TODO Replace this with TheaMessages #Issue: 38
	#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, Copy)]
	pub enum TheaMessage<AccountId> {
		/// AssetDeposited(Recipient, AssetId, Amount)
		AssetDeposited(AccountId, u128, u128)
	}

	#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode)]
	pub struct IngressMessageLimit;

	impl Get<u32> for IngressMessageLimit {
		fn get() -> u32 {
			20 // TODO: Arbitrary value
		}
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Pallet Id
		#[pallet::constant]
		type AssetHandlerPalletId: Get<PalletId>;
	}

	// Queue for enclave ingress messages
	#[pallet::storage]
	#[pallet::getter(fn ingress_messages)]
	pub(super) type IngressMessages<T: Config> = StorageValue<
		_,
		BoundedVec<TheaMessage<T::AccountId>, IngressMessageLimit>,
		ValueQuery,
	>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset Deposited from XCM
		/// parameters. [recipient, asset_id, amount]
		AssetDeposited(T::AccountId, u128, u128),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Invalid Sender
		InvalidSender,
		/// Ingress Messages Limit Reached
		IngressMessagesLimitReached
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			<IngressMessages<T>>::kill();
			Weight::default()
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		// TODO: Assets Pallet and AssetId convertor is not implemented yet. Issue :#35
		///Deposit to Orderbook
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().writes(1))]
		pub fn deposit_asset(origin: OriginFor<T>, recipient: T::AccountId, asset_id: u128, amount: u128) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<IngressMessages<T>>::try_mutate(|ingress_messages| {
				ingress_messages
					.try_push(TheaMessage::AssetDeposited(recipient.clone(), asset_id, amount))
			}).map_err(|_| Error::<T>::IngressMessagesLimitReached)?;
			Self::deposit_event(Event::AssetDeposited(recipient, asset_id, amount));
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_pallet_account() -> T::AccountId {
			T::AssetHandlerPalletId::get().into_account_truncating()
		}
	}
}
