use super::*;

#[allow(unused)]
use crate::Pallet as XcmHelper;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{
    sp_runtime::{traits::Hash, SaturatedConversion},
    traits::{fungibles::Mutate, Currency},
};
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use frame_benchmarking::Box;
use xcm_builder::test_utils::{AssetId, Junction, Junctions, MultiLocation, NetworkId};

const SEED: u32 = 0;

benchmarks! {
    set_thea_key {
        let b in 1 .. 1000;
        let thea_key: [u8;64] = [b as u8;64];
    }: _(RawOrigin::Root, thea_key)

    create_parachain_asset {
        let b in 1 .. 1000;
        let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2011)));
        let asset_id = AssetId::Concrete(asset_location);
    }: _(RawOrigin::Root, Box::new(asset_id))

    whitelist_token {
        let b in 1 .. 1000;
        let token = b as u128;
    }: _(RawOrigin::Root, token)

    withdraw_asset {
        let b in 1 .. 1000;
        let destination = MultiLocation::new(1, Junctions::X2(Junction::Parachain(2011), Junction::AccountId32 { network: NetworkId::Any, id: [b as u8;32] }));
        let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2011)));
        let asset_id = AssetId::Concrete(asset_location);
        //TODO Create Asset Id
        VersionedMultiLocation::
    }
}