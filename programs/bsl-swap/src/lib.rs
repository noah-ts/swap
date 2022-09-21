pub mod state;

use anchor_lang::prelude::*;
use anchor_spl::{token::{Mint, Token, TokenAccount, Transfer}};
use state::{UserState, SwapState, UserEnum};

declare_id!("2kK95sc8qHyHQbyEHADvxC3uwB2kvLLajuqajh1cF27R");

#[program]
pub mod bsl_swap {
    use super::*;

    pub fn initialize_user_state(ctx: Context<InitializeUserState>, bump: u8) -> Result<()> {
        let state = &mut ctx.accounts.user_state;
        state.bump = bump;
        Ok(())
    }

    pub fn initialize_swap_state(ctx: Context<InitializeSwapState>, swap_state_bump: u8, escrow_bump: u8) -> Result<()> {
        let state = &mut ctx.accounts.swap_state;
        state.swap_state_bump = swap_state_bump;
        state.escrow_bump = escrow_bump;
        Ok(())
    }

    pub fn initiate_swap(ctx: Context<InitiateSwap>) -> Result<()> {
        let state = &mut ctx.accounts.swap_state;
        state.offeror = ctx.accounts.offeror.key().clone();
        state.offeree = ctx.accounts.offeree.key().clone();
        state.mint_asset_a = ctx.accounts.mint_asset_a.key().clone();
        state.mint_asset_b = ctx.accounts.mint_asset_b.key().clone();
        state.escrow = ctx.accounts.escrow.key().clone();

        let transfer_instruction = Transfer{
            from: ctx.accounts.ata_offeror_asset_a.to_account_info(),
            to: ctx.accounts.escrow.to_account_info(),
            authority: ctx.accounts.offeror.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        let offeror_state = &mut ctx.accounts.offeror_state;
        let offeree_state = &mut ctx.accounts.offeree_state;
        offeror_state.user_enum = UserEnum::Offeror.to_code();
        offeror_state.counter_party = ctx.accounts.offeree.key().clone();
        offeree_state.user_enum = UserEnum::Offeree.to_code();
        offeree_state.counter_party = ctx.accounts.offeror.key().clone();

        Ok(())
    }

    pub fn cancel_swap(ctx: Context<CancelSwap>) -> Result<()> {
        let bump_vector = &[ctx.accounts.swap_state.swap_state_bump][..];
        let inner = vec![
            b"swap_state".as_ref(),
            ctx.accounts.offeror.key.as_ref(),
            ctx.accounts.offeree.key.as_ref(),
            bump_vector.as_ref(),
        ];
        let outer = vec![inner.as_slice()];

        let transfer_instruction = Transfer{
            from: ctx.accounts.escrow.to_account_info(),
            to: ctx.accounts.ata_offeror_asset_a.to_account_info(),
            authority: ctx.accounts.swap_state.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            outer.as_slice(),
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        let offeror_state = &mut ctx.accounts.offeror_state;
        let offeree_state = &mut ctx.accounts.offeree_state;
        offeror_state.user_enum = UserEnum::None.to_code();
        offeree_state.user_enum = UserEnum::None.to_code();

        Ok(())
    }

    // accept_swap_one and accept_swap_two should run in a single transaction
    // had to split one instruction into two due to Anchor limiting number of accounts in context
    // send from escrow to offeree
    pub fn accept_swap_one(ctx: Context<AcceptSwapOne>) -> Result<()> {
        let bump_vector = &[ctx.accounts.swap_state.swap_state_bump][..];
        let inner = vec![
            b"swap_state".as_ref(),
            ctx.accounts.offeror.key.as_ref(),
            ctx.accounts.offeree.key.as_ref(),
            bump_vector.as_ref(),
        ];
        let outer = vec![inner.as_slice()];

        let transfer_instruction = Transfer{
            from: ctx.accounts.escrow.to_account_info(),
            to: ctx.accounts.ata_offeree_asset_a.to_account_info(),
            authority: ctx.accounts.swap_state.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            outer.as_slice(),
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        let offeror_state = &mut ctx.accounts.offeror_state;
        let offeree_state = &mut ctx.accounts.offeree_state;
        offeror_state.user_enum = UserEnum::None.to_code();
        offeree_state.user_enum = UserEnum::None.to_code();

        Ok(())
    }

    // accept_swap_one and accept_swap_two should run in a single transaction
    // had to split one instruction into two due to Anchor limiting number of accounts in context
    // send from offeree to offeror
    pub fn accept_swap_two(ctx: Context<AcceptSwapTwo>) -> Result<()> {
        let transfer_instruction = Transfer{
            from: ctx.accounts.ata_offeree_asset_b.to_account_info(),
            to: ctx.accounts.ata_offeror_asset_b.to_account_info(),
            authority: ctx.accounts.offeree.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeUserState<'info> {
    #[account(
        init,
        space = 500,
        payer = user,
        seeds=[b"user_state".as_ref(), user.key().as_ref()],
        bump,
    )]
    user_state: Account<'info, UserState>,
    #[account(mut)]
    user: Signer<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeSwapState<'info> {
    // PDAs
    #[account(
        init,
        space = 1000,
        payer = offeror,
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        init,
        payer = offeror,
        seeds=[b"escrow".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump,
        token::mint=mint_asset_a,
        token::authority=swap_state,
    )]
    escrow: Account<'info, TokenAccount>,

    mint_asset_a: Account<'info, Mint>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitiateSwap<'info> {
    // PDAs
    #[account(
        mut,
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        mut,
        seeds=[b"escrow".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.escrow_bump,
    )]
    escrow: Account<'info, TokenAccount>,

    mint_asset_a: Account<'info, Mint>,
    mint_asset_b: Account<'info, Mint>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,

    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeror.key().as_ref()],
        bump = offeror_state.bump,
    )]
    offeror_state: Account<'info, UserState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeree.key().as_ref()],
        bump = offeree_state.bump,
    )]
    offeree_state: Account<'info, UserState>,

    #[account(
        mut,
        constraint=ata_offeror_asset_a.owner == offeror.key(),
        constraint=ata_offeror_asset_a.mint == mint_asset_a.key()
    )]
    ata_offeror_asset_a: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelSwap<'info> {
    // PDAs
    #[account(
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
        has_one = offeror,
        has_one = offeree,
        has_one = mint_asset_a,
        has_one = mint_asset_b,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        mut,
        seeds=[b"escrow".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.escrow_bump,
    )]
    escrow: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeror.key().as_ref()],
        bump = offeror_state.bump,
    )]
    offeror_state: Account<'info, UserState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeree.key().as_ref()],
        bump = offeree_state.bump,
    )]
    offeree_state: Account<'info, UserState>,

    mint_asset_a: Account<'info, Mint>,
    mint_asset_b: Account<'info, Mint>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,

    #[account(
        mut,
        constraint=ata_offeror_asset_a.owner == offeror.key(),
        constraint=ata_offeror_asset_a.mint == mint_asset_a.key()
    )]
    ata_offeror_asset_a: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AcceptSwapOne<'info> {
    // PDAs
    #[account(
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
        has_one = offeror,
        has_one = offeree,
        has_one = mint_asset_a,
        has_one = mint_asset_b,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        mut,
        seeds=[b"escrow".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.escrow_bump,
    )]
    escrow: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeror.key().as_ref()],
        bump = offeror_state.bump,
    )]
    offeror_state: Account<'info, UserState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeree.key().as_ref()],
        bump = offeree_state.bump,
    )]
    offeree_state: Account<'info, UserState>,

    mint_asset_a: Account<'info, Mint>,
    mint_asset_b: Account<'info, Mint>,

    /// CHECK: not reading or writing to this account
    offeror: AccountInfo<'info>,
    /// CHECK: not reading or writing to this account
    offeree: AccountInfo<'info>,

    #[account(
        mut,
        constraint=ata_offeree_asset_a.owner == offeree.key(),
        constraint=ata_offeree_asset_a.mint == mint_asset_a.key()
    )]
    ata_offeree_asset_a: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AcceptSwapTwo<'info> {
    // PDAs
    #[account(
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
        has_one = offeror,
        has_one = offeree,
        has_one = mint_asset_a,
        has_one = mint_asset_b,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        seeds=[b"escrow".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.escrow_bump,
    )]
    escrow: Account<'info, TokenAccount>,

    mint_asset_a: Account<'info, Mint>,
    mint_asset_b: Account<'info, Mint>,

    /// CHECK: only adding tokens to this account
    #[account(mut)]
    offeror: AccountInfo<'info>,
    offeree: Signer<'info>,

    #[account(
        mut,
        constraint=ata_offeror_asset_b.owner == offeror.key(),
        constraint=ata_offeror_asset_b.mint == mint_asset_b.key()
    )]
    ata_offeror_asset_b: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint=ata_offeree_asset_b.owner == offeree.key(),
        constraint=ata_offeree_asset_b.mint == mint_asset_b.key()
    )]
    ata_offeree_asset_b: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
}