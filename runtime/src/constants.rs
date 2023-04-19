pub mod currency {
	pub type Balance = u128;
	pub const PDEX: Balance = 1_000_000_000_000;
	pub const DOLLARS: Balance = PDEX; // 1_000_000_000_000
	pub const CENTS: Balance = DOLLARS / 100; // 10_000_000_000
}
