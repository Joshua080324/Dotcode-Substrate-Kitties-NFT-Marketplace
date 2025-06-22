// Disable the standard library when compiling for WASM
#![cfg_attr(not(feature = "std"), no_std)]

// Include other Rust modules in this pallet
mod impls; // Contains the main business logic for the pallet
mod tests; // Unit tests for this pallet

// Import useful Substrate macros and types
use frame::prelude::*;

// Import traits for working with fungible assets (e.g. balances)
use frame::traits::fungible::Inspect;
use frame::traits::fungible::Mutate;

// Make the pallet module available to external users
pub use pallet::*;

// Define the actual pallet
#[frame::pallet(dev_mode)] // Dev mode enables extra logging and debug features
pub mod pallet {
	use super::*; // Bring external definitions into scope

	// Define the core pallet struct (required for every pallet)
	#[pallet::pallet]
	pub struct Pallet<T>(core::marker::PhantomData<T>); // PhantomData helps compiler handle generics

	// Configuration trait for this pallet; defines external types the pallet needs
	#[pallet::config]
	pub trait Config: frame_system::Config {
		// Event type to emit pallet events into the runtime event pool
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		// Native token used for balance transfers and pricing
		type NativeBalance: Inspect<Self::AccountId> + Mutate<Self::AccountId>;
	}

	// Shortcut type to get the balance type for this runtime
	pub type BalanceOf<T> =
		<<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

	// Define the Kitty struct that represents one digital kitty
	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 32],                 // Unique identifier for the kitty
		pub owner: T::AccountId,          // Account that owns this kitty
		pub price: Option<BalanceOf<T>>,  // Optional price if the kitty is listed for sale
	}

	// Simple counter to keep track of how many kitties have been created
	#[pallet::storage]
	pub(super) type CountForKitties<T: Config> = StorageValue<Value = u32, QueryKind = ValueQuery>;

	// Main storage map: maps kitty DNA to the actual Kitty data
	#[pallet::storage]
	pub(super) type Kitties<T: Config> = StorageMap<Key = [u8; 32], Value = Kitty<T>>;

	// Maps each user (AccountId) to a list of kitties they own (bounded to 100)
	#[pallet::storage]
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		Key = T::AccountId,
		Value = BoundedVec<[u8; 32], ConstU32<100>>, // Up to 100 kitties per account
		QueryKind = ValueQuery,
	>;

	// Events emitted by the pallet (used to notify UI or clients)
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created { owner: T::AccountId }, // A new kitty was created
		Transferred { from: T::AccountId, to: T::AccountId, kitty_id: [u8; 32] }, // A kitty was transferred
		PriceSet { owner: T::AccountId, kitty_id: [u8; 32], new_price: Option<BalanceOf<T>> }, // Price was updated
		Sold { buyer: T::AccountId, kitty_id: [u8; 32], price: BalanceOf<T> }, // Kitty was bought
	}

	// Define possible errors that can occur in pallet operations
	#[pallet::error]
	pub enum Error<T> {
		TooManyKitties,       // Global limit reached for total kitties
		DuplicateKitty,       // A kitty with same DNA already exists
		TooManyOwned,         // User owns too many kitties (over 100)
		TransferToSelf,       // Cannot transfer a kitty to yourself
		NoKitty,              // Kitty not found
		NotOwner,             // Caller does not own the kitty
		NotForSale,           // Kitty is not listed for sale
		MaxPriceTooLow,       // Offered price is less than sale price
	}

	// Define the callable (extrinsic) functions available to the blockchain
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// Public function to create a new kitty
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?; // Ensure transaction is signed
			let dna = Self::gen_dna();        // Generate unique DNA
			Self::mint(who, dna)?;            // Mint the kitty and store it
			Ok(())
		}

		// Public function to transfer a kitty to another user
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			kitty_id: [u8; 32],
		) -> DispatchResult {
			let who = ensure_signed(origin)?;          // Ensure sender is signed
			Self::do_transfer(who, to, kitty_id)?;     // Call internal logic to transfer
			Ok(())
		}

		// Public function to set or unset the price of a kitty
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: [u8; 32],
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;                  // Ensure caller is signed
			Self::do_set_price(who, kitty_id, new_price)?;     // Set the price internally
			Ok(())
		}

		// Public function to buy a kitty if it's listed for sale
		pub fn buy_kitty(
			origin: OriginFor<T>,
			kitty_id: [u8; 32],
			max_price: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;                  // Ensure buyer is signed
			Self::do_buy_kitty(who, kitty_id, max_price)?;     // Handle internal buy logic
			Ok(())
		}
	}
}
