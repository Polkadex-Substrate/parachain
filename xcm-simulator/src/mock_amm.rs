use sp_runtime::DispatchError;
use support::{AMM, Pool};

struct MockedAMM<AccountId, CurrencyId, Balance, BlockNumber> (sp_std::marker::PhantomData<(AccountId, CurrencyId, Balance, BlockNumber)>);

impl<AccountId, CurrencyId, Balance, BlockNumber> AMM<AccountId, CurrencyId, Balance, BlockNumber> for MockedAMM<AccountId, CurrencyId, Balance, BlockNumber> {
    fn get_amounts_out(amount_in: Balance, path: Vec<CurrencyId>) -> Result<Vec<Balance>, DispatchError> {
        todo!()
    }

    fn get_amounts_in(amount_out: Balance, path: Vec<CurrencyId>) -> Result<Vec<Balance>, DispatchError> {
        todo!()
    }

    fn swap(who: &AccountId, pair: (CurrencyId, CurrencyId), amount_in: Balance) -> Result<(), DispatchError> {
        Ok(())
    }

    fn get_pools() -> Result<Vec<(CurrencyId, CurrencyId)>, DispatchError> {
        todo!()
    }

    fn get_pool_by_lp_asset(asset_id: CurrencyId) -> Option<(CurrencyId, CurrencyId, Pool<CurrencyId, Balance, BlockNumber>)> {
        todo!()
    }

    fn get_pool_by_asset_pair(pair: (CurrencyId, CurrencyId)) -> Option<Pool<CurrencyId, Balance, BlockNumber>> {
        todo!()
    }
}