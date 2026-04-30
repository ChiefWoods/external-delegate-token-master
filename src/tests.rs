use alloy_primitives::Address as EthAddress;
use k256::ecdsa::SigningKey;
use k256::elliptic_curve::rand_core::OsRng;
use quasar_lang::prelude::SYSTEM_PROGRAM_ID;
use quasar_svm::{
    token::create_keyed_system_account, ExecutionStatus, ProgramError, Pubkey, QuasarSvm,
};
use sha3::{Digest, Keccak256};
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};

use crate::constants::CENTRAL_AUTHORITY;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

fn setup() -> QuasarSvm {
    let elf = std::fs::read("target/deploy/external_delegate_token_master.so").unwrap();
    QuasarSvm::new().with_program(&Pubkey::from(crate::ID), &elf)
}

fn derive_vault_address(authority: Address) -> Address {
    Address::find_program_address(&[b"vault", authority.as_ref()], &crate::ID).0
}

fn set_vault_lamports(svm: &mut QuasarSvm, vault: Address, lamports: u64) {
    let mut vault_acc = svm.get_account(&vault).unwrap();
    vault_acc.lamports = lamports;
    svm.set_account(vault_acc);
}

fn initialize_vault(payer: Address, vault: Address, eth_address: [u8; 20]) -> Instruction {
    let mut data = vec![0];
    data.extend_from_slice(&eth_address);

    Instruction {
        program_id: Address::from(crate::ID.to_bytes()),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

fn transfer_instruction(
    payer: Address,
    recipient: Address,
    vault: Address,
    amount: u64,
    hash: [u8; 32],
    recovery_id: u8,
    signature: [u8; 64],
) -> Instruction {
    let mut data = vec![1];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(recovery_id);
    data.extend_from_slice(&hash);
    data.extend_from_slice(&signature);

    Instruction {
        program_id: Address::from(crate::ID.to_bytes()),
        accounts: vec![
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new(recipient, false),
            AccountMeta::new(vault, false),
        ],
        data,
    }
}

fn sign_payload(signing_key: &SigningKey, payload: &[u8]) -> ([u8; 32], u8, [u8; 64]) {
    let mut hasher = Keccak256::new();
    hasher.update(payload);
    let hash: [u8; 32] = hasher.finalize().into();

    let (sig, recid) = signing_key
        .sign_prehash_recoverable(&hash)
        .expect("failed to sign prehash");
    let signature: [u8; 64] = sig.to_bytes().into();
    let recovery_id = recid.to_byte();

    (hash, recovery_id, signature)
}

#[test]
fn test_initialize_vault() {
    let mut svm = setup();

    let payer = Pubkey::new_unique();
    let vault: Pubkey = derive_vault_address(payer);
    let eth_address = [0; 20];

    let instruction = initialize_vault(payer, vault, eth_address);

    let result = svm.process_instruction(
        &instruction,
        &[
            create_keyed_system_account(&payer, LAMPORTS_PER_SOL),
            create_keyed_system_account(&vault, 0),
        ],
    );

    result.assert_success();

    let vault_acc = svm.get_account(&vault).unwrap();

    assert_eq!(vault_acc.data[1..33], payer.to_bytes());
    assert_eq!(vault_acc.data[33..53], eth_address);
}

#[test]
fn test_transfer_funds_by_authority() {
    let mut svm = setup();

    let authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let vault = derive_vault_address(authority);
    let amount = LAMPORTS_PER_SOL;
    let signing_key = SigningKey::random(&mut OsRng);
    let eth_address: [u8; 20] =
        EthAddress::from_public_key(signing_key.verifying_key()).into_array();

    let instruction = initialize_vault(authority, vault, eth_address);

    let result = svm.process_instruction(
        &instruction,
        &[
            create_keyed_system_account(&authority, LAMPORTS_PER_SOL),
            create_keyed_system_account(&vault, 0),
        ],
    );

    result.assert_success();

    // override vault lamports
    set_vault_lamports(&mut svm, vault, amount);

    let pre_vault_balance = svm.get_account(&vault).unwrap().lamports;
    let pre_recipient_balance = LAMPORTS_PER_SOL;

    let (hash, recovery_id, signature) = sign_payload(&signing_key, b"transfer_funds:test");
    let instruction = transfer_instruction(
        authority,
        recipient,
        vault,
        amount / 10,
        hash,
        recovery_id,
        signature,
    );

    let result = svm.process_instruction(
        &instruction,
        &[create_keyed_system_account(
            &recipient,
            pre_recipient_balance,
        )],
    );

    result.assert_success();

    let post_vault_balance = svm.get_account(&vault).unwrap().lamports;
    let post_recipient_balance = svm.get_account(&recipient).unwrap().lamports;

    assert_eq!(post_vault_balance, pre_vault_balance - amount / 10);
    assert_eq!(post_recipient_balance, pre_recipient_balance + amount / 10);
}

#[test]
fn test_transfer_funds_by_central_authority() {
    let mut svm = setup();

    let authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let vault = derive_vault_address(authority);
    let amount = LAMPORTS_PER_SOL;
    let signing_key = SigningKey::random(&mut OsRng);
    let eth_address: [u8; 20] =
        EthAddress::from_public_key(signing_key.verifying_key()).into_array();

    let instruction = initialize_vault(authority, vault, eth_address);

    let result = svm.process_instruction(
        &instruction,
        &[
            create_keyed_system_account(&authority, LAMPORTS_PER_SOL),
            create_keyed_system_account(&vault, 0),
        ],
    );

    result.assert_success();

    // override vault lamports
    set_vault_lamports(&mut svm, vault, amount);

    let pre_vault_balance = svm.get_account(&vault).unwrap().lamports;
    let pre_recipient_balance = LAMPORTS_PER_SOL;

    let (hash, recovery_id, signature) = sign_payload(&signing_key, b"transfer_funds:test");
    let instruction = transfer_instruction(
        CENTRAL_AUTHORITY,
        recipient,
        vault,
        amount / 10,
        hash,
        recovery_id,
        signature,
    );

    let result = svm.process_instruction(
        &instruction,
        &[
            create_keyed_system_account(&recipient, pre_recipient_balance),
            create_keyed_system_account(&CENTRAL_AUTHORITY, LAMPORTS_PER_SOL),
        ],
    );

    result.assert_success();

    let post_vault_balance = svm.get_account(&vault).unwrap().lamports;
    let post_recipient_balance = svm.get_account(&recipient).unwrap().lamports;

    assert_eq!(post_vault_balance, pre_vault_balance - amount / 10);
    assert_eq!(post_recipient_balance, pre_recipient_balance + amount / 10);
}

#[test]
fn test_transfer_funds_with_invalid_authority() {
    let mut svm = setup();

    let authority = Pubkey::new_unique();
    let invalid_authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let vault = derive_vault_address(authority);
    let amount = LAMPORTS_PER_SOL;
    let signing_key = SigningKey::random(&mut OsRng);
    let eth_address: [u8; 20] =
        EthAddress::from_public_key(signing_key.verifying_key()).into_array();

    let result = svm.process_instruction(
        &initialize_vault(authority, vault, eth_address),
        &[
            create_keyed_system_account(&authority, LAMPORTS_PER_SOL),
            create_keyed_system_account(&vault, 0),
        ],
    );
    result.assert_success();

    set_vault_lamports(&mut svm, vault, amount);
    let pre_recipient_balance = LAMPORTS_PER_SOL;

    let (hash, recovery_id, signature) = sign_payload(&signing_key, b"transfer_funds:test");
    let result = svm.process_instruction(
        &transfer_instruction(
            invalid_authority,
            recipient,
            vault,
            amount / 10,
            hash,
            recovery_id,
            signature,
        ),
        &[
            create_keyed_system_account(&recipient, pre_recipient_balance),
            create_keyed_system_account(&invalid_authority, LAMPORTS_PER_SOL),
        ],
    );

    assert!(result.is_err());
}

#[test]
fn test_transfer_funds_with_invalid_vault() {
    let mut svm = setup();

    let authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let real_vault = derive_vault_address(authority);
    let fake_vault = Pubkey::new_unique();
    let amount = LAMPORTS_PER_SOL;
    let signing_key = SigningKey::random(&mut OsRng);
    let eth_address: [u8; 20] =
        EthAddress::from_public_key(signing_key.verifying_key()).into_array();

    let result = svm.process_instruction(
        &initialize_vault(authority, real_vault, eth_address),
        &[
            create_keyed_system_account(&authority, LAMPORTS_PER_SOL),
            create_keyed_system_account(&real_vault, 0),
        ],
    );
    result.assert_success();

    let mut vault_acc = svm.get_account(&real_vault).unwrap();
    vault_acc.address = fake_vault;
    svm.set_account(vault_acc);

    let (hash, recovery_id, signature) = sign_payload(&signing_key, b"transfer_funds:test");
    let result = svm.process_instruction(
        &transfer_instruction(
            authority,
            recipient,
            fake_vault,
            amount / 10,
            hash,
            recovery_id,
            signature,
        ),
        &[create_keyed_system_account(&recipient, LAMPORTS_PER_SOL)],
    );

    assert!(result.is_err());
}

#[test]
fn test_transfer_funds_with_invalid_eth_address() {
    let mut svm = setup();

    let authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let vault = derive_vault_address(authority);
    let amount = LAMPORTS_PER_SOL;
    let valid_signing_key = SigningKey::random(&mut OsRng);
    let invalid_signing_key = SigningKey::random(&mut OsRng);
    let eth_address: [u8; 20] =
        EthAddress::from_public_key(valid_signing_key.verifying_key()).into_array();

    let result = svm.process_instruction(
        &initialize_vault(authority, vault, eth_address),
        &[
            create_keyed_system_account(&authority, LAMPORTS_PER_SOL),
            create_keyed_system_account(&vault, 0),
        ],
    );
    result.assert_success();

    set_vault_lamports(&mut svm, vault, amount);
    let pre_recipient_balance = LAMPORTS_PER_SOL;

    let (hash, recovery_id, signature) = sign_payload(&invalid_signing_key, b"transfer_funds:test");
    let result = svm.process_instruction(
        &transfer_instruction(
            authority,
            recipient,
            vault,
            amount / 10,
            hash,
            recovery_id,
            signature,
        ),
        &[create_keyed_system_account(
            &recipient,
            pre_recipient_balance,
        )],
    );

    assert!(result.is_err());
    assert!(svm.get_account(&recipient).is_none());
}

#[test]
fn test_transfer_funds_with_insufficient_vault_funds() {
    let mut svm = setup();

    let authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let vault = derive_vault_address(authority);
    let signing_key = SigningKey::random(&mut OsRng);
    let eth_address: [u8; 20] =
        EthAddress::from_public_key(signing_key.verifying_key()).into_array();

    let result = svm.process_instruction(
        &initialize_vault(authority, vault, eth_address),
        &[
            create_keyed_system_account(&authority, LAMPORTS_PER_SOL),
            create_keyed_system_account(&vault, 0),
        ],
    );
    result.assert_success();

    let pre_vault_balance = svm.get_account(&vault).unwrap().lamports;
    let pre_recipient_balance = LAMPORTS_PER_SOL;
    let transfer_amount = pre_vault_balance + 1;

    let (hash, recovery_id, signature) = sign_payload(&signing_key, b"transfer_funds:test");
    let result = svm.process_instruction(
        &transfer_instruction(
            authority,
            recipient,
            vault,
            transfer_amount,
            hash,
            recovery_id,
            signature,
        ),
        &[create_keyed_system_account(
            &recipient,
            pre_recipient_balance,
        )],
    );

    assert!(result.is_err());
}
