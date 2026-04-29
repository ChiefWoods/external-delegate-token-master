#![cfg_attr(not(test), no_std)]

use quasar_lang::prelude::*;

mod errors;
mod instructions;
mod state;
use instructions::*;

declare_id!("H5J4kYmHhTnAfvjpUwaQQf7VaKgXoPubGn2pYB9NFdvR");

#[program]
mod external_delegate_token_master {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<Initialize>) -> Result<(), ProgramError> {
        ctx.accounts.initialize()
    }
}

#[cfg(test)]
mod tests;
