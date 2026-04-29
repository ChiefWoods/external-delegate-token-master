#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;

mod errors;
mod instructions;
mod state;
mod constants;
use instructions::*;

declare_id!("H5J4kYmHhTnAfvjpUwaQQf7VaKgXoPubGn2pYB9NFdvR");

#[program]
mod external_delegate_token_master {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize_vault(ctx: Ctx<InitializeVault>, eth_address: [u8; 20]) -> Result<(), ProgramError> {
        ctx.accounts.initialize_vault(ctx.bumps, eth_address)
    }

    #[instruction(discriminator = 1)]
    pub fn transfer_funds(
        ctx: Ctx<TransferFunds>,
        amount: u64,
        recovery_id: u8,
        hash: [u8; 32],
        signature: [u8; 64],
    ) -> Result<(), ProgramError> {
        ctx.accounts
            .transfer_funds(amount, hash, recovery_id, signature)
    }
}

#[cfg(test)]
mod tests;
