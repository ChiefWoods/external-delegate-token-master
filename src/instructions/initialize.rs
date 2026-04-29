use quasar_lang::prelude::*;

use crate::state::{Vault, VaultInner};

#[derive(Accounts)]
pub struct InitializeVault {
    #[account(mut)]
    pub authority: Signer,
    #[account(
        mut,
        init,
        payer = authority,
        seeds = Vault::seeds(authority),
        bump,
    )]
    pub vault: Account<Vault>,
    pub system_program: Program<System>,
}

impl InitializeVault {
    #[inline(always)]
    pub fn initialize_vault(&mut self, bumps: InitializeVaultBumps, eth_address: [u8; 20]) -> Result<(), ProgramError> {
        self.vault.set_inner(VaultInner {
            authority: *self.authority.address(),
            eth_address,
            bump: bumps.vault,
        });

        Ok(())
    }
}
