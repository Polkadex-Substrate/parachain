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

mod parachain;
mod relay_chain;

use crate::parachain::XcmHandler;
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
use parachain::{RuntimeEvent, System};
use polkadot_parachain::primitives::Id as ParaId;
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub const AssetHandlerPalletId: PalletId = PalletId(*b"XcmHandl");
pub const ALICE: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([
	109, 111, 100, 108, 88, 99, 109, 72, 97, 110, 100, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0,
]);
pub const BOB: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([0; 32]);
pub const INITIAL_BALANCE: u128 = 1_000_000_000;

decl_test_parachain! {
	pub struct ParaA {
		Runtime = parachain::Runtime,
		XcmpMessageHandler = parachain::MsgQueue,
		DmpMessageHandler = parachain::MsgQueue,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = parachain::Runtime,
		XcmpMessageHandler = parachain::MsgQueue,
		DmpMessageHandler = parachain::MsgQueue,
		new_ext = para_ext(2),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay_chain::Runtime,
		XcmConfig = relay_chain::XcmConfig,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
		],
	}
}

pub fn para_account_id(id: u32) -> relay_chain::AccountId {
	ParaId::from(id).into_account_truncating()
}

pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
	use parachain::{MsgQueue, Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> { balances: vec![(ALICE, INITIAL_BALANCE)] }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		MsgQueue::set_para_id(para_id.into());
	});
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay_chain::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE), (para_account_id(1), INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;
pub type ParachainPalletXcm = pallet_xcm::Pallet<parachain::Runtime>;

#[cfg(test)]
mod tests {
	use super::*;

	use codec::Encode;
	use frame_support::{
		assert_noop, assert_ok, metadata::StorageEntryModifier::Default, traits::Currency,
	};
	use polkadot_core_primitives::AccountId;
	use xcm::{
		latest::prelude::*, VersionedMultiAsset, VersionedMultiAssets, VersionedMultiLocation,
	};
	use xcm_handler::{Error, PendingWithdrawal};
	use xcm_simulator::TestExt;

	// Helper function for forming buy execution message
	fn buy_execution<C>(fees: impl Into<MultiAsset>) -> Instruction<C> {
		BuyExecution { fees: fees.into(), weight_limit: Unlimited }
	}

	//TODO: Fix this.
	// #[test]
	// fn reserve_transfer() {
	// 	MockNet::reset();
	//
	// 	let withdraw_amount = 123;
	//
	// 	Relay::execute_with(|| {
	// 		assert_ok!(RelayChainPalletXcm::(
	// 			relay_chain::RuntimeOrigin::signed(ALICE),
	// 			Box::new(X1(Parachain(1u32)).into().into()),
	// 			Box::new(X1(AccountId32 { network: Any, id: ALICE.into() }).into().into()),
	// 			Box::new((Here, withdraw_amount).into()),
	// 			0,
	// 		));
	// 		assert_eq!(
	// 			parachain::Balances::free_balance(&para_account_id(1)),
	// 			INITIAL_BALANCE + withdraw_amount
	// 		);
	// 	});
	//
	// 	ParaA::execute_with(|| {
	// 		// free execution, full amount received
	// 		assert_eq!(
	// 			pallet_balances::Pallet::<parachain::Runtime>::free_balance(&ALICE),
	// 			INITIAL_BALANCE
	// 		);
	// 	});
	//
	// 	ParaA::execute_with(|| {
	// 		assert!(System::events().iter().any(|r| matches!(
	// 			r.event,
	// 			RuntimeEvent::XcmHandler(xcm_handler::Event::AssetDeposited { .. })
	// 		)));
	// 	});
	// }

	#[test]
	fn test_withdraw_from_parachain_to_relay_chain() {
		MockNet::reset();
		Relay::execute_with(|| {
			assert_eq!(
				pallet_balances::Pallet::<parachain::Runtime>::free_balance(&para_account_id(1)),
				1_000_000_000
			);
			assert_eq!(
				pallet_balances::Pallet::<parachain::Runtime>::free_balance(&ALICE.into()),
				1_000_000_000
			);
		});
		ParaA::execute_with(|| {
			let multi_asset = MultiAsset {
				id: AssetId::Concrete(Parent.into()),
				fun: Fungibility::Fungible(1_000_000u128),
			};
			let multi_assets = VersionedMultiAssets::V1(MultiAssets::from(vec![multi_asset]));
			let dest = MultiLocation::new(
				1,
				X1(Junction::AccountId32 { network: NetworkId::Any, id: ALICE.into() }),
			);
			let versioned_dest = VersionedMultiLocation::V1(dest);
			create_asset();
			mint_dot_token(ALICE);
			assert_ok!(orml_xtokens::module::Pallet::<parachain::Runtime>::transfer_multiassets(
				Some(ALICE).into(),
				Box::from(multi_assets),
				0,
				Box::from(versioned_dest),
				WeightLimit::Unlimited
			));
		});
		Relay::execute_with(|| {
			assert_eq!(
				pallet_balances::Pallet::<parachain::Runtime>::free_balance(&para_account_id(1)),
				999_000_000
			);
			assert_eq!(
				pallet_balances::Pallet::<parachain::Runtime>::free_balance(&ALICE.into()),
				1001_000_000
			);
		});
	}

	#[test]
	fn test_relay_chain_asset_to_sibling() {
		MockNet::reset();
		Relay::execute_with(|| {
			assert_eq!(
				pallet_balances::Pallet::<parachain::Runtime>::free_balance(&para_account_id(1)),
				1_000_000_000
			);
		});

		ParaA::execute_with(|| {
			let multi_asset = MultiAsset {
				id: AssetId::Concrete(Parent.into()),
				fun: Fungibility::Fungible(1_000_000u128),
			};
			let multi_assets = VersionedMultiAssets::V1(MultiAssets::from(vec![multi_asset]));
			let dest = MultiLocation::new(
				1,
				X2(
					Parachain(2),
					Junction::AccountId32 { network: NetworkId::Any, id: ALICE.into() },
				),
			);

			// Mint
			create_asset();
			mint_native_token(ALICE);
			mint_dot_token(ALICE);
			let versioned_dest = VersionedMultiLocation::V1(dest);
			assert_ok!(orml_xtokens::module::Pallet::<parachain::Runtime>::transfer_multiassets(
				Some(ALICE).into(),
				Box::from(multi_assets),
				0,
				Box::from(versioned_dest),
				WeightLimit::Unlimited
			));
			// Check Balance of Native Account
			let actual_balance = get_dot_balance(ALICE);
			assert_eq!(actual_balance, 99_999_999_000_000);
		});

		ParaB::execute_with(|| {
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmHandler(xcm_handler::Event::AssetDeposited(..))
			)));
		});
	}

	#[test]
	fn test_send_sibling_asset_to_reserve_sibling() {
		MockNet::reset();
		ParaA::execute_with(|| {
			let multi_asset = MultiAsset {
				id: AssetId::Concrete(MultiLocation {
					parents: 1,
					interior: Junctions::X1(Parachain(1)),
				}),
				fun: Fungibility::Fungible(1_000_000u128),
			};
			let multi_assets = VersionedMultiAssets::V1(MultiAssets::from(vec![multi_asset]));
			let dest = MultiLocation::new(
				1,
				X2(
					Parachain(2),
					Junction::AccountId32 { network: NetworkId::Any, id: ALICE.into() },
				),
			);
			let versioned_dest = VersionedMultiLocation::V1(dest);

			assert_ok!(orml_xtokens::module::Pallet::<parachain::Runtime>::transfer_multiassets(
				Some(ALICE).into(),
				Box::from(multi_assets),
				0,
				Box::from(versioned_dest),
				WeightLimit::Unlimited
			));
			let other_chain =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(2)) };
			let other_parachain_account =
				XcmHandler::multi_location_to_account_converter(other_chain);
			assert_eq!(Balances::free_balance(other_parachain_account), 1000000);
		});

		ParaB::execute_with(|| {
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmHandler(xcm_handler::Event::AssetDeposited(..))
			)));
		});
	}

	// Bob Placing order
	#[test]
	fn test_withdraw_from_parachain_to_relay_chain_with_wrong_account_will_return_error() {
		MockNet::reset();
		Relay::execute_with(|| {
			assert_eq!(
				pallet_balances::Pallet::<parachain::Runtime>::free_balance(&para_account_id(1)),
				1_000_000_000
			);
			assert_eq!(
				pallet_balances::Pallet::<parachain::Runtime>::free_balance(&ALICE.into()),
				1_000_000_000
			);
		});
		ParaA::execute_with(|| {
			let multi_asset = MultiAsset {
				id: AssetId::Concrete(Parent.into()),
				fun: Fungibility::Fungible(1_000_000u128),
			};
			let multi_assets = VersionedMultiAssets::V1(MultiAssets::from(vec![multi_asset]));
			let dest = MultiLocation::new(
				1,
				X1(Junction::AccountId32 { network: NetworkId::Any, id: BOB.into() }),
			);
			let versioned_dest = VersionedMultiLocation::V1(dest);
			assert_noop!(
				orml_xtokens::module::Pallet::<parachain::Runtime>::transfer_multiassets(
					Some(BOB).into(),
					Box::from(multi_assets),
					0,
					Box::from(versioned_dest),
					WeightLimit::Unlimited
				),
				orml_xtokens::Error::<parachain::Runtime>::XcmExecutionFailed
			);
		});
	}

	#[test]
	fn test_on_initialize_with_pdex_deposit_to_polkadex_parachain() {
		MockNet::reset();
		ParaA::execute_with(|| {
			//Add to pending withdrawal
			let location =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(1)) };
			let asset_id = AssetId::Concrete(location);
			let asset = MultiAsset { id: asset_id, fun: Fungibility::Fungible(1_000_000_000_000) };
			let destination = MultiLocation {
				parents: 0,
				interior: Junctions::X1(Junction::AccountId32 {
					network: NetworkId::Any,
					id: [1; 32],
				}),
			};
			let pending_withdrawal = PendingWithdrawal {
				asset: Box::new(asset.into()),
				destination: Box::new(destination.into()),
				is_blocked: false,
			};
			XcmHandler::insert_pending_withdrawal(100, pending_withdrawal);
			System::set_block_number(99);
			run_to_block(100);
			assert_eq!(
				Balances::free_balance(sp_core::crypto::AccountId32::new([1; 32])),
				1_000_000_000_000
			);
		});
	}

	#[test]
	fn test_on_initialize_with_non_native_asset_deposit_to_polkadex_parachain() {
		MockNet::reset();
		ParaA::execute_with(|| {
			let location =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(2)) };
			let asset_id = AssetId::Concrete(location);
			let asset = MultiAsset { id: asset_id, fun: Fungibility::Fungible(1_000_000_000_000) };
			let destination = MultiLocation {
				parents: 0,
				interior: Junctions::X1(Junction::AccountId32 {
					network: NetworkId::Any,
					id: [1; 32],
				}),
			};
			let pending_withdrawal = PendingWithdrawal {
				asset: Box::new(asset.into()),
				destination: Box::new(destination.into()),
				is_blocked: false,
			};
			create_dot_asset();
			mint_native_token(sp_core::crypto::AccountId32::new([1; 32]));
			XcmHandler::insert_pending_withdrawal(100, pending_withdrawal);
			System::set_block_number(99);
			run_to_block(100);
		});
	}

	use crate::parachain::{AssetsPallet, LocationToAccountId};
	fn mint_dot_token(account: AccountId) {
		let asset_id = 313675452054768990531043083915731189857u128;
		assert_ok!(AssetsPallet::mint_into(asset_id, &account, 100_000_000_000_000));
	}

	fn get_dot_balance(account: AccountId) -> u128 {
		let asset_id = 313675452054768990531043083915731189857u128;
		AssetsPallet::balance(asset_id, &account)
	}

	use crate::parachain::{Balances, RuntimeOrigin};
	fn mint_native_token(account: AccountId) {
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			account,
			1 * 10_000_000_000_000_000u128,
			0
		));
	}

	fn create_asset() {
		let asset_id = 313675452054768990531043083915731189857u128;
		assert_ok!(AssetsPallet::create(RuntimeOrigin::signed(ALICE), asset_id, ALICE, 1));
	}

	fn create_dot_asset() {
		let asset_id = 250795704345233296850763536153850679878u128;
		assert_ok!(AssetsPallet::create(RuntimeOrigin::signed(ALICE), asset_id, ALICE, 1));
	}

	pub fn run_to_block(n: u64) {
		while System::block_number() < n {
			if System::block_number() > 1 {
				System::on_finalize(System::block_number());
			}
			System::set_block_number(System::block_number() + 1);
			System::on_initialize(System::block_number());
			XcmHandler::on_initialize(System::block_number());
		}
	}

	//
}
