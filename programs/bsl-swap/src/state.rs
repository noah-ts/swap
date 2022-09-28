use anchor_lang::prelude::*;

// User State instance for each User
#[account]
pub struct UserState {
    pub user_enum: u8,
    pub counter_party: Pubkey,
    pub bump: u8,
}

// Swap State instance for each Swap
#[account]
pub struct SwapState {
    pub offeror: Pubkey,
    pub offeree: Pubkey,
    pub swap_state_bump: u8,
    pub mints_offeror: Vec<Pubkey>,
    pub mints_offeree: Vec<Pubkey>,
}

// Escrow State instance for each NFT to be sent from offeror to offeree
#[account]
pub struct EscrowState {
    pub escrow: Pubkey,
    pub mint: Pubkey,
    pub ata_offeror: Pubkey,
    pub state_bump: u8,
    pub ata_bump: u8,
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UserEnum {
    // user who is making an offer
    Offeror,
    // user to whom is offer being made
    Offeree,
    // swap completed
    None,
}

impl UserEnum {
    pub fn to_code(&self) -> u8 {
        match self {
            UserEnum::Offeror => 1,
            UserEnum::Offeree => 2,
            UserEnum::None => 3,
        }
    }

    pub fn from(val: u8) -> std::result::Result<UserEnum, anchor_lang::error::Error> {
        match val {
            1 => Ok(UserEnum::Offeror),
            2 => Ok(UserEnum::Offeree),
            3 => Ok(UserEnum::None),
            unknown_value => {
                msg!("Unknown stage: {}", unknown_value);
                return err!(ErrorCode::UserEnumInvalid)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CloseEscrowEnum {
    // user who is making an offer
    Cancel,
    // user to whom is offer being made
    Accept,
}

impl CloseEscrowEnum {
    pub fn to_code(&self) -> u8 {
        match self {
            CloseEscrowEnum::Cancel => 1,
            CloseEscrowEnum::Accept => 2,
        }
    }

    pub fn from(val: u8) -> std::result::Result<CloseEscrowEnum, anchor_lang::error::Error> {
        match val {
            1 => Ok(CloseEscrowEnum::Cancel),
            2 => Ok(CloseEscrowEnum::Accept),
            unknown_value => {
                msg!("Unknown stage: {}", unknown_value);
                return err!(ErrorCode::CloseEscrowEnumInvalid)
            }
        }
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("User is invalid, has to be offeror or offeree")]
    UserEnumInvalid,
    #[msg("Close escrow type is invalid, has to be cancel or accept")]
    CloseEscrowEnumInvalid
}