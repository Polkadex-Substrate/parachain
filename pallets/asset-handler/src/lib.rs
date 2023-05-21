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

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		fail,
		pallet_prelude::*,
		traits::{
			fungibles::{Create, Inspect, Mutate, Transfer},
			tokens::{
				fungible::Inspect as CurrencyInspect, DepositConsequence, WithdrawConsequence,
			},
			Currency, ExistenceRequirement, ReservableCurrency,
		},
	};

	use sp_runtime::SaturatedConversion;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Balances Pallet
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ CurrencyInspect<Self::AccountId>;
		/// MultiCurrency Pallet
		type MultiCurrency: Create<<Self as frame_system::Config>::AccountId>
			+ Mutate<<Self as frame_system::Config>::AccountId, Balance = u128, AssetId = u128>
			+ Inspect<<Self as frame_system::Config>::AccountId>
			+ Transfer<<Self as frame_system::Config>::AccountId>;
		/// Native Currency Identifier
		type NativeCurrencyId: Get<u128>;
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Storage Overflow
		StorageOverflow,
		/// Not a Valid Sender
		BadOrigin,
		/// Cannot Mint Native Asset
		CannotMintNativeAsset,
		/// Cannot Burn Native Asset
		CannotBurnNativeAsset,
		/// Cannot create native Asset
		CannotCreateNativeAsset,
	}

	impl<T: Config> Create<T::AccountId> for Pallet<T> {
		fn create(
			id: Self::AssetId,
			admin: T::AccountId,
			is_sufficient: bool,
			min_balance: Self::Balance,
		) -> DispatchResult {
			if id != T::NativeCurrencyId::get() {
				T::MultiCurrency::create(id, admin, is_sufficient, min_balance)
			} else {
				Err(Error::<T>::CannotCreateNativeAsset.into())
			}
		}
	}

	impl<T: Config> Inspect<T::AccountId> for Pallet<T> {
		type AssetId = u128;
		type Balance = u128;

		fn total_issuance(asset: Self::AssetId) -> Self::Balance {
			// when asset is not polkadex
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::total_issuance(asset.saturated_into()).saturated_into()
			} else {
				<<T as Config>::Currency as frame_support::traits::fungible::Inspect<
					T::AccountId,
				>>::total_issuance()
				.saturated_into()
			}
		}

		fn minimum_balance(asset: Self::AssetId) -> Self::Balance {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::minimum_balance(asset.saturated_into()).saturated_into()
			} else {
				<<T as Config>::Currency as frame_support::traits::fungible::Inspect<
					T::AccountId,
				>>::minimum_balance()
				.saturated_into()
			}
		}

		fn balance(asset: Self::AssetId, who: &T::AccountId) -> Self::Balance {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::balance(asset.saturated_into(), who).saturated_into()
			} else {
				T::Currency::total_balance(who).saturated_into()
			}
		}

		fn reducible_balance(
			asset: Self::AssetId,
			who: &T::AccountId,
			keep_alive: bool,
		) -> Self::Balance {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::reducible_balance(asset.saturated_into(), who, keep_alive)
					.saturated_into()
			} else {
				<<T as Config>::Currency as frame_support::traits::fungible::Inspect<
					T::AccountId,
				>>::reducible_balance(who, true)
				.saturated_into()
			}
		}

		fn can_deposit(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
			mint: bool,
		) -> DepositConsequence {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::can_deposit(asset, who, amount.saturated_into(), mint)
			} else {
				// balance of native asset can always be increased
				DepositConsequence::Success
			}
		}

		fn can_withdraw(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> WithdrawConsequence<Self::Balance> {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::can_withdraw(asset.saturated_into(), who, amount.saturated_into())
			} else if T::Currency::free_balance(who) >= amount.saturated_into() {
				WithdrawConsequence::Success
			} else {
				WithdrawConsequence::UnknownAsset
			}
		}

		fn asset_exists(asset: Self::AssetId) -> bool {
			if asset == T::NativeCurrencyId::get() {
				true
			} else {
				T::MultiCurrency::asset_exists(asset)
			}
		}
	}

	impl<T: Config> Transfer<T::AccountId> for Pallet<T> {
		fn transfer(
			asset: Self::AssetId,
			source: &T::AccountId,
			dest: &T::AccountId,
			amount: Self::Balance,
			keep_alive: bool,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::transfer(asset, source, dest, amount.saturated_into(), keep_alive)
					.map(|x| x.saturated_into())
			} else {
				let existence_requirement = if keep_alive {
					ExistenceRequirement::KeepAlive
				} else {
					ExistenceRequirement::AllowDeath
				};
				T::Currency::transfer(
					source,
					dest,
					amount.saturated_into(),
					existence_requirement,
				)?;
				Ok(amount)
			}
		}
	}

	impl<T: Config> Mutate<T::AccountId> for Pallet<T> {
		fn mint_into(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> DispatchResult {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::mint_into(asset, who, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				T::Currency::deposit_creating(who, amount.saturated_into());
				Ok(())
			}
		}

		fn burn_from(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::burn_from(asset, who, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				fail!(Error::<T>::CannotBurnNativeAsset)
			}
		}

		fn slash(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::slash(asset, who, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				let (_, balance) = T::Currency::slash(who, amount.saturated_into());
				Ok(balance.saturated_into())
			}
		}

		fn teleport(
			asset: Self::AssetId,
			source: &T::AccountId,
			dest: &T::AccountId,
			amount: Self::Balance,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::MultiCurrency::teleport(asset, source, dest, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				T::Currency::transfer(
					source,
					dest,
					amount.saturated_into(),
					ExistenceRequirement::KeepAlive,
				)?;
				Ok(amount)
			}
		}
	}
}
