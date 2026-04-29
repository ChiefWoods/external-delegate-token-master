use quasar_lang::prelude::*;

#[account(discriminator = 1, set_inner)]
#[seeds(b"vault", authority: Address)]
pub struct Vault {
    pub authority: Address,
    pub eth_address: [u8; 20],
    pub bump: u8,
}
