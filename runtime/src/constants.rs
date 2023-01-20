pub mod currency {
	pub type Balance = u128;

	pub const PDEX: Balance = 1_000_000_000_000;
	pub const UNITS: Balance = PDEX;
	pub const DOLLARS: Balance = PDEX; // 1_000_000_000_000
	pub const CENTS: Balance = DOLLARS / 100; // 10_000_000_000
	pub const MILLICENTS: Balance = CENTS / 1_000; // 1000_000_000

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}
