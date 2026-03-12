use anchor_lang::prelude::*;

use crate::constants::{BOUNDARY_TOLERANCE, SECONDS_PER_DAY, TRANSFER_WINDOW_END, TRANSFER_WINDOW_START};

#[account]
#[derive(InitSpace)]
pub struct Oracle {
    pub validation: OracleValidation,
    pub collection: Pubkey,
    pub last_updated: i64,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Oracle {
    pub fn is_allowed_transfer(unix_timestamp: i64) -> bool {
        let seconds_of_day = (unix_timestamp % SECONDS_PER_DAY) as u32;
        
        (TRANSFER_WINDOW_START..TRANSFER_WINDOW_END).contains(&seconds_of_day)
    }

    pub fn is_boundary_time(unix_timestamp: i64) -> bool {
        let seconds_since_midnight = unix_timestamp % SECONDS_PER_DAY;
        (seconds_since_midnight >= TRANSFER_WINDOW_START as i64 
            && seconds_since_midnight < TRANSFER_WINDOW_START as i64 + BOUNDARY_TOLERANCE)
        || (seconds_since_midnight >= TRANSFER_WINDOW_END as i64 
            && seconds_since_midnight < TRANSFER_WINDOW_END as i64 + BOUNDARY_TOLERANCE)
    }

    pub fn transfer_validation(unix_timestamp: i64) -> ExternalValidationResult {
        if Self::is_allowed_transfer(unix_timestamp) {
            ExternalValidationResult::Approved
        } else {
            ExternalValidationResult::Rejected
        }
    }

    pub fn make_validation(transfer_validation: ExternalValidationResult) -> OracleValidation {
        OracleValidation::V1 {
            transfer: transfer_validation,
            create: ExternalValidationResult::Pass,
            burn: ExternalValidationResult::Pass,
            update: ExternalValidationResult::Pass,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Eq, PartialEq)]
pub enum OracleValidation {
    Uninitialized,
    V1 {
        create: ExternalValidationResult,
        transfer: ExternalValidationResult,
        burn: ExternalValidationResult,
        update: ExternalValidationResult,
    },
}

impl Space for OracleValidation {
    const INIT_SPACE: usize = 1 + 4 * ExternalValidationResult::INIT_SPACE;
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Debug, Eq, PartialEq)]
pub enum ExternalValidationResult {
    Approved,
    Rejected,
    Pass,
}

impl Space for ExternalValidationResult {
    const INIT_SPACE: usize = 1;
}

impl ExternalValidationResult {
    pub fn toggle(self) -> Self {
        match self {
            Self::Approved => Self::Rejected,
            Self::Rejected => Self::Approved,
            Self::Pass => Self::Pass,
        }
    }
}
