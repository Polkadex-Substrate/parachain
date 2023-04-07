use super::*;

#[allow(unused)]
use crate::Pallet as XcmHelper;
use frame_benchmarking::{account, benchmarks, whitelisted_caller, Box};
use frame_support::{
	sp_runtime::{traits::Hash, SaturatedConversion},
	traits::{fungibles::Mutate, Currency, EnsureOrigin},
	BoundedVec,
};
use frame_system::RawOrigin;
use parity_scale_codec::Encode;
use sp_core::{
	ecdsa::{self, Signature},
	ConstU32,
};
use sp_std::vec;
use xcm::{
	latest::{
		Error as XcmError, Fungibility, Junction, Junctions, MultiAsset, MultiAssets,
		MultiLocation, NetworkId,
	},
	v1::AssetId,
	v2::WeightLimit,
	VersionedMultiAssets, VersionedMultiLocation,
};

const SEED: u32 = 0;

benchmarks! {
	set_thea_key {
		let b in 1 .. 1000;
		let public_key = sp_core::ecdsa::Public::from_raw([3, 31, 175, 212, 203, 148, 135, 36, 88, 104, 149, 228, 133, 42, 19, 172, 28, 211, 198, 250, 241, 19, 167, 9, 117, 60, 3, 83, 155, 157, 81, 94, 70]);
	}: _(RawOrigin::Root, public_key)

	create_parachain_asset {
		let b in 1 .. 1000;
		let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2011)));
		let asset_id = AssetId::Concrete(asset_location);
		let account = T::AssetCreateUpdateOrigin::successful_origin();
	}: _(occount, Box::new(asset_id))

	whitelist_token {
		let b in 1 .. 1000;
		let token = b as u128;
		let account = T::AssetCreateUpdateOrigin::successful_origin();
	}: _(RawOrigin::Signed(account), token)

	withdraw_asset {
		let b in 1 .. 1000;
		let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(0)));
		let asset_id = AssetId::Concrete(asset_location);
		let multi_asset = MultiAsset::from((asset_id, Fungibility::Fungible(1000000000)));
		let wrapped_multi_asset = VersionedMultiAssets::V1(MultiAssets::from(multi_asset));
		let boxed_asset_id = Box::new(wrapped_multi_asset);
		let destination = MultiLocation::new(1, Junctions::X2(Junction::Parachain(2011), Junction::AccountId32 { network: NetworkId::Any, id: [1;32] }));
		let boxed_multilocation = Box::new(VersionedMultiLocation::V1(destination));
		let boundec_vec:  BoundedVec<
				(
					sp_std::boxed::Box<VersionedMultiAssets>,
					sp_std::boxed::Box<VersionedMultiLocation>,
				),
				ConstU32<10>> = BoundedVec::try_from(vec![(boxed_asset_id, boxed_multilocation)]
		).unwrap();
		let public_key = sp_core::ecdsa::Public::from_raw([3, 191, 139, 57, 61, 154, 241, 161, 142, 8, 97, 254, 246, 93, 185, 192, 74, 92, 193, 149, 71, 80, 241, 158, 151, 148, 181, 81, 0, 207, 99, 100, 103]);
		<ActiveTheaKey<T>>::put(public_key);
		let signature = sp_core::ecdsa::Signature::from_raw([133, 88, 169, 255, 155, 96, 242, 197, 19, 15, 217, 41, 174, 40, 125, 62, 21, 228, 172, 38, 23, 229, 25, 70, 63, 49, 171, 215, 45, 232, 11, 69, 25, 191, 98, 202, 197, 210, 236, 43, 254, 223, 94, 87, 42, 108, 43, 242, 32, 35, 169, 75, 114, 206, 64, 138, 213, 16, 208, 7, 159, 215, 82, 79, 1]);

		let council_member: T::AccountId = account("mem1", b, SEED);
	}: _(RawOrigin::Signed(council_member), boundec_vec, 0, signature)
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(XcmHelper, crate::mock::new_test_ext(), crate::mock::Test);
