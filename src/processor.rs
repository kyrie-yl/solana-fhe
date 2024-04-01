//! Program instruction processor
//! Only the program admin can issue the Init instruction.
//! And anyone can check the loan with the Loan2Value instruction.

use solana_program::account_info::{
    next_account_info,
    AccountInfo,
};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_memory::sol_memcpy;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::clock::Clock;
use solana_program::sysvar::Sysvar;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use pyth_sdk_solana::Price;
use pyth_sdk_solana::state::SolanaPriceAccount;
use solana_program::program::invoke;

use crate::instruction::ExampleInstructions;
use crate::state::AdminConfig;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let signer = next_account_info(account_iter)?;
    let admin_config_account = next_account_info(account_iter)?;
    let pyth_sol_feed_account = next_account_info(account_iter)?;

    let instruction = ExampleInstructions::try_from_slice(input)?;
    match instruction {
        ExampleInstructions::Init {} => {
            // Only an authorized key should be able to configure the price feed id for each asset
            if !(signer.key == program_id && signer.is_signer) {
                return Err(ProgramError::Custom(0));
            }

            let mut config = AdminConfig::try_from_slice(&admin_config_account.try_borrow_data()?)?;

            if config.is_initialized {
                return Err(ProgramError::Custom(1));
            }

            config.is_initialized = true;
            config.sol_price_feed_id = *pyth_sol_feed_account.key;

            // Make sure these Pyth price accounts can be loaded
            SolanaPriceAccount::account_info_to_feed(pyth_sol_feed_account)?;

            let config_data = config.try_to_vec()?;
            let config_dst = &mut admin_config_account.try_borrow_mut_data()?;
            sol_memcpy(config_dst, &config_data, 1 + 32);
            Ok(())
        }

        ExampleInstructions::Usd2Sol {
            usd_qty,
        } => {
            let destination_account = next_account_info(account_iter)?;


            msg!("USD quantity is {}.", usd_qty);

            let config = AdminConfig::try_from_slice(&admin_config_account.try_borrow_data()?)?;

            if !config.is_initialized {
                return Err(ProgramError::Custom(1));
            }

            if config.sol_price_feed_id != *pyth_sol_feed_account.key
            {
                return Err(ProgramError::Custom(2));
            }

            // Here is more explanation on confidence interval in Pyth:
            // https://docs.pyth.network/consume-data/best-practices
            let feed = SolanaPriceAccount::account_info_to_feed(pyth_sol_feed_account)?;
            let current_timestamp = Clock::get()?.unix_timestamp;
            let result = feed
                .get_price_no_older_than(current_timestamp, 60)
                .ok_or(ProgramError::Custom(3))?;

            let fee_lamports = calculate_sol_amount(usd_qty, result);

            if signer.lamports() < fee_lamports {
                return Err(ProgramError::InsufficientFunds)
            }

            invoke(
                &solana_program::system_instruction::transfer(
                    signer.key,
                    destination_account.key,
                    fee_lamports,
                ),
                &[signer.clone(), destination_account.clone()],
            )?;
            return Ok(())
        }
    }
}

fn calculate_sol_amount(usd_qty: i64, price: Price) -> u64 {
     let fee_lamports = (10i64)
        .checked_pow((10 - price.expo) as u32).unwrap()
        .checked_mul(usd_qty).unwrap()
        .checked_div(price.price).unwrap();
    return  fee_lamports as u64
}