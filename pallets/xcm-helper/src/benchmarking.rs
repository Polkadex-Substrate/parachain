use super::*;

#[allow(unused)]
use crate::Pallet as XcmHelper;
use frame_benchmarking::{account, benchmarks, Box};
use frame_support::{sp_runtime::SaturatedConversion, traits::Currency};
use frame_system::RawOrigin;
use sp_core::Get;
use sp_runtime::traits::AccountIdConversion;
use xcm::{
	latest::{Junction, Junctions, MultiLocation},
	v1::AssetId,
};
use frame_support::traits::fungibles::Mutate;

const SEED: u32 = 0;

benchmarks! {
	whitelist_token {
		let b in 1 .. 1000;
		let token = b as u128;
	}: _(RawOrigin::Root, token)

	transfer_fee {
		let b in 1 .. 1000;
		let pallet_account: T::AccountId = T::AssetHandlerPalletId::get().into_account_truncating();
		T::AssetManager::mint_into(100,&pallet_account, 2_000_000_000_000_000u128.saturated_into());
		let recipeint: T::AccountId = account("mem1", b, SEED);
	}: _(RawOrigin::Root, recipeint)

}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(XcmHelper, crate::mock::new_test_ext(), crate::mock::Test);
