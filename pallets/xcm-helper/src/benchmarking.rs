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

const SEED: u32 = 0;

benchmarks! {
	create_parachain_asset {
		let b in 1 .. 1000;
		let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2011)));
		let asset_id = AssetId::Concrete(asset_location);
	}: _(RawOrigin::Root, Box::new(asset_id))

	whitelist_token {
		let b in 1 .. 1000;
		let token = b as u128;
	}: _(RawOrigin::Root, token)

	transfer_fee {
		let b in 1 .. 1000;
		let pallet_account: T::AccountId = T::AssetHandlerPalletId::get().into_account_truncating();
		T::Currency::deposit_creating(&pallet_account, 2_000_000_000_000_000u128.saturated_into());
		let recipeint: T::AccountId = account("mem1", b, SEED);
	}: _(RawOrigin::Root, recipeint, 1_000_000_000_000_000u128.saturated_into())

}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(XcmHelper, crate::mock::new_test_ext(), crate::mock::Test);
