use quasar_lang::{prelude::*, sysvars::Sysvar};
use solana_keccak_hasher::hash as keccak_hash;
use solana_nostd_secp256k1_recover::secp256k1_recover;

use crate::{constants::CENTRAL_AUTHORITY, errors::ExternalDelegateTokenMasterError, state::Vault};

#[derive(Accounts)]
pub struct TransferFunds {
    pub payer: Signer,
    #[account(mut)]
    pub recipient: UncheckedAccount,
    #[account(mut)]
    pub vault: Account<Vault>,
}

impl TransferFunds {
    #[inline(always)]
    pub fn transfer_funds(
        &mut self,
        amount: u64,
        hash: [u8; 32],
        recovery_id: u8,
        signature: [u8; 64],
    ) -> Result<(), ProgramError> {
        // verify hash if payer is not central authority
        if self.payer.address() != &CENTRAL_AUTHORITY {
            // check if vault pda is valid
            let vault_pda = Address::find_program_address(&[b"vault", self.vault.authority.as_ref()], &crate::ID).0;

            if &vault_pda != self.vault.address() {
                return Err(ExternalDelegateTokenMasterError::InvalidVault.into());
            }

            // only central authority or vault authority can transfer funds
            if &self.vault.authority != self.payer.address() {
                return Err(ExternalDelegateTokenMasterError::Unauthorized.into());
            }

            let is_odd = match recovery_id {
                0 | 27 => false,
                1 | 28 => true,
                _ => return Err(ExternalDelegateTokenMasterError::InvalidEthAddress.into()),
            };

            let recovered_pubkey = secp256k1_recover(&hash, is_odd, &signature).unwrap();
            let hashed_pubkey = keccak_hash(&recovered_pubkey).to_bytes();
            let recovered_eth_address: [u8; 20] = hashed_pubkey[12..32]
                .try_into()
                .map_err(|_| ExternalDelegateTokenMasterError::InvalidEthAddress)?;

            if recovered_eth_address != self.vault.eth_address {
                return Err(ExternalDelegateTokenMasterError::InvalidEthAddress.into());
            }
        };

        // transfer funds out from vault to recipient
        let vault_min_rent = Rent::get()?.minimum_balance_unchecked(Vault::SPACE);
        let vault_view = self.vault.to_account_view();
        let vault_lamports = vault_view.lamports();

        if vault_lamports < vault_min_rent.checked_add(amount).ok_or(ExternalDelegateTokenMasterError::MathOverflow)? {
            return Err(ExternalDelegateTokenMasterError::InsufficientVaultFunds.into())
        }

        let recipient_view = self.recipient.to_account_view();
        let recipient_lamports = recipient_view.lamports();

        set_lamports(&vault_view, vault_lamports.checked_sub(amount).ok_or(ExternalDelegateTokenMasterError::MathOverflow)?);
        set_lamports(&recipient_view, recipient_lamports.checked_add(amount).ok_or(ExternalDelegateTokenMasterError::MathOverflow)?);
        
        Ok(())
    }
}
