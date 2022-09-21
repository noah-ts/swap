use anchor_lang::prelude::*;

// State instance for each User
#[account]
pub struct UserState {
    pub user_enum: u8,
    pub counter_party: Pubkey,
    pub bump: u8,
}

// State instance for each Swap
#[account]
pub struct SwapState {
    pub offeror: Pubkey,
    pub offeree: Pubkey,
    pub mint_asset_a: Pubkey,
    pub mint_asset_b: Pubkey,
    pub escrow: Pubkey,
    pub swap_state_bump: u8,
    pub escrow_bump: u8,
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

#[error_code]
pub enum ErrorCode {
    #[msg("User is invalid, has to be offeror or offeree")]
    UserEnumInvalid
}