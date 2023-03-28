#![cfg_attr(not(feature = "std"), no_std)]

//! Thea Council Pallet
//!
//! Thea Council Pallet provides functionality to maintain council members on Parachain.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Overview
//!
//! Thea Council Pallet provides following functionalities:-
//!
//! - Adds member to Council.
//! - Removes member from Council.
//! - Block Transaction.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//! - `add_member` - Adds member to council.
//! - `remove_member` - Removes member from council.
//! - `claim_membership` - Converts Council member status from pending to Active.
//! - `delete_transaction` - Blocks withdrawal request.
//!
//! ### Public Inspection functions - Immutable (getters)
//! - `is_council_member` - Checks if given member is council member.
//!
//! ### Storage Items
//! - `ActiveCouncilMembers` - Stores Active Council Member List.
//! - `PendingCouncilMembers` - Stores Pending Council Member List.
//! - `Proposals` - Stores active proposals.
//! -
//! # Events
//! - `NewPendingMemberAdded` - New Pending Member added.
//! - `NewActiveMemberAdded` - New Active Member added.
//! - `MemberRemoved` - Council Member removed.
//! - `TransactionDeleted` - Transaction blocked.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone)]
	pub enum Proposal<AccountId> {
		AddNewMember(AccountId),
		RemoveExistingMember(AccountId),
	}

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone, Eq, PartialEq, Debug)]
	pub struct Voted<AccountId>(pub AccountId);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + xcm_helper::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	/// Active Council Members
	#[pallet::storage]
	#[pallet::getter(fn get_council_members)]
	pub(super) type ActiveCouncilMembers<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, ConstU32<10>>, ValueQuery>;

	/// Pending Council Members
	#[pallet::storage]
	#[pallet::getter(fn get_pending_council_members)]
	pub(super) type PendingCouncilMembers<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, ConstU32<10>>, ValueQuery>;

	/// Proposals
	#[pallet::storage]
	#[pallet::getter(fn proposal_status)]
	pub(super) type Proposals<T: Config> = StorageMap<
		_,
		frame_support::Blake2_128Concat,
		Proposal<T::AccountId>,
		BoundedVec<Voted<T::AccountId>, ConstU32<100>>,
		ValueQuery,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New Council Member Added [new_pending_member]
		NewPendingMemberAdded(T::AccountId),
		/// New active member added [new_active_member]
		NewActiveMemberAdded(T::AccountId),
		/// Member removed [member]
		MemberRemoved(T::AccountId),
		/// Transaction deleted
		TransactionDeleted(u32),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Storage Overflow
		StorageOverflow,
		/// Not a Valid Sender
		BadOrigin,
		/// Already Council Member
		AlreadyMember,
		/// Not Pending Member
		NotPendingMember,
		/// Sender not council member
		SenderNotCouncilMember,
		/// Sender Already Voted
		SenderAlreadyVoted,
		/// Not Active Member
		NotActiveMember,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Adds member to Thea Council.
		///
		/// # Parameters
		///
		/// * `new_member`: AccountId of New Member.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn add_member(origin: OriginFor<T>, new_member: T::AccountId) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_council_member(&sender), Error::<T>::SenderNotCouncilMember);
			Self::do_add_member(sender, new_member)?;
			Ok(())
		}

		/// Removes member from Thea Council.
		///
		/// # Parameters
		///
		/// * `member_to_be_removed`: AccountId for memebr to be removed.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn remove_member(
			origin: OriginFor<T>,
			member_to_be_removed: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_council_member(&sender), Error::<T>::SenderNotCouncilMember);
			Self::do_remove_member(sender, member_to_be_removed)?;
			Ok(())
		}

		/// Converts Pending Council Member to Active Council Member.
		///
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn claim_membership(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_claim_membership(&sender)?;
			Self::deposit_event(Event::<T>::NewActiveMemberAdded(sender));
			Ok(())
		}

		/// Blocks malicious Pending Transaction.
		///
		/// # Parameters
		///
		/// * `block_no`: Block No which contains malicious transaction.
		/// * `index`: Index of Malicious transaction in the list.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn delete_transaction(
			origin: OriginFor<T>,
			block_no: T::BlockNumber,
			index: u32,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_council_member(&sender), Error::<T>::SenderNotCouncilMember);
			xcm_helper::Pallet::<T>::block_by_ele(block_no, index)?;
			Self::deposit_event(Event::<T>::TransactionDeleted(index));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn is_council_member(sender: &T::AccountId) -> bool {
			let active_members = <ActiveCouncilMembers<T>>::get();
			active_members.contains(sender)
		}

		fn do_add_member(sender: T::AccountId, new_member: T::AccountId) -> DispatchResult {
			let proposal = Proposal::AddNewMember(new_member);
			Self::evaluate_proposal(proposal, sender)?;
			Ok(())
		}

		fn do_remove_member(
			sender: T::AccountId,
			member_to_be_removed: T::AccountId,
		) -> DispatchResult {
			let proposal = Proposal::RemoveExistingMember(member_to_be_removed);
			Self::evaluate_proposal(proposal, sender)?;
			Ok(())
		}

		fn evaluate_proposal(
			proposal: Proposal<T::AccountId>,
			sender: T::AccountId,
		) -> DispatchResult {
			let current_votes =
				|votes: &BoundedVec<Voted<T::AccountId>, ConstU32<100>>| -> usize { votes.len() };
			let expected_votes = || -> usize {
				let total_active_council_size = <ActiveCouncilMembers<T>>::get().len();
				total_active_council_size.saturating_mul(2).saturating_div(3)
			};
			let mut remove_proposal = false;
			<Proposals<T>>::try_mutate(proposal.clone(), |votes| {
				ensure!(!votes.contains(&Voted(sender.clone())), Error::<T>::SenderAlreadyVoted);
				votes.try_push(Voted(sender)).map_err(|_| Error::<T>::StorageOverflow)?;
				if current_votes(votes) >= expected_votes() {
					Self::execute_proposal(proposal.clone())?;
					remove_proposal = true;
				}
				Ok::<(), sp_runtime::DispatchError>(())
			})?;
			if remove_proposal {
				Self::remove_proposal(proposal);
			}
			Ok(())
		}

		fn remove_proposal(proposal: Proposal<T::AccountId>) {
			<Proposals<T>>::remove(proposal);
		}

		fn execute_proposal(proposal: Proposal<T::AccountId>) -> DispatchResult {
			match proposal {
				Proposal::AddNewMember(new_member) => Self::execute_add_member(new_member),
				Proposal::RemoveExistingMember(member_to_be_removed) =>
					Self::execute_remove_member(member_to_be_removed),
			}
		}

		fn execute_add_member(new_member: T::AccountId) -> DispatchResult {
			let mut pending_council_member = <PendingCouncilMembers<T>>::get();
			pending_council_member
				.try_push(new_member.clone())
				.map_err(|_| Error::<T>::StorageOverflow)?;
			<PendingCouncilMembers<T>>::put(pending_council_member);
			Self::deposit_event(Event::<T>::NewPendingMemberAdded(new_member));
			Ok(())
		}

		fn execute_remove_member(member_to_be_removed: T::AccountId) -> DispatchResult {
			let mut active_council_member = <ActiveCouncilMembers<T>>::get();
			let index = active_council_member
				.iter()
				.position(|member| *member == member_to_be_removed)
				.ok_or(Error::<T>::NotActiveMember)?;
			active_council_member.remove(index);
			<ActiveCouncilMembers<T>>::put(active_council_member);
			Self::deposit_event(Event::<T>::MemberRemoved(member_to_be_removed));
			Ok(())
		}

		fn do_claim_membership(sender: &T::AccountId) -> DispatchResult {
			let mut pending_members = <PendingCouncilMembers<T>>::get();
			ensure!(pending_members.contains(sender), Error::<T>::NotPendingMember);
			let index = pending_members
				.iter()
				.position(|member| *member == *sender)
				.ok_or(Error::<T>::NotActiveMember)?;
			pending_members.remove(index);
			<PendingCouncilMembers<T>>::put(pending_members);
			let mut active_council_member = <ActiveCouncilMembers<T>>::get();
			active_council_member
				.try_push(sender.clone())
				.map_err(|_| Error::<T>::StorageOverflow)?;
			<ActiveCouncilMembers<T>>::put(active_council_member);
			Ok(())
		}
	}
}
