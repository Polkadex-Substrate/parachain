use sp_runtime::DispatchError;
use support::{Pool, AMM};

pub struct MockedAMM<AccountId, CurrencyId, Balance, BlockNumber>(
	sp_std::marker::PhantomData<(AccountId, CurrencyId, Balance, BlockNumber)>,
);

impl<AccountId, CurrencyId, Balance, BlockNumber> AMM<AccountId, CurrencyId, Balance, BlockNumber>
	for MockedAMM<AccountId, CurrencyId, Balance, BlockNumber>
{
	fn get_amounts_out(
		_amount_in: Balance,
		_path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError> {
		unimplemented!()
	}

	fn get_amounts_in(
		amount_out: Balance,
		_path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError> {
		Ok(vec![amount_out])
	}

	fn swap(
		_who: &AccountId,
		_pair: (CurrencyId, CurrencyId),
		_amount_in: Balance,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	fn get_pools() -> Result<Vec<(CurrencyId, CurrencyId)>, DispatchError> {
		unimplemented!()
	}

	fn get_pool_by_lp_asset(
		_asset_id: CurrencyId,
	) -> Option<(CurrencyId, CurrencyId, Pool<CurrencyId, Balance, BlockNumber>)> {
		unimplemented!()
	}

	fn get_pool_by_asset_pair(
		_pair: (CurrencyId, CurrencyId),
	) -> Option<Pool<CurrencyId, Balance, BlockNumber>> {
		unimplemented!()
	}
}
