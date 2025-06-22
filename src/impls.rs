use super::*;
// Import frame prelude (includes useful macros and types)
use frame::prelude::*;
// Import the BlakeTwo256 hashing algorithm
use frame::primitives::BlakeTwo256;
// Needed for token preservation options (used in do_buy_kitty)
use frame::traits::tokens::Preservation;
// Trait for generic hash functions
use frame::traits::Hash;

impl<T: Config> Pallet<T> {
	// Function to generate a unique DNA hash for a kitty
	pub fn gen_dna() -> [u8; 32] {
		// Create a tuple of unique values from the current blockchain state
		let unique_payload = (
			frame_system::Pallet::<T>::parent_hash(),        // Hash of the parent block
			frame_system::Pallet::<T>::block_number(),       // Current block number
			frame_system::Pallet::<T>::extrinsic_index(),    // Current extrinsic index (in block)
			CountForKitties::<T>::get(),                     // Current count of kitties created
		);

		// Hash the payload using BlakeTwo256 and convert to [u8; 32] array
		BlakeTwo256::hash_of(&unique_payload).into()
	}

	// Function to mint (create) a new kitty and store it on chain
	pub fn mint(owner: T::AccountId, dna: [u8; 32]) -> DispatchResult {
		// Create the Kitty struct with DNA, owner, and no price initially
		let kitty = Kitty { dna, owner: owner.clone(), price: None };

		// Ensure this DNA does not already exist (no duplicate kitties)
		ensure!(!Kitties::<T>::contains_key(dna), Error::<T>::DuplicateKitty);

		// Get current kitty count
		let current_count: u32 = CountForKitties::<T>::get();

		// Try to increment it, fail with error if overflow
		let new_count = current_count.checked_add(1).ok_or(Error::<T>::TooManyKitties)?;

		// Try to add kitty DNA to the owner's owned list (bounded vec max 100)
		KittiesOwned::<T>::try_append(&owner, dna).map_err(|_| Error::<T>::TooManyOwned)?;

		// Store the new kitty and update the counter
		Kitties::<T>::insert(dna, kitty);
		CountForKitties::<T>::set(new_count);

		// Emit event saying a kitty was created
		Self::deposit_event(Event::<T>::Created { owner });
		Ok(())
	}

	// Logic to transfer a kitty between owners
	pub fn do_transfer(from: T::AccountId, to: T::AccountId, kitty_id: [u8; 32]) -> DispatchResult {
		// Make sure user isn't transferring to self
		ensure!(from != to, Error::<T>::TransferToSelf);

		// Get the kitty, or return error if not found
		let mut kitty = Kitties::<T>::get(kitty_id).ok_or(Error::<T>::NoKitty)?;

		// Ensure the caller is the owner of the kitty
		ensure!(kitty.owner == from, Error::<T>::NotOwner);

		// Update the owner and reset the price
		kitty.owner = to.clone();
		kitty.price = None;

		// Add kitty to new owner's list
		let mut to_owned = KittiesOwned::<T>::get(&to);
		to_owned.try_push(kitty_id).map_err(|_| Error::<T>::TooManyOwned)?;

		// Remove kitty from previous owner's list
		let mut from_owned = KittiesOwned::<T>::get(&from);
		if let Some(ind) = from_owned.iter().position(|&id| id == kitty_id) {
			from_owned.swap_remove(ind); // Efficient remove by swapping with last and popping
		} else {
			return Err(Error::<T>::NoKitty.into()) // Extra check: fail if the kitty was not found
		}

		// Save updated kitty and ownership info
		Kitties::<T>::insert(kitty_id, kitty);
		KittiesOwned::<T>::insert(&to, to_owned);
		KittiesOwned::<T>::insert(&from, from_owned);

		// Emit transfer event
		Self::deposit_event(Event::<T>::Transferred { from, to, kitty_id });
		Ok(())
	}

	// Function to set a new price for a kitty (or unset price if None)
	pub fn do_set_price(
		caller: T::AccountId,
		kitty_id: [u8; 32],
		new_price: Option<BalanceOf<T>>,
	) -> DispatchResult {
		// Get the kitty or return error
		let mut kitty = Kitties::<T>::get(kitty_id).ok_or(Error::<T>::NoKitty)?;

		// Ensure caller is the kitty owner
		ensure!(kitty.owner == caller, Error::<T>::NotOwner);

		// Set new price
		kitty.price = new_price;
		Kitties::<T>::insert(kitty_id, kitty);

		// Emit event
		Self::deposit_event(Event::<T>::PriceSet { owner: caller, kitty_id, new_price });
		Ok(())
	}

	// Logic to buy a kitty
	pub fn do_buy_kitty(
		buyer: T::AccountId,
		kitty_id: [u8; 32],
		price: BalanceOf<T>,
	) -> DispatchResult {
		// Fetch the kitty or fail if doesn't exist
		let kitty = Kitties::<T>::get(kitty_id).ok_or(Error::<T>::NoKitty)?;

		// Ensure the kitty is for sale
		let real_price = kitty.price.ok_or(Error::<T>::NotForSale)?;

		// Buyer must offer equal or higher than listed price
		ensure!(price >= real_price, Error::<T>::MaxPriceTooLow);

		// Transfer funds from buyer to current owner
		T::NativeBalance::transfer(&buyer, &kitty.owner, real_price, Preservation::Preserve)?;

		// Transfer kitty ownership
		Self::do_transfer(kitty.owner, buyer.clone(), kitty_id)?;

		// Emit Sold event
		Self::deposit_event(Event::<T>::Sold { buyer, kitty_id, price: real_price });
		Ok(())
	}
}